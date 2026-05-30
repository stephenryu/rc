mod app;
mod fs_ops;
mod panel;
mod session;
mod ui;

use std::{io, path::PathBuf, time::Duration};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use ui::ui;

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

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let right_path = cli.right.or_else(session::load_right);
    let mut app = App::new(cli.left, right_path);
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !app.handle_key(key.code, key.modifiers) {
                        break;
                    }
                }
            }
        }
    }
    session::save_right(&app.right.cwd);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
