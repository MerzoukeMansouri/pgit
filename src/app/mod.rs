mod ci;
mod commands;
mod pr;
mod security;

use anyhow::Result;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver};
use tokio::time::Instant;

use crate::{
    git,
    types::{CiRun, GitRepo, PrItem, SecurityAlert},
};

pub struct App {
    pub base_path: String,
    pub repos: Vec<GitRepo>,
    pub current_index: usize,
    pub repo_output: Vec<(String, Vec<String>)>,
    pub status_line: String,
    pub last_refresh: Instant,
    pub auto_refresh: bool,
    pub is_running: bool,
    pub input_mode: bool,
    pub input_all: bool,
    pub input_target: Option<String>,
    pub input_buffer: String,
    pub confirm_mode: bool,
    pub confirm_all: bool,
    pub focused_pane: Option<usize>,
    pub tick: u64,
    pub show_help: bool,
    pub pr_mode: bool,
    pub pr_list: Vec<PrItem>,
    pub pr_filtered: Vec<PrItem>,
    pub pr_filter: String,
    pub pr_filter_mode: bool,
    pub pr_index: usize,
    pub ci_mode: bool,
    pub ci_list: Vec<CiRun>,
    pub ci_index: usize,
    pub security_mode: bool,
    pub alert_list: Vec<SecurityAlert>,
    pub alert_index: usize,
    pub(crate) ci_rx: Option<UnboundedReceiver<CiRun>>,
    pub(crate) pr_rx: Option<UnboundedReceiver<PrItem>>,
    pub(crate) alert_rx: Option<UnboundedReceiver<SecurityAlert>>,
    pub(crate) rx: Option<UnboundedReceiver<(String, Vec<String>)>>,
}

impl App {
    pub fn new(base_path: &str) -> Result<Self> {
        let repos = git::find_repos(base_path)?;
        Ok(Self {
            base_path: std::path::Path::new(base_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(base_path)
                .to_string(),
            repos,
            current_index: 0,
            repo_output: vec![],
            status_line: "Loading...".to_string(),
            last_refresh: Instant::now(),
            auto_refresh: true,
            is_running: false,
            input_mode: false,
            input_all: false,
            input_target: None,
            input_buffer: String::new(),
            confirm_mode: false,
            confirm_all: false,
            focused_pane: None,
            tick: 0,
            show_help: false,
            pr_mode: false,
            pr_list: vec![],
            pr_filtered: vec![],
            pr_filter: String::new(),
            pr_filter_mode: false,
            pr_index: 0,
            ci_mode: false,
            ci_list: vec![],
            ci_index: 0,
            security_mode: false,
            alert_list: vec![],
            alert_index: 0,
            ci_rx: None,
            pr_rx: None,
            alert_rx: None,
            rx: None,
        })
    }

    pub async fn refresh(&mut self) {
        let repos = std::mem::take(&mut self.repos);
        let handles: Vec<_> = repos
            .into_iter()
            .map(|mut r| {
                tokio::task::spawn_blocking(move || {
                    git::update_status(&mut r);
                    r
                })
            })
            .collect();
        let mut updated = Vec::with_capacity(handles.len());
        for h in handles {
            if let Ok(r) = h.await {
                updated.push(r);
            }
        }
        self.repos = updated;
        if self.current_index >= self.repos.len() && !self.repos.is_empty() {
            self.current_index = self.repos.len() - 1;
        }
        self.last_refresh = Instant::now();
        self.status_line = format!("{} repos  ·  press h for help", self.repos.len());
    }

    pub fn next(&mut self) {
        if !self.repos.is_empty() {
            self.current_index = (self.current_index + 1) % self.repos.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.repos.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.repos.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    pub fn drain(&mut self) -> bool {
        let mut done = false;
        if let Some(rx) = &mut self.rx {
            loop {
                match rx.try_recv() {
                    Ok((label, lines)) => {
                        if let Some(pane) = self.repo_output.iter_mut().find(|(l, _)| l == &label) {
                            pane.1.extend(lines);
                        } else {
                            self.repo_output.push((label, lines));
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            self.rx = None;
            self.is_running = false;
            self.status_line = "Done.".to_string();
        }
        done
    }

    pub(crate) fn drain_channel<T>(rx: &mut Option<UnboundedReceiver<T>>, out: &mut Vec<T>) -> bool {
        let mut done = false;
        if let Some(receiver) = rx {
            loop {
                match receiver.try_recv() {
                    Ok(item) => out.push(item),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            *rx = None;
        }
        done
    }

    pub fn show_details(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let r = &self.repos[self.current_index].clone();
        let mut header = vec![
            format!("Branch : {}", r.branch),
            format!("Path   : {}", r.path.display()),
        ];
        if r.ahead > 0 || r.behind > 0 {
            header.push(format!("Sync   : ↑{} ↓{}", r.ahead, r.behind));
        }
        if r.modified + r.staged + r.untracked > 0 {
            header.push(format!("Changes: +{} ~{} ?{}", r.staged, r.modified, r.untracked));
        }
        header.push(String::new());
        self.repo_output = vec![(r.name.clone(), header)];
        self.run_git(&["status", "-sb"], false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn drain_channel_collects_items() {
        let (tx, rx) = mpsc::unbounded_channel::<u32>();
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        drop(tx);
        let mut opt: Option<UnboundedReceiver<u32>> = Some(rx);
        let mut out = vec![];
        assert!(App::drain_channel(&mut opt, &mut out));
        assert_eq!(out, vec![1, 2, 3]);
        assert!(opt.is_none());
    }

    #[tokio::test]
    async fn drain_channel_partial_not_done() {
        let (tx, rx) = mpsc::unbounded_channel::<u32>();
        tx.send(42).unwrap();
        let mut opt: Option<UnboundedReceiver<u32>> = Some(rx);
        let mut out = vec![];
        assert!(!App::drain_channel(&mut opt, &mut out));
        assert_eq!(out, vec![42]);
        assert!(opt.is_some());
    }

    #[tokio::test]
    async fn drain_channel_none_is_noop() {
        let mut opt: Option<UnboundedReceiver<u32>> = None;
        let mut out: Vec<u32> = vec![];
        assert!(!App::drain_channel(&mut opt, &mut out));
        assert!(out.is_empty());
    }
}
