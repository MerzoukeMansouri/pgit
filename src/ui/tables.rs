use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::app::App;

pub(super) fn render_pr_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["Repo", "#", "Title", "Author", "Branch", "Date"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .height(1);

    let rows: Vec<Row> = app
        .pr_filtered
        .iter()
        .enumerate()
        .map(|(i, pr)| {
            let style = if i == app.pr_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let date = pr.created_at.get(..10).unwrap_or(&pr.created_at).to_string();
            Row::new(vec![
                Cell::from(pr.repo.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("#{}", pr.number)).style(Style::default().fg(Color::Yellow)),
                Cell::from(pr.title.clone()),
                Cell::from(pr.author.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(pr.branch.clone()).style(Style::default().fg(Color::Blue)),
                Cell::from(date).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(15),
        Constraint::Length(6),
        Constraint::Percentage(36),
        Constraint::Percentage(16),
        Constraint::Percentage(18),
        Constraint::Percentage(12),
    ];

    let title = if app.pr_filtered.is_empty() {
        " Pull Requests ".to_string()
    } else if !app.pr_filter.is_empty() {
        format!(
            " Pull Requests  ({}/{})  filter: {} ",
            app.pr_filtered.len(),
            app.pr_list.len(),
            app.pr_filter
        )
    } else {
        format!(" Pull Requests  ({}) ", app.pr_filtered.len())
    };

    let bottom_hint = if app.pr_filter_mode {
        format!(" filter: {}█ ", app.pr_filter)
    } else {
        " Enter/o open  ·  c checkout  ·  / filter by author  ·  Esc close ".to_string()
    };

    let mut state = TableState::default();
    state.select(Some(app.pr_index));
    f.render_stateful_widget(
        Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Style::default().bg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Magenta))
                    .title(Span::styled(
                        title,
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ))
                    .title_bottom(Span::styled(bottom_hint, Style::default().fg(Color::DarkGray))),
            ),
        area,
        &mut state,
    );
}

pub(super) fn render_ci_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["", "Repo", "Workflow", "Branch", "Event", "Started"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .height(1);

    let rows: Vec<Row> = app
        .ci_list
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let (sym, color) = ci_sym(&run.status, &run.conclusion);
            let sel = i == app.ci_index;
            let base = if sel {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(sym).style(Style::default().fg(color)),
                Cell::from(run.repo.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(run.workflow.clone()),
                Cell::from(run.branch.clone()).style(Style::default().fg(Color::Blue)),
                Cell::from(run.event.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(run.created_at.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(base)
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(18),
        Constraint::Percentage(30),
        Constraint::Percentage(22),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let mut state = TableState::default();
    state.select(Some(app.ci_index));
    f.render_stateful_widget(
        Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Style::default().bg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(
                        format!(" CI Runs ({}) ", app.ci_list.len()),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ))
                    .title_bottom(Span::styled(
                        " Enter open browser  ·  l details  ·  R re-run  ·  Esc close ",
                        Style::default().fg(Color::DarkGray),
                    )),
            ),
        area,
        &mut state,
    );
}

pub(crate) fn ci_sym(status: &str, conclusion: &str) -> (&'static str, Color) {
    match (status, conclusion) {
        ("in_progress", _) => ("⟳", Color::Yellow),
        ("queued", _) => ("◌", Color::Yellow),
        (_, "success") => ("✓", Color::Green),
        (_, "failure") => ("✗", Color::Red),
        (_, "cancelled") => ("○", Color::DarkGray),
        (_, "skipped") => ("–", Color::DarkGray),
        _ => ("?", Color::DarkGray),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_sym_mapping() {
        assert_eq!(ci_sym("in_progress", ""), ("⟳", Color::Yellow));
        assert_eq!(ci_sym("queued", ""), ("◌", Color::Yellow));
        assert_eq!(ci_sym("completed", "success"), ("✓", Color::Green));
        assert_eq!(ci_sym("completed", "failure"), ("✗", Color::Red));
        assert_eq!(ci_sym("completed", "cancelled"), ("○", Color::DarkGray));
    }
}
