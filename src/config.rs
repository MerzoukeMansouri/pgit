use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

fn config_path() -> Option<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(config_dir.join("pgit").join("paths.json"))
}

/// Loads the saved list of watched paths, if any config exists.
pub fn load() -> Option<Vec<String>> {
    let data = fs::read_to_string(config_path()?).ok()?;
    serde_json::from_str(&data).ok()
}

/// Persists the given paths so future launches without args reuse them.
pub fn save(paths: &[String]) -> Result<()> {
    let path = config_path().context("could not determine config directory (no HOME)")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(paths)?;
    fs::write(path, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trips_paths_through_json() {
        let paths = vec!["/a/b".to_string(), "/c/d".to_string()];
        let json = serde_json::to_string(&paths).unwrap();
        let back: Vec<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(paths, back);
    }
}
