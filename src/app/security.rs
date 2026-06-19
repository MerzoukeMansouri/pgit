use super::App;
use crate::types::{GitRepo, SecurityAlert};
use tokio::sync::mpsc;

impl App {
    pub fn fetch_alerts(&mut self, all: bool) {
        let repos: Vec<_> = if all {
            self.repos.clone()
        } else if !self.repos.is_empty() {
            vec![self.repos[self.current_index].clone()]
        } else {
            return;
        };

        let (tx, rx) = mpsc::unbounded_channel::<SecurityAlert>();
        let tasks: Vec<_> = repos
            .into_iter()
            .flat_map(|r| {
                let r2 = r.clone();
                let r3 = r.clone();
                [
                    tokio::spawn(fetch_dependabot(r, tx.clone())),
                    tokio::spawn(fetch_code_scanning(r2, tx.clone())),
                    tokio::spawn(fetch_secret_scanning(r3, tx.clone())),
                ]
            })
            .collect();
        tokio::spawn(async move {
            for t in tasks {
                t.await.ok();
            }
        });
        drop(tx);

        self.alert_list = vec![];
        self.alert_index = 0;
        self.security_mode = true;
        self.alert_rx = Some(rx);
        self.status_line = "⟳ fetching security alerts...".to_string();
    }

    pub fn drain_alerts(&mut self) -> bool {
        let mut received = vec![];
        let done = App::drain_channel(&mut self.alert_rx, &mut received);
        if !received.is_empty() {
            self.alert_list.extend(received);
        }
        if done {
            self.alert_list
                .sort_by_key(|a| (alert_severity_key(&a.severity), kind_key(&a.kind)));
            self.is_running = false;
            self.status_line = if self.alert_list.is_empty() {
                "No open security alerts found.".to_string()
            } else {
                format!(
                    "{} alert(s)  ·  Enter open  ·  Esc close",
                    self.alert_list.len()
                )
            };
        }
        done
    }

    pub fn alert_open_web(&self) {
        if let Some(alert) = self.alert_list.get(self.alert_index) {
            let _ = std::process::Command::new("open").arg(&alert.url).spawn();
        }
    }
}

async fn gh_api_lines(path: &std::path::Path, endpoint: &str) -> Vec<serde_json::Value> {
    let out = tokio::process::Command::new("gh")
        .args(["api", endpoint, "--jq", ".[] | select(.state == \"open\")"])
        .current_dir(path)
        .output()
        .await;
    let Ok(o) = out else { return vec![] };
    if !o.status.success() {
        return vec![];
    }
    let stdout = String::from_utf8_lossy(&o.stdout);
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

async fn fetch_dependabot(r: GitRepo, tx: mpsc::UnboundedSender<SecurityAlert>) {
    for v in gh_api_lines(&r.path, "/repos/{owner}/{repo}/dependabot/alerts").await {
        if let Some(alert) = (|| -> Option<SecurityAlert> {
            Some(SecurityAlert {
                repo: r.name.clone(),
                repo_path: r.path.clone(),
                number: v["number"].as_u64()?,
                kind: "dep".to_string(),
                package: v["dependency"]["package"]["name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                severity: v["security_advisory"]["severity"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                summary: v["security_advisory"]["summary"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                cve_id: v["security_advisory"]["cve_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                url: v["html_url"].as_str().unwrap_or("").to_string(),
            })
        })() {
            let _ = tx.send(alert);
        }
    }
}

async fn fetch_code_scanning(r: GitRepo, tx: mpsc::UnboundedSender<SecurityAlert>) {
    for v in gh_api_lines(&r.path, "/repos/{owner}/{repo}/code-scanning/alerts").await {
        if let Some(alert) = (|| -> Option<SecurityAlert> {
            let severity = v["rule"]["security_severity_level"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| map_code_severity(v["rule"]["severity"].as_str().unwrap_or("")))
                .to_string();
            Some(SecurityAlert {
                repo: r.name.clone(),
                repo_path: r.path.clone(),
                number: v["number"].as_u64()?,
                kind: "code".to_string(),
                package: v["rule"]["id"].as_str().unwrap_or("unknown").to_string(),
                severity,
                summary: v["rule"]["description"].as_str().unwrap_or("").to_string(),
                cve_id: String::new(),
                url: v["html_url"].as_str().unwrap_or("").to_string(),
            })
        })() {
            let _ = tx.send(alert);
        }
    }
}

async fn fetch_secret_scanning(r: GitRepo, tx: mpsc::UnboundedSender<SecurityAlert>) {
    for v in gh_api_lines(&r.path, "/repos/{owner}/{repo}/secret-scanning/alerts").await {
        if let Some(alert) = (|| -> Option<SecurityAlert> {
            let secret_type = v["secret_type_display_name"]
                .as_str()
                .unwrap_or("secret")
                .to_string();
            Some(SecurityAlert {
                repo: r.name.clone(),
                repo_path: r.path.clone(),
                number: v["number"].as_u64()?,
                kind: "secret".to_string(),
                package: secret_type.clone(),
                severity: "critical".to_string(),
                summary: secret_type,
                cve_id: String::new(),
                url: v["html_url"].as_str().unwrap_or("").to_string(),
            })
        })() {
            let _ = tx.send(alert);
        }
    }
}

fn map_code_severity(s: &str) -> &'static str {
    match s {
        "error" => "high",
        "warning" => "medium",
        _ => "low",
    }
}

pub(crate) fn alert_severity_key(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

fn kind_key(kind: &str) -> u8 {
    match kind {
        "dep" => 0,
        "code" => 1,
        "secret" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_sort_order() {
        assert!(alert_severity_key("critical") < alert_severity_key("high"));
        assert!(alert_severity_key("high") < alert_severity_key("medium"));
        assert!(alert_severity_key("medium") < alert_severity_key("low"));
        assert!(alert_severity_key("low") < alert_severity_key("unknown"));
    }

    #[test]
    fn code_severity_mapping() {
        assert_eq!(map_code_severity("error"), "high");
        assert_eq!(map_code_severity("warning"), "medium");
        assert_eq!(map_code_severity("note"), "low");
        assert_eq!(map_code_severity("none"), "low");
    }
}
