use std::{collections::HashSet, fs, path::{Path, PathBuf}};
use ratatui::widgets::ListState;

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

impl Entry {
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        let meta = fs::metadata(path).ok()?;
        Some(Entry {
            name,
            is_dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
        })
    }
}

pub struct Panel {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub state: ListState,
    pub selected: HashSet<usize>,
}

impl Panel {
    pub fn new(path: PathBuf) -> Self {
        let mut p = Panel {
            cwd: path,
            entries: vec![],
            state: ListState::default(),
            selected: HashSet::new(),
        };
        p.refresh();
        p
    }

    pub fn refresh(&mut self) {
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

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn move_cursor(&mut self, delta: i64) {
        let len = self.entries.len() as i64;
        if len == 0 { return; }
        let cur = self.state.selected().unwrap_or(0) as i64;
        let next = (cur + delta).rem_euclid(len) as usize;
        self.state.select(Some(next));
    }

    pub fn page_move(&mut self, page_size: usize, down: bool) {
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

    pub fn toggle_select(&mut self) {
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

    pub fn enter(&mut self) {
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

    pub fn jump_to_char(&mut self, c: char) {
        let target = c.to_ascii_lowercase();
        let len = self.entries.len();
        if len == 0 { return; }
        let current = self.state.selected().unwrap_or(0);
        // search from current+1, wrapping — cycles through all matches on repeated key presses
        for offset in 1..=len {
            let i = (current + offset) % len;
            let first = self.entries[i].name.chars().next().unwrap_or('\0').to_ascii_lowercase();
            if first == target {
                self.state.select(Some(i));
                return;
            }
        }
    }

    pub fn effective_targets(&self) -> Vec<PathBuf> {
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
