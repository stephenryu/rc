use std::{
    fs,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;

/// rc — Rust Commander
///
/// A Norton Commander-style dual-panel terminal file manager.
#[derive(Parser)]
#[command(name = "rc", version = env!("RC_VERSION"), about, long_about = None)]
struct Cli {
    /// Starting directory for the left panel
    left: Option<PathBuf>,
    /// Starting directory for the right panel (defaults to left panel path)
    right: Option<PathBuf>,
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

// ── Entry ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
}

impl Entry {
    fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        let meta = fs::metadata(path).ok()?;
        Some(Entry {
            name,
            is_dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
        })
    }
}

// ── Panel ────────────────────────────────────────────────────────────────────

struct Panel {
    cwd: PathBuf,
    entries: Vec<Entry>,
    state: ListState,
    selected: std::collections::HashSet<usize>,
}

impl Panel {
    fn new(path: PathBuf) -> Self {
        let mut p = Panel {
            cwd: path,
            entries: vec![],
            state: ListState::default(),
            selected: std::collections::HashSet::new(),
        };
        p.refresh();
        p
    }

    fn refresh(&mut self) {
        let mut entries: Vec<Entry> = fs::read_dir(&self.cwd)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| Entry::from_path(&e.path()))
            .collect();

        entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        if self.cwd.parent().is_some() {
            entries.insert(0, Entry { name: "..".into(), is_dir: true, size: 0 });
        }

        self.entries = entries;
        self.selected.clear();
        if self.state.selected().is_none() && !self.entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn selected_entry(&self) -> Option<&Entry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    fn move_cursor(&mut self, delta: i64) {
        let len = self.entries.len() as i64;
        if len == 0 { return; }
        let cur = self.state.selected().unwrap_or(0) as i64;
        let next = (cur + delta).rem_euclid(len) as usize;
        self.state.select(Some(next));
    }

    fn page_move(&mut self, page_size: usize, down: bool) {
        let len = self.entries.len();
        if len == 0 { return; }
        let cur = self.state.selected().unwrap_or(0);
        let next = if down {
            (cur + page_size).min(len - 1)
        } else {
            cur.saturating_sub(page_size)
        };
        self.state.select(Some(next));
    }

    fn toggle_select(&mut self) {
        if let Some(i) = self.state.selected() {
            if self.entries.get(i).map(|e| e.name == "..").unwrap_or(false) {
                return;
            }
            if self.selected.contains(&i) {
                self.selected.remove(&i);
            } else {
                self.selected.insert(i);
            }
            self.move_cursor(1);
        }
    }

    fn enter(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            if entry.is_dir {
                let next = if entry.name == ".." {
                    self.cwd.parent().unwrap().to_path_buf()
                } else {
                    self.cwd.join(&entry.name)
                };
                self.cwd = next;
                self.state.select(Some(0));
                self.refresh();
            }
        }
    }

    fn effective_targets(&self) -> Vec<PathBuf> {
        if self.selected.is_empty() {
            self.selected_entry()
                .filter(|e| e.name != "..")
                .map(|e| vec![self.cwd.join(&e.name)])
                .unwrap_or_default()
        } else {
            let mut v: Vec<usize> = self.selected.iter().copied().collect();
            v.sort();
            v.iter()
                .filter_map(|&i| self.entries.get(i))
                .map(|e| self.cwd.join(&e.name))
                .collect()
        }
    }
}

// ── Mode ─────────────────────────────────────────────────────────────────────

enum Mode {
    Normal,
    About,
    Input { prompt: String, value: String, action: InputAction },
    Viewer { path: PathBuf, content: Vec<String>, scroll: usize },
    Confirm { prompt: String, action: ConfirmAction },
}

#[derive(Clone)]
enum InputAction { Mkdir }

#[derive(Clone)]
enum ConfirmAction { Delete(Vec<PathBuf>) }

// ── App ──────────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum ActivePanel { Left, Right }

struct App {
    left: Panel,
    right: Panel,
    active: ActivePanel,
    message: String,
    mode: Mode,
}

