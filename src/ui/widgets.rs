use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{block, SPINNER};
use crate::{app::App, types::RepoStatus};

pub(super) fn render_title(f: &mut Frame, app: &App, area: Rect) {
    let spinner = if app.is_running {
        let ch = SPINNER[(app.tick as usize / 2) % SPINNER.len()];
        format!(" {ch} ")
    } else {
        "   ".to_string()
    };
    let line = Line::from(vec![
        Span::styled(
            format!("  {} ", app.base_path),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(spinner, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        area,
    );
}

pub(super) fn render_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (sym, color) = repo_sym(&r.status);
            let selected = i == app.current_index;
            let name_style = if selected {
                Style::default().fg(Color::Black).bg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            let meta_style = if selected {
                Style::default().fg(Color::Black).bg(color)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let prefix = if selected { "▶ " } else { "  " };
            let mut spans = vec![
                Span::styled(prefix, name_style),
                Span::styled(sym, name_style),
                Span::raw(" "),
                Span::styled(r.name.clone(), name_style),
                Span::styled(format!("  {}", r.branch), meta_style),
            ];
            if r.ahead > 0 || r.behind > 0 {
                spans.push(Span::styled(
                    format!("  ↑{}↓{}", r.ahead, r.behind),
                    if selected {
                        Style::default().fg(Color::Black).bg(color)
                    } else {
                        Style::default().fg(Color::Blue)
                    },
                ));
            }
            if r.modified + r.staged + r.untracked > 0 {
                spans.push(Span::styled(
                    format!("  +{} ~{} ?{}", r.staged, r.modified, r.untracked),
                    if selected {
                        Style::default().fg(Color::Black).bg(color)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.current_index));
    f.render_stateful_widget(
        List::new(items)
            .block(block("Repositories", true))
            .highlight_style(Style::default()),
        area,
        &mut state,
    );
}

pub(super) fn render_help(f: &mut Frame, area: Rect) {
    let s = |c: Color| Style::default().fg(c).add_modifier(Modifier::BOLD);
    let d = Style::default().fg(Color::DarkGray);
    let g = s(Color::Green);
    let b = s(Color::Blue);
    let y = s(Color::Yellow);
    let m = s(Color::Magenta);
    let r = s(Color::Red);
    let c = s(Color::Cyan);
    let sl = Span::styled("/", d);
    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("↑↓", c),
            Span::styled(" navigate  ", d),
            Span::styled("Enter", c),
            Span::styled("/", d),
            Span::styled("s", c),
            Span::styled(" details  ", d),
            Span::styled("S", c),
            Span::styled(" status all", d),
        ]),
        Line::from(vec![
            Span::styled("u", g),
            sl.clone(),
            Span::styled("U", g),
            Span::styled(" pull  ", d),
            Span::styled("f", g),
            sl.clone(),
            Span::styled("F", g),
            Span::styled(" fetch", d),
        ]),
        Line::from(vec![
            Span::styled("l", b),
            sl.clone(),
            Span::styled("L", b),
            Span::styled(" log  ", d),
            Span::styled("d", r),
            sl.clone(),
            Span::styled("D", r),
            Span::styled(" discard", d),
        ]),
        Line::from(vec![
            Span::styled("c", y),
            sl.clone(),
            Span::styled("C", y),
            Span::styled(" cmd  ", d),
            Span::styled("p", m),
            sl.clone(),
            Span::styled("P", m),
            Span::styled(" pr  ", d),
            Span::styled("n", m),
            Span::styled(" pr create", d),
        ]),
        Line::from(vec![
            Span::styled("a", y),
            sl.clone(),
            Span::styled("A", y),
            Span::styled(" ci runs  ", d),
            Span::styled("x", r),
            sl.clone(),
            Span::styled("X", r),
            Span::styled(" security", d),
        ]),
        Line::from(vec![Span::styled("o", m), Span::styled(" open repo", d)]),
        Line::from(vec![
            Span::styled("s", c),
            Span::styled("/", d),
            Span::styled("S", c),
            Span::styled(" status", d),
        ]),
        Line::from(vec![
            Span::styled("r", y),
            Span::styled(" refresh  ", d),
            Span::styled("h", d),
            Span::styled(" help  ", d),
            Span::styled("q", r),
            Span::styled(" quit", d),
        ]),
    ])
    .block(block("Controls", false));
    f.render_widget(help, area);
}

pub(super) fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let (text, color) = if app.confirm_mode {
        (format!(" {}", app.status_line), Color::Red)
    } else if app.input_mode {
        let scope = if let Some(ref t) = app.input_target {
            t.as_str().to_string()
        } else if app.input_all {
            "ALL".to_string()
        } else {
            "current".to_string()
        };
        let shell_mode = app.input_buffer.starts_with('!');
        let (mode_label, color) = if shell_mode {
            ("sh", Color::Magenta)
        } else {
            ("git", Color::Green)
        };
        (format!(" [{}] {} > {}▌", scope, mode_label, app.input_buffer), color)
    } else if app.is_running {
        (format!(" {}", app.status_line), Color::Yellow)
    } else {
        (format!(" {}", app.status_line), Color::Cyan)
    };

    let border_color = if app.confirm_mode {
        Color::Red
    } else if app.input_mode {
        if app.input_buffer.starts_with('!') {
            Color::Magenta
        } else {
            Color::Green
        }
    } else {
        Color::DarkGray
    };

    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(color)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        ),
        area,
    );
}

fn repo_sym(status: &RepoStatus) -> (&'static str, Color) {
    match status {
        RepoStatus::Clean => ("✓", Color::Green),
        RepoStatus::Dirty => ("●", Color::Yellow),
        RepoStatus::Ahead => ("↑", Color::Blue),
        RepoStatus::Behind => ("↓", Color::Magenta),
        RepoStatus::Diverged => ("⇅", Color::Red),
    }
}
