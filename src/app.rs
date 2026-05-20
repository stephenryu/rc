use std::{fs, io, path::{Path, PathBuf}};

use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{
    fs_ops::{auto_rename, copy_dir_all},
    panel::Panel,
};

// ── Mode ─────────────────────────────────────────────────────────────────────

pub enum Mode {
    Normal,
    About,
    Input { prompt: String, value: String, action: InputAction },
    Viewer { path: PathBuf, content: Vec<String>, scroll: usize },
    Confirm { prompt: String, action: ConfirmAction },
    Conflict {
        src: PathBuf,
        dst: PathBuf,
        queue: Vec<(PathBuf, PathBuf)>,
        is_move: bool,
        done: usize,
        errors: Vec<String>,
    },
}

#[derive(Clone)]
pub enum InputAction { Mkdir, Rename(PathBuf) }

#[derive(Clone)]
pub enum ConfirmAction { Delete(Vec<PathBuf>) }

// ── App ──────────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
pub enum ActivePanel { Left, Right }

pub struct App {
    pub left: Panel,
    pub right: Panel,
    pub active: ActivePanel,
    pub message: String,
    pub mode: Mode,
}

impl App {
    pub fn new(left_path: Option<PathBuf>, right_path: Option<PathBuf>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let left_dir = left_path.unwrap_or_else(|| cwd.clone());
        let right_dir = right_path.unwrap_or_else(|| left_dir.clone());
        App {
            left: Panel::new(left_dir),
            right: Panel::new(right_dir),
            active: ActivePanel::Left,
            message: String::new(),
            mode: Mode::Normal,
        }
    }

    pub fn active_panel(&mut self) -> &mut Panel {
        match self.active {
            ActivePanel::Left  => &mut self.left,
            ActivePanel::Right => &mut self.right,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match &self.mode {
            Mode::Normal       => self.handle_normal(code, modifiers),
            Mode::About        => { self.mode = Mode::Normal; true }
            Mode::Input { .. } => { self.handle_input(code); true }
            Mode::Viewer { .. }  => { self.handle_viewer(code); true }
            Mode::Confirm { .. } => { self.handle_confirm(code); true }
            Mode::Conflict { .. } => { self.handle_conflict(code); true }
        }
    }

    // ── key handlers ─────────────────────────────────────────────────────────

    fn handle_normal(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.message.clear();
        match code {
            KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => return false,
            KeyCode::Tab => {
                self.active = match self.active {
                    ActivePanel::Left  => ActivePanel::Right,
                    ActivePanel::Right => ActivePanel::Left,
                };
            }
            KeyCode::Up       => self.active_panel().move_cursor(-1),
            KeyCode::Down     => self.active_panel().move_cursor(1),
            KeyCode::PageUp   => { let p = self.active_panel(); p.page_move(20, false); }
            KeyCode::PageDown => { let p = self.active_panel(); p.page_move(20, true); }
            KeyCode::Home     => { let p = self.active_panel(); p.state.select(Some(0)); }
            KeyCode::End      => {
                let p = self.active_panel();
                let last = p.entries.len().saturating_sub(1);
                p.state.select(Some(last));
            }
            KeyCode::Char(' ') | KeyCode::Insert => self.active_panel().toggle_select(),
            KeyCode::Enter => self.active_panel().enter(),
            KeyCode::F(1) => { self.mode = Mode::About; }
            KeyCode::F(2) => self.prompt_rename(),
            KeyCode::F(3) => self.open_viewer(),
            KeyCode::F(4) => self.open_editor(),
            KeyCode::F(5) => self.copy_files(),
            KeyCode::F(6) => self.move_files(),
            KeyCode::F(7) => {
                self.mode = Mode::Input {
                    prompt: "New directory name:".into(),
                    value: String::new(),
                    action: InputAction::Mkdir,
                };
            }
            KeyCode::F(8) => self.prompt_delete(),
            _ => {}
        }
        true
    }

    fn handle_input(&mut self, code: KeyCode) {
        let Mode::Input { ref mut value, ref action, .. } = self.mode else { return };
        match code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Enter => {
                let val = value.trim().to_string();
                let act = action.clone();
                self.mode = Mode::Normal;
                if !val.is_empty() {
                    match act {
                        InputAction::Mkdir => self.do_mkdir(val),
                        InputAction::Rename(src) => self.do_rename(src, val),
                    }
                }
            }
            KeyCode::Backspace => { value.pop(); }
            KeyCode::Char(c)   => { value.push(c); }
            _ => {}
        }
    }

