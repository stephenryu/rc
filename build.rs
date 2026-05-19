use std::process::Command;

fn main() {
    // rerun when HEAD or branch refs change
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");

    let build_date = build_date();
    let pkg_version = env!("CARGO_PKG_VERSION");

    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let full_version = match git_hash {
        Some(hash) => format!("{pkg_version} ({hash} {build_date})"),
        None       => format!("{pkg_version} ({build_date})"),
    };

    println!("cargo:rustc-env=RC_VERSION={full_version}");
    println!("cargo:rustc-env=RC_BUILD_DATE={build_date}");
}

fn build_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = ymd_from_epoch(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

fn ymd_from_epoch(secs: u64) -> (i32, u32, u32) {
    let mut days = (secs / 86400) as i32;
    let mut year = 1970i32;

    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        year += 1;
    }

    let month_days: [i32; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for ml in month_days {
        if days < ml { break; }
        days -= ml;
        month += 1;
    }

    (year, month, days as u32 + 1)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
