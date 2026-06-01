#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::many_single_char_names,   // intentional in help widget color vars
    clippy::struct_excessive_bools,   // App UI state — all fields are needed
    clippy::cast_possible_truncation, // TUI coords always fit usize/u16
    clippy::doc_markdown,
)]

mod app;
mod engine;
mod git;
mod types;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{io, time::Duration};
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<()> {
    let base = std::env::current_dir()?;
    let mut app = App::new(base.to_str().unwrap())?;

    if app.repos.is_empty() {
        eprintln!("No git repositories found in current directory");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    app.refresh().await;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut tick = interval(Duration::from_millis(100));

    loop {
        tick.tick().await;
        app.tick = app.tick.wrapping_add(1);

        if app.drain() {
            app.refresh().await;
        }

        app.drain_ci();
        app.drain_prs();

        if !app.is_running && app.auto_refresh && app.last_refresh.elapsed() > Duration::from_secs(30) {
            app.refresh().await;
        }

        terminal.draw(|f| ui::draw(f, app))?;

        let mut nav_done = false;
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let is_nav = matches!(key.code, KeyCode::Up | KeyCode::Down);
                    if is_nav && nav_done {
                        continue;
                    }
                    if is_nav {
                        nav_done = true;
                    }
                    if handle_key(app, key.code).await? {
                        return Ok(());
                    }
                }
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Left) => {
                    handle_click(app, me.column, me.row, terminal.get_frame().area());
                }
                _ => {}
            }
        }
    }
}

fn handle_click(app: &mut App, col: u16, row: u16, size: Rect) {
    if app.pr_mode || app.repo_output.is_empty() {
        return;
    }
    let rects = ui::pane_rects(size, app.repo_output.len());
    for (i, rect) in rects.iter().enumerate() {
        if col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height {
            if app.focused_pane == Some(i) {
                // second click → open input for this repo
                open_pane_input(app, i);
            } else {
                app.focused_pane = Some(i);
                let name = app.repo_output[i].0.clone();
                app.status_line = format!("focused: {name}  ·  Enter cmd  ·  Esc unfocus");
            }
            return;
        }
    }
    app.focused_pane = None;
}

fn open_pane_input(app: &mut App, pane_idx: usize) {
    let name = app.repo_output[pane_idx].0.clone();
    app.focused_pane = None;
    app.input_target = Some(name);
    app.input_mode = true;
    app.input_buffer.clear();
}

