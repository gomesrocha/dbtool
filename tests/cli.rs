use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;
use sqlx::{mysql::MySqlPoolOptions, postgres::PgPoolOptions, Row};
use std::fs;

const PG_DB_URL: &str = "postgres://postgres:postgres@127.0.0.1:5432/postgres";
const MYSQL_DB_URL: &str = "mysql://root:root@127.0.0.1:3306/mysql";

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
async fn test_apply_and_destroy_table_postgres() -> Result<()> {
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
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
    cmd_apply.assert().success();

    // Verify creation
    let pool = PgPoolOptions::new().connect(PG_DB_URL).await?;
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
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
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
async fn test_apply_and_destroy_table_mysql() -> Result<()> {
    let table_name = "test_table_apply_destroy_mysql";
    let playbook_path = format!("{}.yml", table_name);
    let sql_path = format!("create_{}.sql", table_name);
    let playbook_content = format!(
        r#"
databases: []
tables:
  - database: mysql
    name: {}
    if_not_exists: {}
"#,
        table_name, sql_path
    );
    fs::write(&playbook_path, playbook_content)?;
    fs::write(
        &sql_path,
        format!("CREATE TABLE {} (id INT AUTO_INCREMENT PRIMARY KEY);", table_name),
    )?;

    // Apply
    let mut cmd_apply = Command::cargo_bin("dbtool")?;
    cmd_apply
        .arg("apply")
        .arg("--playbook")
        .arg(&playbook_path)
        .arg("--db-url")
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_apply.assert().success();

    // Verify creation
    let pool = MySqlPoolOptions::new().connect(MYSQL_DB_URL).await?;
    let row = sqlx::query(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
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
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_destroy.assert().success();

    // Verify destruction
    let row = sqlx::query(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
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
async fn test_apply_and_destroy_database_postgres() -> Result<()> {
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
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
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
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
    cmd_apply.assert().success();

    // Verify creation
    let pool = PgPoolOptions::new().connect(PG_DB_URL).await?;
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
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
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

#[tokio::test]
async fn test_apply_and_destroy_database_mysql() -> Result<()> {
    let db_name = "test_db_apply_destroy_mysql";
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
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
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
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_apply.assert().success();

    // Verify creation
    let pool = MySqlPoolOptions::new().connect(MYSQL_DB_URL).await?;
    let row = sqlx::query("SELECT 1 FROM information_schema.schemata WHERE schema_name = ?")
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
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_destroy.assert().success();

    // Verify destruction
    let row = sqlx::query("SELECT 1 FROM information_schema.schemata WHERE schema_name = ?")
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

#[tokio::test]
async fn test_status_command_postgres() -> Result<()> {
    let db_name = "test_db_status";
    let table_name = "test_table_status";
    let playbook_path = "test_playbook_status.yml";
    let sql_db_path = "create_test_db.sql";
    let sql_table_path = "create_test_table.sql";

    let playbook_content = format!(
        r#"
databases:
  - name: {}
    if_not_exists: {}
tables:
  - database: {}
    name: {}
    if_not_exists: {}
"#,
        db_name, sql_db_path, db_name, table_name, sql_table_path
    );

    fs::write(playbook_path, &playbook_content)?;
    fs::write(sql_db_path, format!("CREATE DATABASE {};", db_name))?;
    fs::write(sql_table_path, format!("CREATE TABLE {} (id SERIAL PRIMARY KEY);", table_name))?;

    // Apply to create database and table
    let mut cmd_apply = Command::cargo_bin("dbtool")?;
    cmd_apply
        .arg("apply")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
    cmd_apply.assert().success();

    // Run status command
    let mut cmd_status = Command::cargo_bin("dbtool")?;
    cmd_status
        .arg("status")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
    cmd_status
        .assert()
        .success()
        .stdout(contains(format!("Database {}: Exists", db_name)))
        .stdout(contains(format!("Table {}.{} in schema 'public': Exists", db_name, table_name)));

    // Clean up
    let mut cmd_destroy = Command::cargo_bin("dbtool")?;
    cmd_destroy
        .arg("destroy")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(PG_DB_URL)
        .arg("--db-type")
        .arg("postgres");
    cmd_destroy.assert().success();

    fs::remove_file(playbook_path)?;
    fs::remove_file(sql_db_path)?;
    fs::remove_file(sql_table_path)?;
    Ok(())
}

#[tokio::test]
async fn test_status_command_mysql() -> Result<()> {
    let db_name = "test_db_status_mysql";
    let table_name = "test_table_status_mysql";
    let playbook_path = "test_playbook_status_mysql.yml";
    let sql_db_path = "create_test_db_mysql.sql";
    let sql_table_path = "create_test_table_mysql.sql";

    let playbook_content = format!(
        r#"
databases:
  - name: {}
    if_not_exists: {}
tables:
  - database: {}
    name: {}
    if_not_exists: {}
"#,
        db_name, sql_db_path, db_name, table_name, sql_table_path
    );

    fs::write(playbook_path, &playbook_content)?;
    fs::write(sql_db_path, format!("CREATE DATABASE {};", db_name))?;
    fs::write(sql_table_path, format!("CREATE TABLE {} (id INT AUTO_INCREMENT PRIMARY KEY);", table_name))?;

    // Apply to create database and table
    let mut cmd_apply = Command::cargo_bin("dbtool")?;
    cmd_apply
        .arg("apply")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_apply.assert().success();

    // Run status command
    let mut cmd_status = Command::cargo_bin("dbtool")?;
    cmd_status
        .arg("status")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_status
        .assert()
        .success()
        .stdout(contains(format!("Database {}: Exists", db_name)))
        .stdout(contains(format!("Table {}.{} in schema 'public': Exists", db_name, table_name)));

    // Clean up
    let mut cmd_destroy = Command::cargo_bin("dbtool")?;
    cmd_destroy
        .arg("destroy")
        .arg("--playbook")
        .arg(playbook_path)
        .arg("--db-url")
        .arg(MYSQL_DB_URL)
        .arg("--db-type")
        .arg("mysql");
    cmd_destroy.assert().success();

    fs::remove_file(playbook_path)?;
    fs::remove_file(sql_db_path)?;
    fs::remove_file(sql_table_path)?;
    Ok(())
}