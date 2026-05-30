use std::path::Path;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    app::{ActivePanel, App, Mode},
    panel::Panel,
};

pub fn ui(f: &mut Frame, app: &mut App) {
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
    f.render_widget(
        Paragraph::new(msg).style(Style::default().bg(Color::Blue).fg(Color::White)),
        rows[1],
    );

    let fkeys = Line::from(vec![
        key_span("F1", "About"), key_span("F2", "Rename"), key_span("F3", "View"), key_span("F4", "Edit"),
        key_span("F5", "Copy"),  key_span("F6", "Move"), key_span("F7", "MkDir"),
        key_span("F8", "Del"),   key_span("^Q", "Quit"),
    ]);
    f.render_widget(Paragraph::new(fkeys).style(Style::default().bg(Color::Black)), rows[2]);

    match &app.mode {
        Mode::About => render_about(f, area),
        Mode::Input { prompt, value, .. } => render_input(f, area, prompt, value),
        Mode::Viewer { path, lines, scroll } => render_viewer(f, area, path, lines, *scroll),
        Mode::Confirm { prompt, .. } => render_confirm(f, area, prompt),
        Mode::Conflict { src, dst, queue, done, .. } =>
            render_conflict(f, area, src, dst, *done, queue.len()),
        Mode::Normal => {}
    }
}

// ── panel ─────────────────────────────────────────────────────────────────────

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
            let size_str = if e.is_dir { "<DIR>     ".into() } else { format_size(e.size) };
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

// ── overlays ──────────────────────────────────────────────────────────────────

fn render_about(f: &mut Frame, area: Rect) {
    let popup = centered_rect(50, 40, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(
            " About rc ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

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
        Line::from("  F2         Rename"),
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

fn render_conflict(
    f: &mut Frame,
    area: Rect,
    src: &Path,
    dst: &Path,
    done: usize,
    remaining: usize,
) {
    let popup = centered_rect(64, 10, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(
            format!(" File Conflict  ({done} done, {remaining} remaining) "),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    let fname   = dst.file_name().unwrap_or_default().to_string_lossy();
    let dst_dir = dst.parent().map(|p| p.to_string_lossy()).unwrap_or_default();
    let src_dir = src.parent().map(|p| p.to_string_lossy()).unwrap_or_default();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  File: "),
            Span::styled(fname.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::styled(format!("  From: {src_dir}"), Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(format!("  To:   {dst_dir}"), Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(vec![
            key_hint("O", "Overwrite"), Span::raw("  "),
            key_hint("A", "All"),       Span::raw("  "),
            key_hint("S", "Skip"),      Span::raw("  "),
            key_hint("N", "None"),      Span::raw("  "),
            key_hint("R", "Rename"),    Span::raw("  "),
            key_hint("Esc", "Cancel"),
        ]),
    ];

    f.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_input(f: &mut Frame, area: Rect, prompt: &str, value: &str) {
    let popup = centered_rect(50, 5, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(" Input ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    f.render_widget(Paragraph::new(format!("{prompt}\n> {value}_")).block(block), popup);
}

fn render_confirm(f: &mut Frame, area: Rect, prompt: &str) {
    let popup = centered_rect(60, 5, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(" Confirm ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
    f.render_widget(Paragraph::new(prompt.to_string()).block(block), popup);
}

fn render_viewer(f: &mut Frame, area: Rect, path: &Path, lines: &[Line], scroll: usize) {
    let popup = centered_rect(90, 90, area);
    f.render_widget(Clear, popup);
    let title = format!(" {} ", path.file_name().unwrap_or_default().to_string_lossy());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(title, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let visible: Vec<ListItem> = lines
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|line| ListItem::new(line.clone()))
        .collect();

    let total = lines.len();
    let pct = if total == 0 { 0 } else { ((scroll + inner.height as usize) * 100 / total).min(100) };
    let info = format!(" {}/{} ({}%) — ESC to close ", scroll + 1, total, pct);
    let info_area = Rect { y: popup.bottom() - 1, x: popup.x + 1, width: popup.width - 2, height: 1 };
    f.render_widget(
        Paragraph::new(info).style(Style::default().bg(Color::Green).fg(Color::Black)),
        info_area,
    );

    f.render_widget(List::new(visible), inner);
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_h: u16, area: Rect) -> Rect {
    let popup_w = area.width * percent_x / 100;
    let popup_h = (area.height * percent_h / 100).max(5);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    Rect { x: area.x + x, y: area.y + y, width: popup_w, height: popup_h }
}

fn key_span(key: &str, label: &str) -> Span<'static> {
    Span::styled(format!("[{key}]{label} "), Style::default().fg(Color::Cyan))
}

fn key_hint(key: &str, label: &str) -> Span<'static> {
    Span::styled(format!("[{key}]{label}"), Style::default().fg(Color::Cyan))
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
