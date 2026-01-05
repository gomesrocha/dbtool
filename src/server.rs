use crate::cli::DbType;
use crate::database::{apply_playbook, DbPool};
use crate::playbook::{Database, Playbook, Table};
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::fs;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub playbook_path: String,
    pub db_url: String,
    pub db_type: DbType,
    pub pool: Arc<DbPool>,
    pub auth: Arc<(String, String)>, // username, password
}

#[derive(Deserialize)]
struct Demand {
    kind: String, // "database" or "table"
    name: String,
    database: Option<String>, // required for table
    sql_content: String,
}

#[derive(Serialize)]
struct StatusResponse {
    databases: Vec<StatusItem>,
    tables: Vec<StatusItem>,
}

#[derive(Serialize)]
struct StatusItem {
    name: String,
    exists: bool,
    database: Option<String>,
}

pub async fn start_server(
    playbook_path: String,
    db_url: String,
    db_type: DbType,
    port: u16,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    start_server_with_listener(playbook_path, db_url, db_type, listener, username, password).await
}

pub async fn start_server_with_listener(
    playbook_path: String,
    db_url: String,
    db_type: DbType,
    listener: tokio::net::TcpListener,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    // Initialize pool
    let pool = match db_type {
        DbType::Postgres => DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy(&db_url)
                .map_err(|e| anyhow::anyhow!("Failed to connect to Postgres: {}", e))?,
        ),
        DbType::MySQL => DbPool::MySQL(
            MySqlPoolOptions::new()
                .max_connections(5)
                .connect_lazy(&db_url)
                .map_err(|e| anyhow::anyhow!("Failed to connect to MySQL: {}", e))?,
        ),
    };

    let state = AppState {
        playbook_path,
        db_url,
        db_type,
        pool: Arc::new(pool),
        auth: Arc::new((username, password)),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/status", get(get_status))
        .route("/api/apply", post(trigger_apply))
        .route("/api/demand", post(add_demand))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = listener.local_addr()?;
    tracing::info!("Server running on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            if value.starts_with("Basic ") {
                Some(value[6..].to_string())
            } else {
                None
            }
        });

    if let Some(auth_str) = auth_header {
        if let Ok(decoded) = general_purpose::STANDARD.decode(&auth_str) {
            if let Ok(cred_str) = String::from_utf8(decoded) {
                if let Some((u, p)) = cred_str.split_once(':') {
                    if u == state.auth.0 && p == state.auth.1 {
                        return next.run(req).await;
                    }
                }
            }
        }
    }

    // Return 401 with WWW-Authenticate header to trigger browser prompt
    (
        StatusCode::UNAUTHORIZED,
        [(
            axum::http::header::WWW_AUTHENTICATE,
            "Basic realm=\"DBTools\"",
        )],
        "Unauthorized",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_server_startup() {
        // Setup
        let playbook_path = "test_web_server.yml".to_string();
        std::fs::write(&playbook_path, "---\ndatabases: []\ntables: []").unwrap();
        let db_url = "postgres://postgres:password@127.0.0.1:5432/postgres".to_string();
        let db_type = DbType::Postgres;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Spawn server
        tokio::spawn(async move {
            start_server_with_listener(
                playbook_path,
                "postgres://postgres:password@127.0.0.1:5432/postgres".to_string(),
                DbType::Postgres,
                listener,
                "admin".to_string(),
                "password".to_string(),
            )
            .await
            .unwrap();
        });

        // Wait a bit
        tokio::time::sleep(Duration::from_secs(2)).await;

        println!("Sending request to http://127.0.0.1:{}/", port);
        // Use reqwest to hit the endpoint
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test without auth
        let resp = client
            .get(format!("http://127.0.0.1:{}/", port))
            .send()
            .await;
        assert_eq!(resp.unwrap().status(), StatusCode::UNAUTHORIZED);

        // Test with auth
        let resp = client
            .get(format!("http://127.0.0.1:{}/", port))
            .basic_auth("admin", Some("password"))
            .send()
            .await;

        println!("Response: {:?}", resp);

        // Clean up
        std::fs::remove_file("test_web_server.yml").unwrap_or(());

        // We expect it to be OK
        assert_eq!(resp.unwrap().status(), StatusCode::OK);
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

async fn get_status(State(state): State<AppState>) -> Result<Json<StatusResponse>, StatusCode> {
    let playbook_content = fs::read_to_string(&state.playbook_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let playbook: Playbook =
        serde_yaml::from_str(&playbook_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut db_statuses = Vec::new();
    for db in playbook.databases {
        let exists = match state.pool.as_ref() {
            crate::database::DbPool::Postgres(pg_pool) => {
                crate::database::check_database_exists_postgres(pg_pool, &db.name)
                    .await
                    .unwrap_or(false)
            }
            crate::database::DbPool::MySQL(mysql_pool) => {
                crate::database::check_database_exists_mysql(mysql_pool, &db.name)
                    .await
                    .unwrap_or(false)
            }
        };
        db_statuses.push(StatusItem {
            name: db.name,
            exists,
            database: None,
        });
    }

    let mut table_statuses = Vec::new();
    for table in playbook.tables {
        let exists = match state.pool.as_ref() {
            crate::database::DbPool::Postgres(pg_pool) => {
                crate::database::check_table_exists_postgres(pg_pool, &table.name)
                    .await
                    .unwrap_or(false)
            }
            crate::database::DbPool::MySQL(mysql_pool) => {
                crate::database::check_table_exists_mysql(mysql_pool, &table.name)
                    .await
                    .unwrap_or(false)
            }
        };
        table_statuses.push(StatusItem {
            name: table.name,
            exists,
            database: Some(table.database),
        });
    }

    Ok(Json(StatusResponse {
        databases: db_statuses,
        tables: table_statuses,
    }))
}

async fn trigger_apply(State(state): State<AppState>) -> Result<String, StatusCode> {
    // We run apply with auto_approve=true because it's triggered via API
    // Note: apply_playbook currently creates its own pool. Optimization to share pool in apply_playbook is out of scope for now,
    // as apply is infrequent operation.
    apply_playbook(
        &state.playbook_path,
        &state.db_url,
        false,
        true,
        state.db_type,
        true,
    )
    .await
    .map_err(|e| {
        tracing::error!("Apply failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok("Applied successfully".to_string())
}

async fn add_demand(
    State(state): State<AppState>,
    Json(payload): Json<Demand>,
) -> Result<String, StatusCode> {
    // Sanitize name
    if !payload
        .name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 1. Write the SQL file
    let sql_filename = if payload.kind == "database" {
        format!("create_database_{}.sql", payload.name)
    } else {
        format!("create_table_{}.sql", payload.name)
    };

    // Use tokio fs
    fs::write(&sql_filename, &payload.sql_content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Read playbook
    let playbook_content = fs::read_to_string(&state.playbook_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut playbook: Playbook =
        serde_yaml::from_str(&playbook_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Update playbook struct
    if payload.kind == "database" {
        playbook.databases.push(Database {
            name: payload.name,
            if_not_exists: sql_filename,
        });
    } else if payload.kind == "table" {
        if let Some(db_name) = payload.database {
            playbook.tables.push(Table {
                database: db_name,
                name: payload.name,
                if_not_exists: sql_filename,
            });
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 4. Write back playbook
    let new_yaml =
        serde_yaml::to_string(&playbook).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    fs::write(&state.playbook_path, new_yaml)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok("Demand added".to_string())
}
