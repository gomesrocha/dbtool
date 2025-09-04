use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;
use sqlx::{postgres::PgPoolOptions, Row};
use std::fs;

const DB_URL: &str = "postgres://postgres:postgres@127.0.0.1:5432/postgres";

#[test]
fn test_init_command() -> Result<()> {
    let playbook_path = "test_playbook_init.yml";
    let mut cmd = Command::cargo_bin("dbtool")?;
    cmd.arg("init").arg("--playbook").arg(playbook_path);
    cmd.assert().success();

    let content = fs::read_to_string(playbook_path)?;
    assert!(content.contains("databases: []"));
    assert!(content.contains("tables: []"));

    fs::remove_file(playbook_path)?;

    Ok(())
}

#[tokio::test]
async fn test_apply_and_destroy_table() -> Result<()> {
    let table_name = "test_table_apply_destroy";
    let playbook_path = format!("{}.yml", table_name);
    let sql_path = format!("create_{}.sql", table_name);

    let playbook_content = format!(
        r#"
databases: []
tables:
  - database: postgres
    name: {}
    if_not_exists: {}
"#,
        table_name, sql_path
    );

    fs::write(&playbook_path, playbook_content)?;
    fs::write(
        &sql_path,
        format!("CREATE TABLE {} (id SERIAL PRIMARY KEY);", table_name),
    )?;

    // Apply
    let mut cmd_apply = Command::cargo_bin("dbtool")?;
    cmd_apply
        .arg("apply")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(DB_URL);
    cmd_apply.assert().success();

    // Verify creation
    let pool = PgPoolOptions::new().connect(DB_URL).await?;
    let row = sqlx::query(
        "SELECT 1 FROM information_schema.tables WHERE table_name = $1",
    )
    .bind(table_name)
    .fetch_optional(&pool)
    .await?;
    assert!(row.is_some());

    // Destroy
    let mut cmd_destroy = Command::cargo_bin("dbtool")?;
    cmd_destroy
        .arg("destroy")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(DB_URL);
    cmd_destroy.assert().success();

    // Verify destruction
    let row = sqlx::query(
        "SELECT 1 FROM information_schema.tables WHERE table_name = $1",
    )
    .bind(table_name)
    .fetch_optional(&pool)
    .await?;
    assert!(row.is_none());

    fs::remove_file(&playbook_path)?;
    fs::remove_file(&sql_path)?;

    Ok(())
}

#[tokio::test]
async fn test_apply_and_destroy_database() -> Result<()> {
    let db_name = "test_db_apply_destroy";
    let playbook_path = format!("{}.yml", db_name);
    let sql_path = format!("create_{}.sql", db_name);

    let playbook_content = format!(
        r#"
databases:
  - name: {}
    if_not_exists: {}
tables: []
"#,
        db_name, sql_path
    );

    fs::write(&playbook_path, playbook_content)?;
    fs::write(&sql_path, format!("CREATE DATABASE {};", db_name))?;

    // Plan
    let mut cmd_plan = Command::cargo_bin("dbtool")?;
    cmd_plan
        .arg("plan")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(DB_URL);
    cmd_plan.assert().success().stdout(contains(
        &format!("[PLAN] Would create database {}", db_name),
    ));

    // Apply
    let mut cmd_apply = Command::cargo_bin("dbtool")?;
    cmd_apply
        .arg("apply")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(DB_URL);
    cmd_apply.assert().success();

    // Verify creation
    let pool = PgPoolOptions::new().connect(DB_URL).await?;
    let row = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(db_name)
        .fetch_optional(&pool)
        .await?;
    assert!(row.is_some());

    // Destroy
    let mut cmd_destroy = Command::cargo_bin("dbtool")?;
    cmd_destroy
        .arg("destroy")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(DB_URL);
    cmd_destroy.assert().success();

    // Verify destruction
    let row = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(db_name)
        .fetch_optional(&pool)
        .await?;
    assert!(row.is_none());

    fs::remove_file(&playbook_path)?;
    fs::remove_file(&sql_path)?;

    Ok(())
}

#[test]
fn test_validate_command_success() -> Result<()> {
    let playbook_path = "test_playbook_validate_success.yml";
    let sql_path = "test_create_db.sql";

    let playbook_content = format!(
        r#"
databases:
  - name: test_db
    if_not_exists: {}
tables: []
"#,
        sql_path
    );

    fs::write(playbook_path, playbook_content)?;
    fs::write(sql_path, "CREATE DATABASE test_db;")?;

    let mut cmd = Command::cargo_bin("dbtool")?;
    cmd.arg("validate").arg("--playbook").arg(playbook_path);
    cmd.assert().success();

    fs::remove_file(playbook_path)?;
    fs::remove_file(sql_path)?;

    Ok(())
}

#[test]
fn test_validate_command_missing_sql_file() -> Result<()> {
    let playbook_path = "test_playbook_validate_missing_sql.yml";
    let sql_path = "non_existent.sql";

    let playbook_content = format!(
        r#"
databases:
  - name: test_db
    if_not_exists: {}
tables: []
"#,
        sql_path
    );

    fs::write(playbook_path, playbook_content)?;

    let mut cmd = Command::cargo_bin("dbtool")?;
    cmd.arg("validate").arg("--playbook").arg(playbook_path);
    cmd.assert().failure();

    fs::remove_file(playbook_path)?;

    Ok(())
}
