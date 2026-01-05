use anyhow::{Context, Result};
use sqlx::{mysql::MySqlPoolOptions, postgres::PgPoolOptions, MySql, Pool, Postgres, Transaction};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{error, info, warn};
use std::io::{self, Write};
use crate::cli::DbType;
use crate::playbook::{Playbook, validate_playbook};
use crate::tasks::{Task, TaskStatus};
use crate::state::DbState;

pub enum DbPool {
    Postgres(Pool<Postgres>),
    MySQL(Pool<MySql>),
}

pub enum DbTransaction<'a> {
    Postgres(Transaction<'a, Postgres>),
    MySQL(Transaction<'a, MySql>),
}

pub async fn apply_playbook(playbook_path: &str, db_url: &str, dry_run: bool, rollback: bool, db_type: DbType, auto_approve: bool) -> Result<()> {
    info!("Reading playbook from: {}", playbook_path);
    let playbook_content = std::fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    info!("Playbook content:\n{}", playbook_content);
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    let state_path = "dbtools.dbstate";
    let mut state = DbState::load(state_path)?;

    // Filter items not in state
    let pending_databases: Vec<&crate::playbook::Database> = playbook.databases.iter()
        .filter(|db| !state.has_database(&db.name))
        .collect();

    let pending_tables: Vec<&crate::playbook::Table> = playbook.tables.iter()
        .filter(|table| !state.has_table(&table.database, &table.name))
        .collect();

    if pending_databases.is_empty() && pending_tables.is_empty() {
        info!("No changes needed (state matches playbook).");
        return Ok(());
    }

    info!("Execution Plan (Changes to be applied):");
    for db in &pending_databases {
        info!("  + Database: {}", db.name);
    }
    for table in &pending_tables {
        info!("  + Table: {}.{}", table.database, table.name);
    }

    if dry_run {
        return Ok(());
    }

    // Interactive confirmation
    if !auto_approve {
        print!("Do you want to proceed? (yes/no): ");
        io::stdout().flush().context("Failed to flush stdout")?;
        let mut input = String::new();
        io::stdin().read_line(&mut input).context("Failed to read stdin")?;
        let trimmed = input.trim().to_lowercase();
        if trimmed != "yes" && trimmed != "y" {
            info!("Operation cancelled by user.");
            return Ok(());
        }
    }

    info!("Using database URL: {}", db_url);
    let pool = match db_type {
        DbType::Postgres => DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to PostgreSQL database")?,
        ),
        DbType::MySQL => DbPool::MySQL(
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to MySQL database")?,
        ),
    };

    let mut tasks: Vec<Task> = Vec::new();

    // Process databases
    for db in pending_databases {
        let task_name = format!("check_database_{}", db.name);
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_database_exists_postgres(pg_pool, &db.name).await?,
            DbPool::MySQL(mysql_pool) => check_database_exists_mysql(mysql_pool, &db.name).await?,
        };

        if exists {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Database {} already exists, updating state", db.name);
            state.add_database(db.name.clone());
            state.save(state_path)?;
            continue;
        }

        match execute_sql_file(&pool, &db.if_not_exists).await {
            Ok(_) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                info!("Created database {}", db.name);
                state.add_database(db.name.clone());
                state.save(state_path)?;
            }
            Err(e) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                error!("Failed to create database {}: {}. Rollback not supported for databases.", db.name, e);
            }
        }
    }

    // Process tables (with transaction for rollback)
    let mut tx = if rollback {
        match &pool {
            DbPool::Postgres(pg_pool) => Some(DbTransaction::Postgres(
                pg_pool.begin().await.context("Failed to start PostgreSQL transaction")?,
            )),
            DbPool::MySQL(mysql_pool) => Some(DbTransaction::MySQL(
                mysql_pool.begin().await.context("Failed to start MySQL transaction")?,
            )),
        }
    } else {
        None
    };

    for table in pending_tables {
        let task_name = format!("check_table_{}_{}", table.database, table.name);
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_table_exists_postgres(pg_pool, &table.name).await?,
            DbPool::MySQL(mysql_pool) => check_table_exists_mysql(mysql_pool, &table.name).await?,
        };

        if exists {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Table {}.{} already exists, updating state", table.database, table.name);
            state.add_table(table.database.clone(), table.name.clone());
            state.save(state_path)?;
            continue;
        }

        match execute_sql_file(&pool, &table.if_not_exists).await {
            Ok(_) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                info!("Created table {}.{}", table.database, table.name);
                state.add_table(table.database.clone(), table.name.clone());
                state.save(state_path)?;
            }
            Err(e) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                error!("Failed to create table {}.{}: {}", table.database, table.name, e);
                if let Some(tx_inner) = tx.take() {
                    warn!("Rolling back transaction due to error");
                    match tx_inner {
                        DbTransaction::Postgres(pg_tx) => pg_tx.rollback().await.context("Failed to rollback PostgreSQL transaction")?,
                        DbTransaction::MySQL(mysql_tx) => mysql_tx.rollback().await.context("Failed to rollback MySQL transaction")?,
                    }
                }
                return Err(e);
            }
        }
    }

    // Commit transaction if no errors
    if let Some(tx_inner) = tx {
        match tx_inner {
            DbTransaction::Postgres(pg_tx) => pg_tx.commit().await.context("Failed to commit PostgreSQL transaction")?,
            DbTransaction::MySQL(mysql_tx) => mysql_tx.commit().await.context("Failed to commit MySQL transaction")?,
        }
    }

    // Print task summary
    info!("Task Summary:");
    for task in &tasks {
        match &task.status {
            TaskStatus::Success => info!("Task {}: Success", task.name),
            TaskStatus::Failed(err) => error!("Task {}: Failed - {}", task.name, err),
            TaskStatus::Skipped => info!("Task {}: Skipped", task.name),
        }
    }

    Ok(())
}

