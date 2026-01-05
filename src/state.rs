use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;

// Legacy state for backward compatibility
#[derive(Serialize, Deserialize, Debug)]
pub struct LegacyDbState {
    pub databases: HashSet<String>,
    pub tables: HashSet<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DbState {
    // Key: name, Value: hash
    pub databases: HashMap<String, String>,
    // Key: "database.table", Value: hash
    pub tables: HashMap<String, String>,
}

impl DbState {
    pub fn load(path: &str) -> Result<Self> {
        if fs::metadata(path).is_err() {
            return Ok(Self::default());
        }
        let content =
            fs::read_to_string(path).context(format!("Failed to read state file: {}", path))?;

        // Try parsing as new format
        if let Ok(state) = serde_yaml::from_str::<DbState>(&content) {
            return Ok(state);
        }

        // Try parsing as legacy format
        let legacy: LegacyDbState = serde_yaml::from_str(&content)
            .context("Failed to parse state YAML (tried both new and legacy formats)")?;

        // Convert legacy to new
        let mut databases = HashMap::new();
        for db in legacy.databases {
            databases.insert(db, String::new()); // Empty hash for unknown
        }
        let mut tables = HashMap::new();
        for table in legacy.tables {
            tables.insert(table, String::new());
        }

        Ok(DbState { databases, tables })
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize state")?;
        fs::write(path, content).context(format!("Failed to write state file: {}", path))?;
        Ok(())
    }

    pub fn add_database(&mut self, name: String, hash: String) {
        self.databases.insert(name, hash);
    }

    pub fn add_table(&mut self, database: String, table: String, hash: String) {
        self.tables.insert(format!("{}.{}", database, table), hash);
    }

    pub fn get_database_hash(&self, name: &str) -> Option<&String> {
        self.databases.get(name)
    }

    pub fn get_table_hash(&self, database: &str, table: &str) -> Option<&String> {
        self.tables.get(&format!("{}.{}", database, table))
    }

    pub fn has_database(&self, name: &str) -> bool {
        self.databases.contains_key(name)
    }

    pub fn has_table(&self, database: &str, table: &str) -> bool {
        self.tables.contains_key(&format!("{}.{}", database, table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_legacy_migration() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "databases:\n  - db1\ntables:\n  - db1.table1")?;
        let path = file.path().to_str().unwrap();

        let state = DbState::load(path)?;

        assert!(state.has_database("db1"));
        assert!(state.has_table("db1", "table1"));
        assert!(state.get_database_hash("db1").unwrap().is_empty());

        Ok(())
    }

    #[test]
    fn test_new_format() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        let content = r#"
databases:
  db1: "hash1"
tables:
  db1.table1: "hash2"
"#;
        writeln!(file, "{}", content)?;
        let path = file.path().to_str().unwrap();

        let state = DbState::load(path)?;

        assert!(state.has_database("db1"));
        assert!(state.has_table("db1", "table1"));
        assert_eq!(state.get_database_hash("db1").unwrap(), "hash1");
        assert_eq!(state.get_table_hash("db1", "table1").unwrap(), "hash2");

        Ok(())
    }
}
