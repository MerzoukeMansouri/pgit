use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub struct Target {
    pub label: String,
    pub workdir: PathBuf,
}

/// Runs `program args` concurrently across all targets. Each task sends its
/// `(label, lines)` batch atomically — no interleaving between repos. The
/// channel closes once all tasks complete.
pub fn run(targets: Vec<Target>, program: String, args: Vec<String>) -> UnboundedReceiver<(String, Vec<String>)> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let tasks: Vec<_> = targets
            .into_iter()
            .map(|t| {
                let tx = tx.clone();
                let program = program.clone();
                let args = args.clone();
                tokio::spawn(async move {
                    let mut lines = Vec::new();
                    match Command::new(&program)
                        .args(&args)
                        .current_dir(&t.workdir)
                        .output()
                        .await
                    {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            lines.extend(stdout.lines().map(str::to_string));
                            lines.extend(stderr.lines().map(|l| format!("⚠ {l}")));
                            if stdout.is_empty() && stderr.is_empty() {
                                lines.push("✓ done".to_string());
                            }
                        }
                        Err(e) => lines.push(format!("✗ {e}")),
                    }
                    let _ = tx.send((t.label, lines));
                })
            })
            .collect();

        for t in tasks {
            t.await.ok();
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_captures_stdout() {
        let dir = std::env::temp_dir();
        let mut rx = run(
            vec![Target {
                label: "t".into(),
                workdir: dir,
            }],
            "echo".into(),
            vec!["hello_gitp".into()],
        );
        let mut results = vec![];
        while let Some(msg) = rx.recv().await {
            results.push(msg);
        }
        assert_eq!(results.len(), 1);
        let (label, lines) = &results[0];
        assert_eq!(label, "t");
        assert!(lines.iter().any(|l| l.contains("hello_gitp")));
    }

    #[tokio::test]
    async fn run_parallel_labels() {
        let dir = std::env::temp_dir();
        let targets = vec![
            Target {
                label: "a".into(),
                workdir: dir.clone(),
            },
            Target {
                label: "b".into(),
                workdir: dir,
            },
        ];
        let mut rx = run(targets, "echo".into(), vec!["ok".into()]);
        let mut labels = vec![];
        while let Some((label, _)) = rx.recv().await {
            labels.push(label);
        }
        labels.sort();
        assert_eq!(labels, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn run_bad_command_produces_error_line() {
        let dir = std::env::temp_dir();
        let mut rx = run(
            vec![Target {
                label: "err".into(),
                workdir: dir,
            }],
            "this_cmd_does_not_exist_gitp".into(),
            vec![],
        );
        let (_, lines) = rx.recv().await.unwrap();
        assert!(lines.iter().any(|l| l.starts_with('✗')));
    }
}
