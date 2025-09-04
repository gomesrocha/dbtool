use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Transaction};
use std::fs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{error, info, warn};

#[derive(Parser)]
#[clap(name = "dbtool", about = "A database schema management tool")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Apply {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, default_value = "false")]
        no_rollback: bool,
    },
    Plan {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
    },
    Init {
        #[clap(long)]
        playbook: String,
    },
    Test {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
    },
    Destroy {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
    },
    Validate {
        #[clap(long)]
        playbook: String,
    },
}

#[derive(Deserialize)]
struct Playbook {
    databases: Vec<Database>,
    tables: Vec<Table>,
}

#[derive(Deserialize)]
struct Database {
    name: String,
    if_not_exists: String,
}

#[derive(Deserialize)]
struct Table {
    database: String,
    name: String,
    if_not_exists: String,
}

#[derive(Debug)]
struct Task {
    name: String,
    status: TaskStatus,
}

#[derive(Debug)]
enum TaskStatus {
    Success,
    Failed(String),
    Skipped,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Apply { playbook, db_url, no_rollback } => {
            apply_playbook(&playbook, &db_url, false, !no_rollback).await
        }
        Commands::Plan { playbook, db_url } => {
            apply_playbook(&playbook, &db_url, true, false).await
        }
        Commands::Init { playbook } => {
            init_playbook(&playbook).await
        }
        Commands::Test { playbook, db_url } => {
            test_playbook(&playbook, &db_url).await
        }
        Commands::Destroy { playbook, db_url } => {
            destroy_playbook(&playbook, &db_url).await
        }
        Commands::Validate { playbook } => {
            validate_playbook(&playbook).await
        }
    }
}

async fn init_playbook(playbook_path: &str) -> Result<()> {
    let playbook_content = r#"---
databases: []
tables: []
"#;
    fs::write(playbook_path, playbook_content)
        .context(format!("Failed to write playbook to: {}", playbook_path))?;
    info!("Created playbook: {}", playbook_path);
    Ok(())
}

