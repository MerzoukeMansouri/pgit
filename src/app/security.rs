use super::App;
use crate::types::SecurityAlert;
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
                let tx1 = tx.clone();
                let tx2 = tx.clone();
                let tx3 = tx.clone();
                let r1 = r.clone();
                let r2 = r.clone();
                let r3 = r.clone();
                [
                    tokio::spawn(async move { fetch_dependabot(&r1, tx1).await }),
                    tokio::spawn(async move { fetch_code_scanning(&r2, tx2).await }),
                    tokio::spawn(async move { fetch_secret_scanning(&r3, tx3).await }),
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
            self.alert_list
                .sort_by_key(|a| (alert_severity_key(&a.severity), kind_key(&a.kind)));
        }
        if done {
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

async fn fetch_dependabot(r: &crate::types::GitRepo, tx: mpsc::UnboundedSender<SecurityAlert>) {
    let out = tokio::process::Command::new("gh")
        .args([
            "api",
            "/repos/{owner}/{repo}/dependabot/alerts",
            "--jq",
            ".[] | select(.state == \"open\")",
        ])
        .current_dir(&r.path)
        .output()
        .await;
    let Ok(o) = out else { return };
    if !o.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
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

async fn fetch_code_scanning(r: &crate::types::GitRepo, tx: mpsc::UnboundedSender<SecurityAlert>) {
    let out = tokio::process::Command::new("gh")
        .args([
            "api",
            "/repos/{owner}/{repo}/code-scanning/alerts",
            "--jq",
            ".[] | select(.state == \"open\")",
        ])
        .current_dir(&r.path)
        .output()
        .await;
    let Ok(o) = out else { return };
    if !o.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
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
                summary: v["rule"]["description"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                cve_id: String::new(),
                url: v["html_url"].as_str().unwrap_or("").to_string(),
            })
        })() {
            let _ = tx.send(alert);
        }
    }
}

async fn fetch_secret_scanning(
    r: &crate::types::GitRepo,
    tx: mpsc::UnboundedSender<SecurityAlert>,
) {
    let out = tokio::process::Command::new("gh")
        .args([
            "api",
            "/repos/{owner}/{repo}/secret-scanning/alerts",
            "--jq",
            ".[] | select(.state == \"open\")",
        ])
        .current_dir(&r.path)
        .output()
        .await;
    let Ok(o) = out else { return };
    if !o.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
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
