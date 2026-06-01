mod output;
mod tables;
mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    if app.ci_mode {
        let [table_area, status_area] = split_v(f.area(), [0, 3]);
        tables::render_ci_list(f, app, table_area);
        widgets::render_status(f, app, status_area);
        return;
    }
    if app.pr_mode {
        let [table_area, status_area] = split_v(f.area(), [0, 3]);
        tables::render_pr_list(f, app, table_area);
        widgets::render_status(f, app, status_area);
        return;
    }
    let [left, right] = split_h(f.area(), [30, 70]);
    let [output_area, status_area] = split_v(right, [0, 3]);
    output::render_output(f, app, output_area);
    widgets::render_status(f, app, status_area);
    if app.show_help {
        let [title_area, list_area, help_area] = split_v(left, [3, 0, 9]);
        widgets::render_title(f, app, title_area);
        widgets::render_repo_list(f, app, list_area);
        widgets::render_help(f, help_area);
    } else {
        let [title_area, list_area] = split_v(left, [3, 0]);
        widgets::render_title(f, app, title_area);
        widgets::render_repo_list(f, app, list_area);
    }
}

/// Compute output grid pane rects from the full terminal area.
/// Used for mouse click hit-testing — mirrors render_output layout exactly.
pub fn pane_rects(terminal_area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Min(1)])
        .split(terminal_area);
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(h[1]);
    let output_area = v[0];
    let (cols, rows) = grid_shape(n);
    let row_areas = split_equal_v(output_area, rows);
    let mut rects = Vec::new();
    for (row_i, row_area) in row_areas.iter().copied().enumerate() {
        let start = row_i * cols;
        let end = (start + cols).min(n);
        rects.extend(split_equal_h(row_area, end - start));
    }
    rects
}

// ── shared layout helpers (pub(super) → accessible to submodules) ─────────────

pub(super) fn grid_shape(n: usize) -> (usize, usize) {
    let cols = match n {
        1 => 1,
        2..=4 => 2,
        5..=9 => 3,
        _ => 4,
    };
    (cols, n.div_ceil(cols))
}

pub(super) fn split_h<const N: usize>(area: Rect, pcts: [u16; N]) -> [Rect; N] {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pcts.map(Constraint::Percentage))
        .split(area);
    std::array::from_fn(|i| chunks[i])
}

pub(super) fn split_v<const N: usize>(area: Rect, lengths: [u16; N]) -> [Rect; N] {
    let constraints: Vec<Constraint> = lengths
        .iter()
        .map(|&l| {
            if l == 0 {
                Constraint::Min(1)
            } else {
                Constraint::Length(l)
            }
        })
        .collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    std::array::from_fn(|i| chunks[i])
}

pub(super) fn split_equal_h(area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let pct = 100u16 / n as u16;
    let constraints: Vec<Constraint> = (0..n)
        .map(|i| {
            if i == n - 1 {
                Constraint::Min(1)
            } else {
                Constraint::Percentage(pct)
            }
        })
        .collect();
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub(super) fn split_equal_v(area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let pct = 100u16 / n as u16;
    let constraints: Vec<Constraint> = (0..n)
        .map(|i| {
            if i == n - 1 {
                Constraint::Min(1)
            } else {
                Constraint::Percentage(pct)
            }
        })
        .collect();
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub(super) fn block(title: &str, active: bool) -> ratatui::widgets::Block<'_> {
    use ratatui::{
        style::Modifier,
        style::{Color, Style},
        text::Span,
        widgets::{Block, BorderType, Borders},
    };
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ))
}

pub(super) const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_shape_boundaries() {
        assert_eq!(grid_shape(1), (1, 1));
        assert_eq!(grid_shape(2), (2, 1));
        assert_eq!(grid_shape(3), (2, 2));
        assert_eq!(grid_shape(4), (2, 2));
        assert_eq!(grid_shape(5), (3, 2));
        assert_eq!(grid_shape(9), (3, 3));
        assert_eq!(grid_shape(10), (4, 3));
        assert_eq!(grid_shape(12), (4, 3));
        assert_eq!(grid_shape(13), (4, 4));
    }
}
