use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::info;

#[derive(Deserialize, Serialize)]
pub struct Playbook {
    pub databases: Vec<Database>,
    pub tables: Vec<Table>,
}

#[derive(Deserialize, Serialize)]
pub struct Database {
    pub name: String,
    pub if_not_exists: String,
}

#[derive(Deserialize, Serialize)]
pub struct Table {
    pub database: String,
    pub name: String,
    pub if_not_exists: String,
}

pub async fn init_playbook(playbook_path: &str) -> Result<()> {
    let playbook_content = r#"---
databases: []
tables: []
"#;
    fs::write(playbook_path, playbook_content)
        .context(format!("Failed to write playbook to: {}", playbook_path))?;
    info!("Created playbook: {}", playbook_path);
    Ok(())
}

pub async fn validate_playbook(playbook_path: &str) -> Result<()> {
    info!("Validating playbook: {}", playbook_path);
    let playbook_content = fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook =
        serde_yaml::from_str(&playbook_content).context("Failed to parse playbook YAML")?;

    // Check if SQL files exist
    for db in &playbook.databases {
        if !fs::metadata(&db.if_not_exists).is_ok() {
            return Err(anyhow::anyhow!("SQL file not found: {}", db.if_not_exists));
        }
    }
    for table in &playbook.tables {
        if !fs::metadata(&table.if_not_exists).is_ok() {
            return Err(anyhow::anyhow!(
                "SQL file not found: {}",
                table.if_not_exists
            ));
        }
    }

    info!("Playbook validated successfully");
    Ok(())
}
