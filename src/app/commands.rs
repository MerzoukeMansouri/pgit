use super::App;
use crate::engine::{self, Target};

impl App {
    pub fn run_cmd(&mut self, program: &str, args: &[&str], all: bool) {
        if self.is_running {
            return;
        }
        let targets: Vec<Target> = if all {
            self.repos
                .iter()
                .map(|r| Target {
                    label: r.name.clone(),
                    workdir: r.path.clone(),
                })
                .collect()
        } else if !self.repos.is_empty() {
            let r = &self.repos[self.current_index];
            vec![Target {
                label: r.name.clone(),
                workdir: r.path.clone(),
            }]
        } else {
            self.repo_output = vec![("gitp".to_string(), vec!["⚠ No repositories available".to_string()])];
            return;
        };
        self.run_on(targets, program, args);
    }

    pub fn run_on(&mut self, targets: Vec<Target>, program: &str, args: &[&str]) {
        if self.is_running || targets.is_empty() {
            return;
        }
        let label = if targets.len() == 1 {
            targets[0].label.clone()
        } else {
            format!("{} repos", targets.len())
        };
        let args_owned: Vec<String> = args.iter().map(ToString::to_string).collect();
        self.focused_pane = None;
        self.repo_output = targets.iter().map(|t| (t.label.clone(), vec![])).collect();
        self.status_line = format!("⟳ {} {} → {}...", program, args_owned.join(" "), label);
        self.is_running = true;
        self.rx = Some(engine::run(targets, program.to_string(), args_owned));
    }

    pub fn run_git(&mut self, args: &[&str], all: bool) {
        self.run_cmd("git", args, all);
    }

    pub fn discard_dirty(&mut self, all: bool) {
        if self.is_running {
            return;
        }
        let candidates: Vec<_> = if all {
            self.repos
                .iter()
                .filter(|r| r.modified > 0 || r.staged > 0 || r.untracked > 0)
                .cloned()
                .collect()
        } else if !self.repos.is_empty() {
            let r = &self.repos[self.current_index];
            if r.modified > 0 || r.staged > 0 || r.untracked > 0 {
                vec![r.clone()]
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        let targets: Vec<Target> = candidates
            .into_iter()
            .map(|r| Target {
                label: r.name.clone(),
                workdir: r.path.clone(),
            })
            .collect();
        if targets.is_empty() {
            self.repo_output = vec![(
                "gitp".to_string(),
                vec!["✓ Nothing to discard — all repos clean.".to_string()],
            )];
            self.status_line = "Nothing to discard.".to_string();
            return;
        }
        self.run_on(targets, "sh", &["-c", "git reset --hard HEAD && git clean -fd"]);
    }
}