pub async fn test_playbook(playbook_path: &str, db_url: &str, db_type: DbType) -> Result<()> {
    info!("Testing playbook: {}", playbook_path);
    validate_playbook(playbook_path).await?;
    let pool = match db_type {
        DbType::Postgres => DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to PostgreSQL database")?,
        ),
        DbType::MySQL => DbPool::MySQL(
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to MySQL database")?,
        ),
    };
    info!("Database connection successful");

    let playbook_content = std::fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    for db in &playbook.databases {
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_database_exists_postgres(pg_pool, &db.name).await?,
            DbPool::MySQL(mysql_pool) => check_database_exists_mysql(mysql_pool, &db.name).await?,
        };
        info!("Database {} exists: {}", db.name, exists);
    }
    for table in &playbook.tables {
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_table_exists_postgres(pg_pool, &table.name).await?,
            DbPool::MySQL(mysql_pool) => check_table_exists_mysql(mysql_pool, &table.name).await?,
        };
        info!("Table {}.{} exists: {}", table.database, table.name, exists);
    }
    Ok(())
}

pub async fn status(playbook_path: &str, db_url: &str, db_type: DbType) -> Result<()> {
    info!("Checking status for playbook: {}", playbook_path);
    let playbook_content = std::fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    let pool = match db_type {
        DbType::Postgres => DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to PostgreSQL database")?,
        ),
        DbType::MySQL => DbPool::MySQL(
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to MySQL database")?,
        ),
    };

    info!("Database Status:");
    for db in &playbook.databases {
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_database_exists_postgres(pg_pool, &db.name).await?,
            DbPool::MySQL(mysql_pool) => check_database_exists_mysql(mysql_pool, &db.name).await?,
        };
        if exists {
            info!("- Database {}: Exists", db.name);
        } else {
            info!("- Database {}: Missing", db.name);
        }
    }

    info!("Table Status:");
    for table in &playbook.tables {
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_table_exists_postgres(pg_pool, &table.name).await?,
            DbPool::MySQL(mysql_pool) => check_table_exists_mysql(mysql_pool, &table.name).await?,
        };
        if exists {
            info!("- Table {}.{}: Exists", table.database, table.name);
        } else {
            info!("- Table {}.{}: Missing", table.database, table.name);
        }
    }

    Ok(())
}