impl App {
    fn new(left_path: Option<PathBuf>, right_path: Option<PathBuf>) -> Self {
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

    fn active_panel(&mut self) -> &mut Panel {
        match self.active {
            ActivePanel::Left => &mut self.left,
            ActivePanel::Right => &mut self.right,
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match &self.mode {
            Mode::Normal => self.handle_normal(code, modifiers),
            Mode::About => { self.mode = Mode::Normal; true }
            Mode::Input { .. } => { self.handle_input(code); true }
            Mode::Viewer { .. } => { self.handle_viewer(code); true }
            Mode::Confirm { .. } => { self.handle_confirm(code); true }
        }
    }

    fn handle_normal(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.message.clear();
        match code {
            KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => return false,
            KeyCode::Tab => {
                self.active = match self.active {
                    ActivePanel::Left => ActivePanel::Right,
                    ActivePanel::Right => ActivePanel::Left,
                };
            }
            KeyCode::Up   => self.active_panel().move_cursor(-1),
            KeyCode::Down => self.active_panel().move_cursor(1),
            KeyCode::PageUp   => { let p = self.active_panel(); p.page_move(20, false); }
            KeyCode::PageDown => { let p = self.active_panel(); p.page_move(20, true); }
            KeyCode::Home => { let p = self.active_panel(); let _ = p.state.select(Some(0)); }
            KeyCode::End  => {
                let p = self.active_panel();
                let last = p.entries.len().saturating_sub(1);
                p.state.select(Some(last));
            }
            KeyCode::Char(' ') | KeyCode::Insert => self.active_panel().toggle_select(),
            KeyCode::Enter => self.active_panel().enter(),
            KeyCode::F(1) => { self.mode = Mode::About; }
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
                    }
                }
            }
            KeyCode::Backspace => { value.pop(); }
            KeyCode::Char(c) => { value.push(c); }
            _ => {}
        }
    }

    fn handle_viewer(&mut self, code: KeyCode) {
        let Mode::Viewer { ref content, ref mut scroll, .. } = self.mode else { return };
        let max = content.len().saturating_sub(1);
        match code {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::Char('q') => { self.mode = Mode::Normal; }
            KeyCode::Up   => { *scroll = scroll.saturating_sub(1); }
            KeyCode::Down => { *scroll = (*scroll + 1).min(max); }
            KeyCode::PageUp   => { *scroll = scroll.saturating_sub(20); }
            KeyCode::PageDown => { *scroll = (*scroll + 20).min(max); }
            KeyCode::Home => { *scroll = 0; }
            KeyCode::End  => { *scroll = max; }
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

    // ── file operations ──────────────────────────────────────────────────────

    fn open_viewer(&mut self) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let Some(entry) = panel.selected_entry() else { return };
        if entry.is_dir || entry.name == ".." { return; }
        let path = panel.cwd.join(&entry.name);
        let content = match fs::read_to_string(&path) {
            Ok(s) => s.lines().map(String::from).collect(),
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

        // temporarily leave TUI
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
        let mut errors = vec![];
        for src in &targets {
            let dst = dst_dir.join(src.file_name().unwrap_or_default());
            if src.is_dir() {
                if let Err(e) = copy_dir_all(src, &dst) { errors.push(e.to_string()); }
            } else if let Err(e) = fs::copy(src, &dst) {
                errors.push(e.to_string());
            }
        }
        if errors.is_empty() {
            self.message = format!("Copied {} item(s) → {}", targets.len(), dst_dir.display());
        } else {
            self.message = format!("Errors: {}", errors.join("; "));
        }
        self.left.refresh();
        self.right.refresh();
    }

    fn move_files(&mut self) {
        let (targets, dst_dir) = self.targets_and_dst();
        if targets.is_empty() { return; }
        let mut errors = vec![];
        for src in &targets {
            let dst = dst_dir.join(src.file_name().unwrap_or_default());
            if let Err(e) = fs::rename(src, &dst) { errors.push(e.to_string()); }
        }
        if errors.is_empty() {
            self.message = format!("Moved {} item(s) → {}", targets.len(), dst_dir.display());
        } else {
            self.message = format!("Errors: {}", errors.join("; "));
        }
        self.left.refresh();
        self.right.refresh();
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

    fn do_mkdir(&mut self, name: String) {
        let panel = match self.active { ActivePanel::Left => &self.left, ActivePanel::Right => &self.right };
        let path = panel.cwd.join(&name);
        match fs::create_dir_all(&path) {
            Ok(_) => self.message = format!("Created {name}"),
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

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

// ── Rendering ────────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let left_active = app.active == ActivePanel::Left;
    render_panel(f, &mut app.left, cols[0], left_active);
    render_panel(f, &mut app.right, cols[1], !left_active);

    let msg = if app.message.is_empty() {
        let (src, dst) = match app.active {
            ActivePanel::Left  => (&app.left,  &app.right),
            ActivePanel::Right => (&app.right, &app.left),
        };
        format!(" {} → {}", src.cwd.display(), dst.cwd.display())
    } else {
        format!(" {}", app.message)
    };
    let status = Paragraph::new(msg).style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(status, rows[1]);

    let fkeys = Line::from(vec![
        key_span("F1", "About"), key_span("F3", "View"), key_span("F4", "Edit"),
        key_span("F5", "Copy"), key_span("F6", "Move"), key_span("F7", "MkDir"),
        key_span("F8", "Del"), key_span("^Q", "Quit"),
    ]);
    f.render_widget(Paragraph::new(fkeys).style(Style::default().bg(Color::Black)), rows[2]);

    // overlays
    match &app.mode {
        Mode::About => render_about(f, area),
        Mode::Input { prompt, value, .. } => render_input(f, area, prompt, value),
        Mode::Viewer { path, content, scroll } => render_viewer(f, area, path, content, *scroll),
        Mode::Confirm { prompt, .. } => render_confirm(f, area, prompt),
        Mode::Normal => {}
    }
}

fn render_about(f: &mut Frame, area: Rect) {
    let popup = centered_rect(50, 40, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" About rc ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));

    let version = env!("RC_VERSION");
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  rc — Rust Commander", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("  v{version}"), Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from("  A Norton Commander-style file manager"),
        Line::from("  written in Rust."),
        Line::from(""),
        Line::from(Span::styled("  Key bindings", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab        Switch panel"),
        Line::from("  Enter      Open dir"),
        Line::from("  Space/Ins  Tag file"),
        Line::from("  F3         View file"),
        Line::from("  F4         Edit ($EDITOR)"),
        Line::from("  F5         Copy"),
        Line::from("  F6         Move"),
        Line::from("  F7         Make dir"),
        Line::from("  F8         Delete"),
        Line::from("  ^Q         Quit"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    f.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_input(f: &mut Frame, area: Rect, prompt: &str, value: &str) {
    let popup = centered_rect(50, 5, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(" Input ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let text = format!("{}\n> {}_", prompt, value);
    f.render_widget(Paragraph::new(text).block(block), popup);
}

fn render_confirm(f: &mut Frame, area: Rect, prompt: &str) {
    let popup = centered_rect(60, 5, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(" Confirm ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
    f.render_widget(Paragraph::new(prompt.to_string()).block(block), popup);
}

fn render_viewer(f: &mut Frame, area: Rect, path: &Path, content: &[String], scroll: usize) {
    let popup = centered_rect(90, 90, area);
    f.render_widget(Clear, popup);
    let title = format!(" {} ", path.file_name().unwrap_or_default().to_string_lossy());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(title, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let visible: Vec<ListItem> = content
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|line| ListItem::new(Line::from(Span::raw(line.as_str()))))
        .collect();

    let total = content.len();
    let pct = if total == 0 { 0 } else { ((scroll + inner.height as usize) * 100 / total).min(100) };
    let info = format!(" {}/{} ({}%) — ESC to close ", scroll + 1, total, pct);
    let info_area = Rect { y: popup.bottom() - 1, x: popup.x + 1, width: popup.width - 2, height: 1 };
    f.render_widget(
        Paragraph::new(info).style(Style::default().bg(Color::Green).fg(Color::Black)),
        info_area,
    );

    f.render_widget(List::new(visible), inner);
}

fn centered_rect(percent_x: u16, percent_h: u16, area: Rect) -> Rect {
    let popup_w = area.width * percent_x / 100;
    let popup_h = (area.height * percent_h / 100).max(5);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    Rect { x: area.x + x, y: area.y + y, width: popup_w, height: popup_h }
}

fn key_span(key: &str, label: &str) -> Span<'static> {
    Span::styled(
        format!("[{key}]{label} "),
        Style::default().fg(Color::Cyan),
    )
}

fn render_panel(f: &mut Frame, panel: &mut Panel, area: Rect, active: bool) {
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let sel_count = panel.selected.len();
    let title = if sel_count > 0 {
        format!("{} [{sel_count} selected]", panel.cwd.display())
    } else {
        panel.cwd.to_string_lossy().into_owned()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD)));

    let items: Vec<ListItem> = panel
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let is_tagged = panel.selected.contains(&i);
            let (icon, mut style) = if e.is_dir {
                ("/", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                (" ", Style::default().fg(Color::White))
            };
            if is_tagged {
                style = style.fg(Color::Magenta).add_modifier(Modifier::BOLD);
            }
            let size_str = if e.is_dir {
                "<DIR>     ".into()
            } else {
                format_size(e.size)
            };
            let tag = if is_tagged { "★ " } else { "  " };
            let line = Line::from(vec![
                Span::styled(tag, Style::default().fg(Color::Magenta)),
                Span::styled(format!("{icon} "), style),
                Span::styled(e.name.clone(), style),
                Span::styled(format!("  {size_str}"), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶");

    f.render_stateful_widget(list, area, &mut panel.state);
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{:>8}B ", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:>7.1}K ", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:>7.1}M ", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:>7.1}G ", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli.left, cli.right);
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if !app.handle_key(key.code, key.modifiers) {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
