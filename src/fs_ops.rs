use std::{fs, io, path::{Path, PathBuf}};

pub fn auto_rename(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = path.parent().unwrap_or(Path::new("."));
    let candidate = dir.join(format!("{stem}_copy{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    let mut n = 2u32;
    loop {
        let c = dir.join(format!("{stem}_copy{n}{ext}"));
        if !c.exists() { return c; }
        n += 1;
    }
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
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
