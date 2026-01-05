use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::{Result, Context};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DbState {
    pub databases: HashSet<String>,
    pub tables: HashSet<String>, // Format: "database_name.table_name"
}

impl DbState {
    pub fn load(path: &str) -> Result<Self> {
        if fs::metadata(path).is_err() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .context(format!("Failed to read state file: {}", path))?;
        let state: DbState = serde_yaml::from_str(&content)
            .context("Failed to parse state YAML")?;
        Ok(state)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_yaml::to_string(self)
            .context("Failed to serialize state")?;
        fs::write(path, content)
            .context(format!("Failed to write state file: {}", path))?;
        Ok(())
    }

    pub fn add_database(&mut self, name: String) {
        self.databases.insert(name);
    }

    pub fn add_table(&mut self, database: String, table: String) {
        self.tables.insert(format!("{}.{}", database, table));
    }

    pub fn has_database(&self, name: &str) -> bool {
        self.databases.contains(name)
    }

    pub fn has_table(&self, database: &str, table: &str) -> bool {
        self.tables.contains(&format!("{}.{}", database, table))
    }
}
