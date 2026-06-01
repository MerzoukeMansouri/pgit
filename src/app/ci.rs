use super::App;
use crate::{engine::Target, types::CiRun};
use tokio::sync::mpsc;

impl App {
    pub fn fetch_runs(&mut self, all: bool) {
        let repos: Vec<_> = if all {
            self.repos.clone()
        } else if !self.repos.is_empty() {
            vec![self.repos[self.current_index].clone()]
        } else {
            return;
        };

        let (tx, rx) = mpsc::unbounded_channel::<CiRun>();
        let tasks: Vec<_> = repos
            .into_iter()
            .flat_map(|r| {
                let wf_files = workflow_files(&r.path);
                wf_files.into_iter().map(move |wf| (r.clone(), wf))
            })
            .map(|(r, wf)| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let out = tokio::process::Command::new("gh")
                        .args([
                            "run",
                            "list",
                            "--workflow",
                            &wf,
                            "--json",
                            "databaseId,workflowName,status,conclusion,headBranch,event,createdAt",
                            "--limit",
                            "1",
                        ])
                        .current_dir(&r.path)
                        .output()
                        .await;
                    let Ok(o) = out else { return };
                    if !o.status.success() {
                        return;
                    }
                    let json = String::from_utf8_lossy(&o.stdout);
                    let Ok(vals) = serde_json::from_str::<Vec<serde_json::Value>>(&json) else {
                        return;
                    };
                    let Some(v) = vals.into_iter().next() else { return };
                    if let Some(run) = (|| -> Option<CiRun> {
                        Some(CiRun {
                            repo: r.name.clone(),
                            repo_path: r.path.clone(),
                            id: v["databaseId"].as_u64()?,
                            workflow: v["workflowName"].as_str().unwrap_or(&wf).to_string(),
                            status: v["status"].as_str().unwrap_or("").to_string(),
                            conclusion: v["conclusion"].as_str().unwrap_or("").to_string(),
                            branch: v["headBranch"].as_str().unwrap_or("").to_string(),
                            event: v["event"].as_str().unwrap_or("").to_string(),
                            created_at: v["createdAt"].as_str().unwrap_or("").chars().take(16).collect(),
                        })
                    })() {
                        let _ = tx.send(run);
                    }
                })
            })
            .collect();
        tokio::spawn(async move {
            for t in tasks {
                t.await.ok();
            }
        });
        drop(tx);

        self.ci_list = vec![];
        self.ci_index = 0;
        self.ci_mode = true;
        self.ci_rx = Some(rx);
        self.status_line = "⟳ fetching CI runs...".to_string();
    }

    pub fn drain_ci(&mut self) -> bool {
        let mut received = vec![];
        let done = App::drain_channel(&mut self.ci_rx, &mut received);
        if !received.is_empty() {
            self.ci_list.extend(received);
            self.ci_list.sort_by_key(|r| ci_sort_key(&r.status, &r.conclusion));
        }
        if done {
            self.is_running = false;
            self.status_line = if self.ci_list.is_empty() {
                "No CI runs found.".to_string()
            } else {
                format!(
                    "{} run(s)  ·  Enter open  ·  l details  ·  R re-run  ·  Esc close",
                    self.ci_list.len()
                )
            };
        }
        done
    }

    pub fn ci_open_web(&self) {
        if let Some(run) = self.ci_list.get(self.ci_index) {
            let _ = std::process::Command::new("gh")
                .args(["run", "view", &run.id.to_string(), "--web"])
                .current_dir(&run.repo_path)
                .spawn();
        }
    }

    pub fn ci_rerun(&self) {
        if let Some(run) = self.ci_list.get(self.ci_index) {
            let _ = std::process::Command::new("gh")
                .args(["run", "rerun", &run.id.to_string()])
                .current_dir(&run.repo_path)
                .spawn();
        }
    }

    pub fn ci_show_logs(&mut self) {
        if let Some(run) = self.ci_list.get(self.ci_index).cloned() {
            let id = run.id.to_string();
            let target = Target {
                label: run.workflow.clone(),
                workdir: run.repo_path.clone(),
            };
            self.ci_mode = false;
            self.run_on(vec![target], "gh", &["run", "view", &id]);
        }
    }
}

pub(crate) fn ci_sort_key(status: &str, conclusion: &str) -> u8 {
    match (status, conclusion) {
        ("in_progress", _) => 0,
        ("queued", _) => 1,
        (_, "failure") => 2,
        (_, "success") => 3,
        _ => 4,
    }
}

fn workflow_files(repo_path: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(repo_path.join(".github/workflows"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let ext = std::path::Path::new(&name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(str::to_ascii_lowercase);
                    matches!(ext.as_deref(), Some("yml" | "yaml")).then_some(name)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sort_key_order() {
        assert!(ci_sort_key("in_progress", "") < ci_sort_key("queued", ""));
        assert!(ci_sort_key("queued", "") < ci_sort_key("", "failure"));
        assert!(ci_sort_key("", "failure") < ci_sort_key("", "success"));
        assert!(ci_sort_key("", "success") < ci_sort_key("", "unknown"));
    }

    #[test]
    fn workflow_files_filters_correctly() {
        let base = std::env::temp_dir().join(format!("gitp_wf_{}", std::process::id()));
        let wf_dir = base.join(".github/workflows");
        fs::create_dir_all(&wf_dir).unwrap();
        fs::write(wf_dir.join("ci.yml"), "").unwrap();
        fs::write(wf_dir.join("cd.yaml"), "").unwrap();
        fs::write(wf_dir.join("notes.md"), "").unwrap();
        let mut files = workflow_files(&base);
        files.sort();
        assert_eq!(files, vec!["cd.yaml", "ci.yml"]);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn workflow_files_missing_dir_returns_empty() {
        let base = std::env::temp_dir().join("gitp_wf_noexist_99999");
        assert!(workflow_files(&base).is_empty());
    }
}
