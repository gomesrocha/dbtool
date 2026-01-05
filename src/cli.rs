use crate::database::{apply_playbook, destroy_playbook, status, test_playbook};
use crate::playbook::{init_playbook, validate_playbook};
use crate::server::start_server;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[clap(name = "dbtool", about = "A database schema management tool")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Clone, ValueEnum)]
pub enum DbType {
    Postgres,
    MySQL,
}

#[derive(Subcommand)]
pub enum Commands {
    Apply {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, default_value = "false")]
        no_rollback: bool,
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
        #[clap(long, default_value = "false")]
        auto_approve: bool,
    },
    Plan {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
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
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
    },
    Destroy {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
    },
    Validate {
        #[clap(long)]
        playbook: String,
    },
    Status {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
    },
    Serve {
        #[clap(long)]
        playbook: String,
        #[clap(long)]
        db_url: String,
        #[clap(long, value_enum, default_value = "postgres")]
        db_type: DbType,
        #[clap(long, default_value = "3000")]
        port: u16,
        #[clap(long, env = "DBTOOLS_USER", default_value = "admin")]
        username: String,
        #[clap(long, env = "DBTOOLS_PASS", default_value = "password")]
        password: String,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    match cli.command {
        Commands::Apply {
            playbook,
            db_url,
            no_rollback,
            db_type,
            auto_approve,
        } => {
            apply_playbook(
                &playbook,
                &db_url,
                false,
                !no_rollback,
                db_type,
                auto_approve,
            )
            .await
        }
        Commands::Plan {
            playbook,
            db_url,
            db_type,
        } => apply_playbook(&playbook, &db_url, true, false, db_type, false).await,
        Commands::Init { playbook } => init_playbook(&playbook).await,
        Commands::Test {
            playbook,
            db_url,
            db_type,
        } => test_playbook(&playbook, &db_url, db_type).await,
        Commands::Destroy {
            playbook,
            db_url,
            db_type,
        } => destroy_playbook(&playbook, &db_url, db_type).await,
        Commands::Validate { playbook } => validate_playbook(&playbook).await,
        Commands::Status {
            playbook,
            db_url,
            db_type,
        } => status(&playbook, &db_url, db_type).await,
        Commands::Serve {
            playbook,
            db_url,
            db_type,
            port,
            username,
            password,
        } => start_server(playbook, db_url, db_type, port, username, password).await,
    }
}
