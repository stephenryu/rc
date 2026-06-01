use std::path::Path;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

pub fn highlight(path: &Path, text: &str) -> Vec<Line<'static>> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    LinesWithEndings::from(text)
        .map(|line| {
            let ranges = h.highlight_line(line, &ps).unwrap_or_default();
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    Span::styled(
                        text.trim_end_matches(|c: char| c == '\n' || c == '\r').replace('\t', "    ").to_owned(),
                        Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)).bg(Color::Black),
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}