async fn handle_key(app: &mut App, code: KeyCode) -> Result<bool> {
    if app.ci_mode {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => app.ci_mode = false,
            KeyCode::Up => {
                if app.ci_index > 0 {
                    app.ci_index -= 1;
                }
            }
            KeyCode::Down => {
                if app.ci_index + 1 < app.ci_list.len() {
                    app.ci_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') => app.ci_open_web(),
            KeyCode::Char('l') => app.ci_show_logs(),
            KeyCode::Char('R') => app.ci_rerun(),
            _ => {}
        }
        return Ok(false);
    }

    if app.pr_mode {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => app.pr_mode = false,
            KeyCode::Up => {
                if app.pr_index > 0 {
                    app.pr_index -= 1;
                }
            }
            KeyCode::Down => {
                if app.pr_index + 1 < app.pr_list.len() {
                    app.pr_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') => app.pr_open_web(),
            KeyCode::Char('c') => app.pr_checkout(),
            _ => {}
        }
        return Ok(false);
    }

    if app.confirm_mode {
        if let KeyCode::Char('y' | 'Y') = code {
            let all = app.confirm_all;
            app.confirm_mode = false;
            app.discard_dirty(all);
        } else {
            app.confirm_mode = false;
            app.status_line = "Discard cancelled.".to_string();
        }
        return Ok(false);
    }

    if app.input_mode {
        match code {
            KeyCode::Esc => {
                app.input_mode = false;
                app.input_target = None;
                app.input_buffer.clear();
                app.status_line = "Cancelled.".to_string();
            }
            KeyCode::Enter => {
                let all = app.input_all;
                let target = app.input_target.take();
                app.input_mode = false;
                let raw = app.input_buffer.trim().to_string();
                app.input_buffer.clear();
                if !raw.is_empty() {
                    let (program, args_str) = if let Some(rest) = raw.strip_prefix('!') {
                        let mut parts = rest.split_whitespace();
                        let prog = parts.next().unwrap_or("").to_string();
                        let args: Vec<&str> = parts.collect();
                        (prog, args)
                    } else {
                        ("git".to_string(), raw.split_whitespace().collect())
                    };
                    if let Some(name) = target {
                        if let Some(repo) = app.repos.iter().find(|r| r.name == name).cloned() {
                            let t = engine::Target {
                                label: repo.name.clone(),
                                workdir: repo.path.clone(),
                            };
                            app.run_on(vec![t], &program, &args_str);
                        }
                    } else if program == "git" {
                        app.run_git(&args_str, all);
                    } else {
                        app.run_cmd(&program, &args_str, all);
                    }
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
            }
            _ => {}
        }
        return Ok(false);
    }

    // focused pane shortcuts
    if let Some(i) = app.focused_pane {
        match code {
            KeyCode::Esc => {
                app.focused_pane = None;
                return Ok(false);
            }
            KeyCode::Enter | KeyCode::Char('c') => {
                open_pane_input(app, i);
                return Ok(false);
            }
            _ => {}
        }
    }

    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('h') => app.show_help = !app.show_help,
        KeyCode::Esc => app.focused_pane = None,
        KeyCode::Up => app.previous(),
        KeyCode::Down => app.next(),
        KeyCode::Enter | KeyCode::Char('s') => app.show_details(),

        KeyCode::Char('u') => app.run_git(&["pull", "--rebase"], false),
        KeyCode::Char('U') => app.run_git(&["pull", "--rebase"], true),
        KeyCode::Char('f') => app.run_git(&["fetch", "--all"], false),
        KeyCode::Char('F') => app.run_git(&["fetch", "--all"], true),
        KeyCode::Char('l') => app.run_git(&["log", "--oneline", "-10"], false),
        KeyCode::Char('L') => app.run_git(&["log", "--oneline", "-10"], true),
        KeyCode::Char('d') if !app.is_running => {
            app.confirm_mode = true;
            app.confirm_all = false;
            let name = app.repos.get(app.current_index).map_or("current", |r| r.name.as_str());
            app.status_line = format!("⚠ Discard changes in {name}? [y/N]");
        }
        KeyCode::Char('D') if !app.is_running => {
            app.confirm_mode = true;
            app.confirm_all = true;
            app.status_line = "⚠ Discard ALL changes in ALL repos? [y/N]".to_string();
        }
        KeyCode::Char('S') => app.run_git(&["status", "-sb"], true),

        KeyCode::Char('a') => app.fetch_runs(false),
        KeyCode::Char('A') => app.fetch_runs(true),
        KeyCode::Char('p') => app.fetch_prs(false),
        KeyCode::Char('P') => app.fetch_prs(true),
        KeyCode::Char('o') => app.run_cmd("gh", &["repo", "view", "--web"], false),
        KeyCode::Char('n') => app.run_cmd("gh", &["pr", "create", "--fill", "--web"], false),

        KeyCode::Char('c') if !app.is_running => {
            app.input_mode = true;
            app.input_all = false;
            app.input_target = None;
            app.input_buffer.clear();
        }
        KeyCode::Char('C') if !app.is_running => {
            app.input_mode = true;
            app.input_all = true;
            app.input_target = None;
            app.input_buffer.clear();
        }
        KeyCode::Char('r') if !app.is_running => {
            app.refresh().await;
            app.status_line = "Refreshed.".to_string();
        }

        _ => {}
    }
    Ok(false)
}
