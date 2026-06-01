use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub enum RepoStatus {
    Clean,
    Dirty,
    Ahead,
    Behind,
    Diverged,
}

#[derive(Clone)]
pub struct GitRepo {
    pub name: String,
    pub path: PathBuf,
    pub status: RepoStatus,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
}

#[derive(Clone)]
pub struct CiRun {
    pub repo: String,
    pub repo_path: std::path::PathBuf,
    pub id: u64,
    pub workflow: String,
    pub status: String,
    pub conclusion: String,
    pub branch: String,
    pub event: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct PrItem {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub branch: String,
    pub url: String,
}