async fn apply_playbook(playbook_path: &str, db_url: &str, dry_run: bool, rollback: bool) -> Result<()> {
    info!("Reading playbook from: {}", playbook_path);
    let playbook_content = fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    info!("Playbook content:\n{}", playbook_content);
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    info!("Using database URL: {}", db_url);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .context("Failed to connect to database")?;

    let mut tasks: Vec<Task> = Vec::new();

    // Process databases (no transaction for databases, as CREATE DATABASE can't be rolled back)
    for db in &playbook.databases {
        let task_name = format!("check_database_{}", db.name);
        let exists = check_database_exists(&pool, &db.name).await?;

        if exists {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Database {} already exists, skipping", db.name);
            continue;
        }

        if dry_run {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
            info!("[PLAN] Would create database {}", db.name);
            continue;
        }

        match execute_sql_file(&pool, &db.if_not_exists).await {
            Ok(_) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                info!("Created database {}", db.name);
            }
            Err(e) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                error!("Failed to create database {}: {}. Rollback not supported for databases.", db.name, e);
            }
        }
    }

    // Process tables (with transaction for rollback)
    let mut tx: Option<Transaction<'_, Postgres>> = if rollback && !dry_run {
        Some(pool.begin().await.context("Failed to start transaction")?)
    } else {
        None
    };

    for table in &playbook.tables {
        let task_name = format!("check_table_{}_{}", table.database, table.name);
        let exists = check_table_exists(&pool, &table.name).await?;

        if exists {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Table {}.{} already exists in schema 'public', skipping", table.database, table.name);
            continue;
        }

        if dry_run {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
            info!("[PLAN] Would create table {}.{} in schema 'public'", table.database, table.name);
            continue;
        }

        match execute_sql_file(&pool, &table.if_not_exists).await {
            Ok(_) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                info!("Created table {}.{} in schema 'public'", table.database, table.name);
            }
            Err(e) => {
                tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                error!("Failed to create table {}.{}: {}", table.database, table.name, e);
                if let Some(tx_inner) = tx.take() {
                    warn!("Rolling back transaction due to error");
                    tx_inner.rollback().await.context("Failed to rollback transaction")?;
                }
                return Err(e);
            }
        }
    }

    // Commit transaction if no errors
    if let Some(tx_inner) = tx {
        tx_inner.commit().await.context("Failed to commit transaction")?;
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

async fn test_playbook(playbook_path: &str, db_url: &str) -> Result<()> {
    info!("Testing playbook: {}", playbook_path);
    validate_playbook(playbook_path).await?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .context("Failed to connect to database during test")?;
    info!("Database connection successful");
    // Test database existence
    let playbook_content = fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;
    for db in &playbook.databases {
        let exists = check_database_exists(&pool, &db.name).await?;
        info!("Database {} exists: {}", db.name, exists);
    }
    for table in &playbook.tables {
        let exists = check_table_exists(&pool, &table.name).await?;
        info!("Table {}.{} exists in schema 'public': {}", table.database, table.name, exists);
    }
    Ok(())
}

async fn destroy_playbook(playbook_path: &str, db_url: &str) -> Result<()> {
    info!("Destroying resources from playbook: {}", playbook_path);
    let playbook_content = fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .context("Failed to connect to database")?;

    let mut tasks: Vec<Task> = Vec::new();

    // Destroy tables
    for table in playbook.tables.iter().rev() {
        let task_name = format!("drop_table_{}_{}", table.database, table.name);
        let exists = check_table_exists(&pool, &table.name).await?;
        if exists {
            match sqlx::query(&format!("DROP TABLE IF EXISTS public.{} CASCADE", table.name))
                .execute(&pool)
                .await
                .context(format!("Failed to drop table {}.{}", table.database, table.name))
            {
                Ok(_) => {
                    tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                    info!("Dropped table {}.{} in schema 'public'", table.database, table.name);
                }
                Err(e) => {
                    tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                    error!("Failed to drop table {}.{}: {}", table.database, table.name, e);
                }
            }
        } else {
            tasks.push(Task { name: task_name.clone(), status: TaskStatus::Skipped });
            info!("Table {}.{} does not exist, skipping", table.database, table.name);
        }
    }

    // Destroy databases (careful, drops entire DB)
    for db in playbook.databases.iter().rev() {
        let task_name = format!("drop_database_{}", db.name);
        let exists = check_database_exists(&pool, &db.name).await?;
        if exists {
            match sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db.name))
                .execute(&pool)
                .await
                .context(format!("Failed to drop database {}", db.name))
            {
                Ok(_) => {
                    tasks.push(Task { name: task_name.clone(), status: TaskStatus::Success });
                    info!("Dropped database {}", db.name);
                }
                Err(e) => {
                    tasks.push(Task { name: task_name.clone(), status: TaskStatus::Failed(e.to_string()) });
                    error!("Failed to drop database {}: {}", db.name, e);
                }
            }
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

async fn validate_playbook(playbook_path: &str) -> Result<()> {
    info!("Validating playbook: {}", playbook_path);
    let playbook_content = fs::read_to_string(playbook_path)
        .context(format!("Failed to read playbook: {}", playbook_path))?;
    let playbook: Playbook = serde_yaml::from_str(&playbook_content)
        .context("Failed to parse playbook YAML")?;

    // Check if SQL files exist
    for db in &playbook.databases {
        if !fs::metadata(&db.if_not_exists).is_ok() {
            return Err(anyhow::anyhow!("SQL file not found: {}", db.if_not_exists));
        }
    }
    for table in &playbook.tables {
        if !fs::metadata(&table.if_not_exists).is_ok() {
            return Err(anyhow::anyhow!("SQL file not found: {}", table.if_not_exists));
        }
    }

    info!("Playbook validated successfully");
    Ok(())
}

async fn check_database_exists(pool: &Pool<Postgres>, db_name: &str) -> Result<bool> {
    info!("Checking if database {} exists", db_name);
    let row: (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(db_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check existence of database {}", db_name))?;
    info!("Database {} exists: {}", db_name, row.0);
    Ok(row.0)
}

async fn check_table_exists(pool: &Pool<Postgres>, table_name: &str) -> Result<bool> {
    info!("Checking if table {} exists in schema 'public'", table_name);
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

async fn execute_sql_file(pool: &Pool<Postgres>, file_path: &str) -> Result<()> {
    info!("Attempting to open SQL file: {}", file_path);
    let mut file = File::open(file_path)
        .await
        .context(format!("Failed to open SQL file: {}", file_path))?;
    let mut sql = String::new();
    file.read_to_string(&mut sql)
        .await
        .context(format!("Failed to read SQL file: {}", file_path))?;

    info!("Executing SQL:\n{}", sql);
    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("SQL execution failed: {} (file: {})", e, file_path))?;
    Ok(())
}