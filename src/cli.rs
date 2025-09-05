use clap::{Parser, Subcommand, ValueEnum};
use crate::database::{apply_playbook, destroy_playbook, test_playbook, status};
use crate::playbook::{init_playbook, validate_playbook};

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
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    match cli.command {
        Commands::Apply { playbook, db_url, no_rollback, db_type } => {
            apply_playbook(&playbook, &db_url, false, !no_rollback, db_type).await
        }
        Commands::Plan { playbook, db_url, db_type } => {
            apply_playbook(&playbook, &db_url, true, false, db_type).await
        }
        Commands::Init { playbook } => {
            init_playbook(&playbook).await
        }
        Commands::Test { playbook, db_url, db_type } => {
            test_playbook(&playbook, &db_url, db_type).await
        }
        Commands::Destroy { playbook, db_url, db_type } => {
            destroy_playbook(&playbook, &db_url, db_type).await
        }
        Commands::Validate { playbook } => {
            validate_playbook(&playbook).await
        }
        Commands::Status { playbook, db_url, db_type } => {
            status(&playbook, &db_url, db_type).await
        }
    }
}