pub async fn destroy_playbook(playbook_path: &str, db_url: &str, db_type: DbType) -> Result<()> {
    info!("Destroying resources from playbook: {}", playbook_path);
    let playbook_content = std::fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    let pool = match db_type {
        DbType::Postgres => DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to PostgreSQL database")?,
        ),
        DbType::MySQL => DbPool::MySQL(
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
                .context("Failed to connect to MySQL database")?,
        ),
    };

    let mut tasks: Vec<Task> = Vec::new();

    // Destroy tables
    for table in playbook.tables.iter().rev() {
        let task_name = format!("drop_table_{}_{}", table.database, table.name);
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_table_exists_postgres(pg_pool, &table.name).await?,
            DbPool::MySQL(mysql_pool) => check_table_exists_mysql(mysql_pool, &table.name).await?,
        };
        if exists {
            let query = match db_type {
                DbType::Postgres => format!("DROP TABLE IF EXISTS public.{} CASCADE", table.name),
                DbType::MySQL => format!("DROP TABLE IF EXISTS {}", table.name),
            };
            match &pool {
                DbPool::Postgres(pg_pool) => {
                    sqlx::query(&query)
                        .execute(pg_pool)
                        .await
                        .context(format!("Failed to drop table {}.{}", table.database, table.name))?;
                }
                DbPool::MySQL(mysql_pool) => {
                    sqlx::query(&query)
                        .execute(mysql_pool)
                        .await
                        .context(format!("Failed to drop table {}.{}", table.database, table.name))?;
                }
            }
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
            info!("Dropped table {}.{}", table.database, table.name);
        } else {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Table {}.{} does not exist, skipping", table.database, table.name);
        }
    }

    // Destroy databases
    for db in playbook.databases.iter().rev() {
        let task_name = format!("drop_database_{}", db.name);
        let exists = match &pool {
            DbPool::Postgres(pg_pool) => check_database_exists_postgres(pg_pool, &db.name).await?,
            DbPool::MySQL(mysql_pool) => check_database_exists_mysql(mysql_pool, &db.name).await?,
        };
        if exists {
            match &pool {
                DbPool::Postgres(pg_pool) => {
                    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db.name))
                        .execute(pg_pool)
                        .await
                        .context(format!("Failed to drop database {}", db.name))?;
                }
                DbPool::MySQL(mysql_pool) => {
                    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db.name))
                        .execute(mysql_pool)
                        .await
                        .context(format!("Failed to drop database {}", db.name))?;
                }
            }
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
            info!("Dropped database {}", db.name);
        } else {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Database {} does not exist, skipping", db.name);
        }
    }

    // Print task summary
    info!("Task Summary:");
    for task in &tasks {
        match &task.status {
            TaskStatus::Success => info!("Task {}: Success", task.name),
            TaskStatus::Failed(err) => error!("Task {}: Failed - {}", task.name, err),
            TaskStatus::Skipped => info!("Task {}: Skipped", task.name),
        }
    }

    Ok(())
}

async fn check_database_exists_postgres(pool: &Pool<Postgres>, db_name: &str) -> Result<bool> {
    info!("Checking if PostgreSQL database {} exists", db_name);
    let row: (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(db_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check existence of database {}", db_name))?;
    info!("Database {} exists: {}", db_name, row.0);
    Ok(row.0)
}

async fn check_database_exists_mysql(pool: &Pool<MySql>, db_name: &str) -> Result<bool> {
    info!("Checking if MySQL database {} exists", db_name);
    let row: (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = ?)")
        .bind(db_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check existence of database {}", db_name))?;
    info!("Database {} exists: {}", db_name, row.0);
    Ok(row.0)
}

async fn check_table_exists_postgres(pool: &Pool<Postgres>, table_name: &str) -> Result<bool> {
    info!("Checking if PostgreSQL table {} exists in schema 'public'", table_name);
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1)"
    )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check existence of table {}", table_name))?;
    info!("Table {} exists in schema 'public': {}", table_name, row.0);
    Ok(row.0)
}

async fn check_table_exists_mysql(pool: &Pool<MySql>, table_name: &str) -> Result<bool> {
    info!("Checking if MySQL table {} exists", table_name);
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?)"
    )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check existence of table {}", table_name))?;
    info!("Table {} exists: {}", table_name, row.0);
    Ok(row.0)
}

async fn execute_sql_file(pool: &DbPool, file_path: &str) -> Result<()> {
    info!("Attempting to open SQL file: {}", file_path);
    let mut file = File::open(file_path)
        .await
        .context(format!("Failed to open SQL file: {}", file_path))?;
    let mut sql = String::new();
    file.read_to_string(&mut sql)
        .await
        .context(format!("Failed to read SQL file: {}", file_path))?;

    info!("Executing SQL:\n{}", sql);
    match pool {
        DbPool::Postgres(pg_pool) => {
            sqlx::query(&sql)
                .execute(pg_pool)
                .await
                .map_err(|e| anyhow::anyhow!("SQL execution failed: {} (file: {})", e, file_path))?;
        }
        DbPool::MySQL(mysql_pool) => {
            sqlx::query(&sql)
                .execute(mysql_pool)
                .await
                .map_err(|e| anyhow::anyhow!("SQL execution failed: {} (file: {})", e, file_path))?;
        }
    }
    Ok(())
}