    fn handle_viewer(&mut self, code: KeyCode) {
        let Mode::Viewer { ref content, ref mut scroll, .. } = self.mode else { return };
        let max = content.len().saturating_sub(1);
        match code {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::Char('q') => { self.mode = Mode::Normal; }
            KeyCode::Up       => { *scroll = scroll.saturating_sub(1); }
            KeyCode::Down     => { *scroll = (*scroll + 1).min(max); }
            KeyCode::PageUp   => { *scroll = scroll.saturating_sub(20); }
            KeyCode::PageDown => { *scroll = (*scroll + 20).min(max); }
            KeyCode::Home     => { *scroll = 0; }
            KeyCode::End      => { *scroll = max; }
            _ => {}
        }
    }

    fn handle_confirm(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let Mode::Confirm { action, .. } = std::mem::replace(&mut self.mode, Mode::Normal)
                else { return };
                match action {
                    ConfirmAction::Delete(targets) => self.do_delete(targets),
                }
            }
            _ => { self.mode = Mode::Normal; }
        }
    }

    fn handle_conflict(&mut self, code: KeyCode) {
        let Mode::Conflict { src, dst, queue, is_move, done, errors } =
            std::mem::replace(&mut self.mode, Mode::Normal)
        else { return };

        match code {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                let mut errs = errors;
                self.do_single_copy(&src, &dst, is_move, &mut errs);
                self.process_copy_queue(queue, is_move, done + 1, errs);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                let mut errs = errors;
                self.do_single_copy(&src, &dst, is_move, &mut errs);
                let mut remaining = queue;
                for (s, d) in remaining.drain(..) {
                    self.do_single_copy(&s, &d, is_move, &mut errs);
                }
                self.finish_copy(done + 1 + remaining.len(), errs);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.process_copy_queue(queue, is_move, done, errors);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.finish_copy(done, errors);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let new_dst = auto_rename(&dst);
                let mut errs = errors;
                self.do_single_copy(&src, &new_dst, is_move, &mut errs);
                self.process_copy_queue(queue, is_move, done + 1, errs);
            }
            _ => { self.finish_copy(done, errors); }
        }
    }

    // ── copy / move queue ─────────────────────────────────────────────────────

    fn do_single_copy(&self, src: &Path, dst: &Path, is_move: bool, errors: &mut Vec<String>) {
        let result = if is_move {
            fs::rename(src, dst).map(|_| 0u64)
        } else if src.is_dir() {
            copy_dir_all(src, dst).map(|_| 0u64)
        } else {
            fs::copy(src, dst)
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e}", src.file_name().unwrap_or_default().to_string_lossy()));
        }
    }

    fn process_copy_queue(
        &mut self,
        mut queue: Vec<(PathBuf, PathBuf)>,
        is_move: bool,
        done: usize,
        mut errors: Vec<String>,
    ) {
        while let Some((src, dst)) = queue.first().cloned() {
            queue.remove(0);
            if dst.exists() {
                self.mode = Mode::Conflict { src, dst, queue, is_move, done, errors };
                return;
            }
            self.do_single_copy(&src, &dst, is_move, &mut errors);
        }
        self.finish_copy(done, errors);
    }

    fn finish_copy(&mut self, done: usize, errors: Vec<String>) {
        if errors.is_empty() {
            self.message = format!("{} {} item(s)",
                if done > 0 { "Processed" } else { "Nothing to do:" }, done);
        } else {
            self.message = format!("Done ({done} ok) — Errors: {}", errors.join("; "));
        }
        self.left.refresh();
        self.right.refresh();
    }

    // ── file operations ───────────────────────────────────────────────────────

    fn open_viewer(&mut self) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let Some(entry) = panel.selected_entry() else { return };
        if entry.is_dir || entry.name == ".." { return; }
        let path = panel.cwd.join(&entry.name);
        let content = match fs::read_to_string(&path) {
            Ok(s)  => s.lines().map(String::from).collect(),
            Err(e) => vec![format!("Cannot read file: {e}")],
        };
        self.mode = Mode::Viewer { path, content, scroll: 0 };
    }

    fn open_editor(&mut self) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let Some(entry) = panel.selected_entry() else { return };
        if entry.name == ".." { return; }
        let path = panel.cwd.join(&entry.name);
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());

        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = std::process::Command::new(&editor).arg(&path).status();
        let _ = execute!(io::stdout(), EnterAlternateScreen);
        let _ = enable_raw_mode();

        self.left.refresh();
        self.right.refresh();
    }

    fn copy_files(&mut self) {
        let (targets, dst_dir) = self.targets_and_dst();
        if targets.is_empty() { return; }
        let queue = targets
            .into_iter()
            .map(|s| { let d = dst_dir.join(s.file_name().unwrap_or_default()); (s, d) })
            .collect();
        self.process_copy_queue(queue, false, 0, vec![]);
    }

    fn move_files(&mut self) {
        let (targets, dst_dir) = self.targets_and_dst();
        if targets.is_empty() { return; }
        let queue = targets
            .into_iter()
            .map(|s| { let d = dst_dir.join(s.file_name().unwrap_or_default()); (s, d) })
            .collect();
        self.process_copy_queue(queue, true, 0, vec![]);
    }

    fn prompt_delete(&mut self) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let targets = panel.effective_targets();
        if targets.is_empty() { return; }
        let names: Vec<_> = targets.iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect();
        let prompt = format!("Delete {}? [y/N]", names.join(", "));
        self.mode = Mode::Confirm { prompt, action: ConfirmAction::Delete(targets) };
    }

    fn do_delete(&mut self, targets: Vec<PathBuf>) {
        let mut errors = vec![];
        for path in &targets {
            let result = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
            if let Err(e) = result { errors.push(e.to_string()); }
        }
        if errors.is_empty() {
            self.message = format!("Deleted {} item(s)", targets.len());
        } else {
            self.message = format!("Errors: {}", errors.join("; "));
        }
        self.left.refresh();
        self.right.refresh();
    }

    fn prompt_rename(&mut self) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let Some(entry) = panel.selected_entry() else { return };
        if entry.name == ".." { return; }
        let src = panel.cwd.join(&entry.name);
        self.mode = Mode::Input {
            prompt: "Rename:".into(),
            value: entry.name.clone(),
            action: InputAction::Rename(src),
        };
    }

    fn do_rename(&mut self, src: PathBuf, new_name: String) {
        let dst = src.parent().unwrap_or(src.as_path()).join(&new_name);
        match fs::rename(&src, &dst) {
            Ok(_)  => self.message = format!("Renamed to {new_name}"),
            Err(e) => self.message = format!("Rename error: {e}"),
        }
        self.left.refresh();
        self.right.refresh();
    }

    fn do_mkdir(&mut self, name: String) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let path = panel.cwd.join(&name);
        match fs::create_dir_all(&path) {
            Ok(_)  => self.message = format!("Created {name}"),
            Err(e) => self.message = format!("mkdir error: {e}"),
        }
        self.left.refresh();
        self.right.refresh();
    }

    fn targets_and_dst(&self) -> (Vec<PathBuf>, PathBuf) {
        let (src_panel, dst_panel) = match self.active {
            ActivePanel::Left  => (&self.left,  &self.right),
            ActivePanel::Right => (&self.right, &self.left),
        };
        (src_panel.effective_targets(), dst_panel.cwd.clone())
    }
}
