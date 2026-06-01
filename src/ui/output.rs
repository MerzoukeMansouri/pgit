use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::{block, grid_shape, split_equal_h, split_equal_v, SPINNER};
use crate::app::App;

pub(super) fn render_output(f: &mut Frame, app: &App, area: Rect) {
    let n = app.repo_output.len();
    if n == 0 {
        f.render_widget(Paragraph::new("").block(block("Output", false)), area);
        return;
    }
    let (cols, rows) = grid_shape(n);
    let row_areas = split_equal_v(area, rows);
    for (row_i, row_area) in row_areas.iter().copied().enumerate() {
        let start = row_i * cols;
        let col_areas = split_equal_h(row_area, cols);
        for (col_i, pane_area) in col_areas.into_iter().enumerate() {
            let idx = start + col_i;
            if idx < n {
                let (label, lines) = &app.repo_output[idx];
                render_mini_terminal(f, app, idx, label, lines, pane_area);
            } else {
                f.render_widget(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
                    pane_area,
                );
            }
        }
    }
}

fn render_mini_terminal(f: &mut Frame, app: &App, idx: usize, label: &str, lines: &[String], area: Rect) {
    let focused = app.focused_pane == Some(idx);
    let spinner = if app.is_running && lines.is_empty() {
        let ch = SPINNER[(app.tick as usize / 2) % SPINNER.len()];
        format!(" {ch}")
    } else {
        String::new()
    };
    let border_color = if focused {
        Color::Cyan
    } else if app.is_running {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let title_style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    };
    let text: Vec<Line> = lines.iter().map(|l| colorize_output_line(l)).collect();
    let scroll = text.len().saturating_sub(area.height as usize - 2) as u16;
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(format!(" {label}{spinner} "), title_style))
                    .title_bottom(if focused {
                        Span::styled(" Enter / click again to run cmd ", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::raw("")
                    }),
            )
            .scroll((scroll, 0)),
        area,
    );
}

pub(crate) fn colorize_output_line(line: &str) -> Line<'_> {
    if line.starts_with("\n=== ") || line.starts_with("=== ") {
        Line::from(Span::styled(
            line.trim_start(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
    } else if line.starts_with("⚠") {
        Line::from(Span::styled(line, Style::default().fg(Color::Yellow)))
    } else if line.starts_with("✗") {
        Line::from(Span::styled(line, Style::default().fg(Color::Red)))
    } else if line.starts_with("✓") {
        Line::from(Span::styled(line, Style::default().fg(Color::Green)))
    } else if line.contains("Already up to date") {
        Line::from(Span::styled(line, Style::default().fg(Color::DarkGray)))
    } else if line.contains("Successfully rebased") || line.contains("Fast-forward") {
        Line::from(Span::styled(line, Style::default().fg(Color::Green)))
    } else {
        Line::from(Span::styled(line, Style::default().fg(Color::White)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg(line: &Line) -> Option<Color> {
        line.spans.first()?.style.fg
    }

    #[test]
    fn colorize_check_ok() {
        assert_eq!(fg(&colorize_output_line("✓ done")), Some(Color::Green));
        assert_eq!(fg(&colorize_output_line("✗ error")), Some(Color::Red));
        assert_eq!(fg(&colorize_output_line("⚠ warn")), Some(Color::Yellow));
        assert_eq!(fg(&colorize_output_line("Already up to date")), Some(Color::DarkGray));
        assert_eq!(fg(&colorize_output_line("Successfully rebased")), Some(Color::Green));
        assert_eq!(fg(&colorize_output_line("Fast-forward")), Some(Color::Green));
        assert_eq!(fg(&colorize_output_line("normal text")), Some(Color::White));
        assert_eq!(fg(&colorize_output_line("=== header")), Some(Color::Cyan));
    }
}
