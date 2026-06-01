use crate::types::{GitRepo, RepoStatus};
use anyhow::Result;
use std::{fs, process::Command};

pub fn find_repos(base_path: &str) -> Result<Vec<GitRepo>> {
    let mut repos = Vec::new();

    for entry in fs::read_dir(base_path)? {
        let path = entry?.path();
        if path.is_dir() && path.join(".git").exists() {
            repos.push(GitRepo {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path,
                status: RepoStatus::Clean,
                branch: "unknown".to_string(),
                ahead: 0,
                behind: 0,
                modified: 0,
                staged: 0,
                untracked: 0,
            });
        }
    }

    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

pub fn update_status(repo: &mut GitRepo) {
    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&repo.path)
        .output()
    {
        repo.branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }

    repo.ahead = 0;
    repo.behind = 0;
    if let Ok(out) = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(&repo.path)
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() == 2 {
            repo.ahead = parts[0].parse().unwrap_or(0);
            repo.behind = parts[1].parse().unwrap_or(0);
        }
    }

    repo.modified = 0;
    repo.staged = 0;
    repo.untracked = 0;
    if let Ok(out) = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo.path)
        .output()
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if line.len() < 2 {
                continue;
            }
            let b: Vec<char> = line.chars().collect();
            if b[0] != ' ' && b[0] != '?' {
                repo.staged += 1;
            }
            if b[1] == 'M' || b[1] == 'D' {
                repo.modified += 1;
            }
            if b[0] == '?' && b[1] == '?' {
                repo.untracked += 1;
            }
        }
    }

    repo.status = if repo.modified + repo.staged + repo.untracked > 0 {
        RepoStatus::Dirty
    } else if repo.ahead > 0 && repo.behind > 0 {
        RepoStatus::Diverged
    } else if repo.ahead > 0 {
        RepoStatus::Ahead
    } else if repo.behind > 0 {
        RepoStatus::Behind
    } else {
        RepoStatus::Clean
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_repos_discovers_git_dirs() {
        let base = std::env::temp_dir().join(format!("gitp_findrepos_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("alpha/.git")).unwrap();
        fs::create_dir_all(base.join("beta/.git")).unwrap();
        fs::create_dir_all(base.join("not_a_repo")).unwrap();

        let repos = find_repos(base.to_str().unwrap()).unwrap();
        let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]); // sorted
        assert!(!names.contains(&"not_a_repo"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn find_repos_empty_dir() {
        let base = std::env::temp_dir().join(format!("gitp_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let repos = find_repos(base.to_str().unwrap()).unwrap();
        assert!(repos.is_empty());

        fs::remove_dir_all(&base).unwrap();
    }
}
