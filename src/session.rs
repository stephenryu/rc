use std::{
    fs,
    path::{Path, PathBuf},
};

fn state_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".rc"))
}

pub fn load_right() -> Option<PathBuf> {
    let path = state_dir()?.join("right");
    let raw = fs::read_to_string(&path).ok()?;
    let p = PathBuf::from(raw.trim());
    if p.is_dir() { Some(p) } else { None }
}

pub fn save_right(cwd: &Path) {
    let Some(dir) = state_dir() else { return };
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join("right"), cwd.to_string_lossy().as_ref());
}
