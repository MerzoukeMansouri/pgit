use super::App;
use crate::{engine::Target, types::PrItem};
use tokio::sync::mpsc;

impl App {
    pub fn fetch_prs(&mut self, all: bool) {
        let repos: Vec<_> = if all {
            self.repos.clone()
        } else if !self.repos.is_empty() {
            vec![self.repos[self.current_index].clone()]
        } else {
            return;
        };

        let (tx, rx) = mpsc::unbounded_channel::<PrItem>();
        let tasks: Vec<_> = repos
            .into_iter()
            .map(|r| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let out = tokio::process::Command::new("gh")
                        .args([
                            "pr",
                            "list",
                            "--json",
                            "number,title,author,headRefName,url,createdAt",
                            "--limit",
                            "50",
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
                    for v in vals {
                        if let Some(pr) = (|| {
                            Some(PrItem {
                                repo: r.name.clone(),
                                number: v["number"].as_u64()?,
                                title: v["title"].as_str()?.to_string(),
                                author: v["author"]["login"].as_str().unwrap_or("unknown").to_string(),
                                branch: v["headRefName"].as_str()?.to_string(),
                                url: v["url"].as_str()?.to_string(),
                                created_at: v["createdAt"].as_str().unwrap_or("").to_string(),
                            })
                        })() {
                            let _ = tx.send(pr);
                        }
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

        self.pr_list = vec![];
        self.pr_filtered = vec![];
        self.pr_filter.clear();
        self.pr_filter_mode = false;
        self.pr_index = 0;
        self.pr_mode = true;
        self.pr_rx = Some(rx);
        self.status_line = "⟳ fetching PRs...".to_string();
    }

    pub fn apply_pr_filter(&mut self) {
        let f = self.pr_filter.to_lowercase();
        let mut filtered: Vec<PrItem> = self
            .pr_list
            .iter()
            .filter(|pr| f.is_empty() || pr.author.to_lowercase().contains(&f))
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        self.pr_filtered = filtered;
        self.pr_index = 0;
    }

    pub fn drain_prs(&mut self) {
        let mut received = vec![];
        let done = App::drain_channel(&mut self.pr_rx, &mut received);
        self.pr_list.extend(received);
        self.apply_pr_filter();
        if done {
            self.status_line = if self.pr_filtered.is_empty() {
                "No open PRs found.".to_string()
            } else {
                format!(
                    "{} open PR(s)  ·  Enter/o open  ·  c checkout  ·  / filter by author  ·  Esc close",
                    self.pr_filtered.len()
                )
            };
        }
    }

    pub fn pr_open_web(&self) {
        if let Some(pr) = self.pr_filtered.get(self.pr_index) {
            let _ = std::process::Command::new("open").arg(&pr.url).spawn();
        }
    }

    pub fn pr_checkout(&mut self) {
        let Some(pr) = self.pr_filtered.get(self.pr_index).cloned() else {
            return;
        };
        let Some(repo) = self.repos.iter().find(|r| r.name == pr.repo).cloned() else {
            return;
        };
        self.pr_mode = false;
        let target = Target {
            label: repo.name.clone(),
            workdir: repo.path.clone(),
        };
        self.run_on(vec![target], "git", &["checkout", &pr.branch]);
    }
}
