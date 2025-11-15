use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{self, Error as SerdeError};

use super::models::CountdownPersistedState;

pub fn load_snapshot(path: &Path) -> Result<CountdownPersistedState> {
    if !path.exists() {
        return Ok(CountdownPersistedState::default());
    }

    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read countdowns from {}", path.display()))?;
    let snapshot = serde_json::from_str(&data).map_err(|err| map_deser_error(err, path))?;
    Ok(snapshot)
}

pub fn save_snapshot(path: &Path, snapshot: &CountdownPersistedState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create dir {}", parent.display()))?;
    }

    let data = serde_json::to_string_pretty(snapshot)?;
    fs::write(path, data)
        .with_context(|| format!("failed to write countdowns to {}", path.display()))?;
    Ok(())
}

fn map_deser_error(err: SerdeError, path: &Path) -> anyhow::Error {
    anyhow::Error::new(err).context(format!(
        "failed to deserialize countdowns from {}",
        path.display()
    ))
}
