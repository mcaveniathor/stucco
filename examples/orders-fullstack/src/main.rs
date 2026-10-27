// The optimized SSR entry point instantiates the same nested Leptos views as
// the library, so both compilation units need the matching recursion limit.
#![recursion_limit = "256"]

use clap::{Parser, Subcommand, ValueEnum};
use orders_fullstack::sessions::Sessions;
use orders_fullstack::{
    auth,
    store::{OrderStore, StoreError},
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use tower_sessions::ExpiredDeletion;

/// Full-stack Orders reference and offline database maintenance.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Database file; falls back to ORDERS_DATABASE, then orders.redb.
    #[arg(long, global = true)]
    database: Option<PathBuf>,
    /// Storage engine; SQLite requires a build with --features sqlite.
    #[arg(long, global = true, value_enum, default_value_t = DatabaseBackend::Redb)]
    backend: DatabaseBackend,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Clone, Copy, Default, ValueEnum)]
enum DatabaseBackend {
    #[default]
    Redb,
    #[cfg(feature = "sqlite")]
    Sqlite,
}
impl DatabaseBackend {
    async fn open(self, path: &Path) -> Result<OrderStore, StoreError> {
        match self {
            Self::Redb => OrderStore::open(path).await,
            #[cfg(feature = "sqlite")]
            Self::Sqlite => OrderStore::open_sqlite(path).await,
        }
    }
    async fn validate(self, path: &Path) -> Result<(), StoreError> {
        match self {
            Self::Redb => OrderStore::validate_backup(path).await,
            #[cfg(feature = "sqlite")]
            Self::Sqlite => OrderStore::validate_sqlite_backup(path).await,
        }
    }
    fn filename(self) -> &'static str {
        match self {
            Self::Redb => "orders.redb",
            #[cfg(feature = "sqlite")]
            Self::Sqlite => "orders.sqlite3",
        }
    }
}
#[derive(Subcommand)]
enum Command {
    /// Serve the application (also the default command).
    Serve,
    /// Provision an account. Read its password from stdin, never command arguments.
    UserAdd {
        #[arg(long)]
        tenant: String,
        #[arg(long)]
        username: String,
        #[arg(long)]
        display_name: String,
    },
    /// Seed sample orders only if the user's tenant currently has none.
    Seed {
        #[arg(long)]
        username: String,
    },
    /// Revoke all sessions for an account, including other browser devices.
    SessionsRevoke {
        #[arg(long)]
        username: String,
    },
    /// Disable an account and invalidate its sessions. Stop the redb server first.
    UserDisable {
        #[arg(long)]
        username: String,
    },
    /// Write a consistent snapshot to a new file. Stop the redb server first.
    Backup {
        #[arg(long)]
        destination: PathBuf,
    },
    /// Restore a validated snapshot into a NEW database file; invalidates sessions.
    Restore {
        #[arg(long)]
        source: PathBuf,
    },
}
#[tokio::main]
async fn main() -> Result<(), StoreError> {
    let cli = Cli::parse();
    let database = cli
        .database
        .or_else(|| std::env::var_os("ORDERS_DATABASE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(cli.backend.filename()));
    let command = cli.command.unwrap_or(Command::Serve);
    if let Command::Restore { source } = command {
        return restore(cli.backend, &source, &database).await;
    }
    if !matches!(command, Command::UserAdd { .. }) && !database.is_file() {
        return Err("Database does not exist; provision an account with user-add first or correct --database".into());
    }
    let options = if matches!(command, Command::Serve) {
        let options = leptos::config::get_configuration(None)?.leptos_options;
        orders_fullstack::server::validate_runtime_options(&options)?;
        Some(options)
    } else {
        None
    };
    let store = cli.backend.open(&database).await?;
    let result = match command {
        Command::UserAdd {
            tenant,
            username,
            display_name,
        } => {
            if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                return Err("Pipe the password on stdin from a secure prompt or secret manager; terminal echo is not disabled".into());
            }
            let mut password = String::new();
            std::io::stdin().take(1027).read_to_string(&mut password)?;
            if password.ends_with('\n') {
                password.pop();
                if password.ends_with('\r') {
                    password.pop();
                }
            }
            auth::add_user(&store, &tenant, &username, &display_name, password).await?;
            println!("Account created");
            Ok(())
        }
        Command::Seed { username } => {
            store
                .seed_orders(&auth::principal_for_username(&store, &username).await?)
                .await?;
            println!("Sample orders are ready");
            Ok(())
        }
        Command::SessionsRevoke { username } => {
            if !store.revoke_sessions(&username).await? {
                return Err("Account not found or session epoch exhausted".into());
            }
            println!("Sessions revoked");
            Ok(())
        }
        Command::UserDisable { username } => {
            if !store.set_user_enabled(&username, false).await? {
                return Err("Account not found".into());
            }
            println!("Account disabled");
            Ok(())
        }
        Command::Backup { destination } => {
            store.backup_to(&destination).await?;
            println!("Backup created");
            Ok(())
        }
        Command::Serve => serve(store.clone(), options.expect("validated serve options")).await,
        Command::Restore { .. } => unreachable!(),
    };
    store.close().await;
    result
}
async fn serve(
    store: OrderStore,
    options: leptos::config::LeptosOptions,
) -> Result<(), StoreError> {
    let addr = options.site_addr;
    let origin = std::env::var("ORDERS_ORIGIN").unwrap_or_else(|_| format!("http://{addr}"));
    // Persistent sessions must never outlive a silently changing CSRF key.
    let secret = std::env::var("STUCCO_SECRET_KEY")
        .map_err(|_| "Set STUCCO_SECRET_KEY to 64 random hexadecimal digits")?;
    let key = stucco_tower::SecretKey::from_hex(&secret)?;
    let seconds = std::env::var("ORDERS_SESSION_SECONDS")
        .unwrap_or_else(|_| "28800".into())
        .parse::<i64>()?;
    let (app, exports) =
        orders_fullstack::server::app_with_exports(options, key, &origin, store.clone(), seconds)
            .await?;
    let session_store = Sessions::new(&store).await?;
    session_store.delete_expired().await?;
    let cleanup = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if session_store.delete_expired().await.is_err() {
                eprintln!("Expired session cleanup failed");
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Orders full-stack application: {origin}");
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await;
    exports.shutdown().await;
    cleanup.abort();
    let _ = cleanup.await;
    result?;
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
async fn restore(
    backend: DatabaseBackend,
    source: &Path,
    destination: &Path,
) -> Result<(), StoreError> {
    // Accept self-contained snapshots only. Never silently omit SQLite WAL data.
    require_no_sidecars(source)?;
    require_new_destination(destination)?;
    // Default redb's read-only file lock prevents a writer changing the source
    // until the copied snapshot has been validated. Never migrate the source.
    let guard = match backend {
        DatabaseBackend::Redb => Some(redb::ReadOnlyDatabase::open(source)?),
        #[cfg(feature = "sqlite")]
        DatabaseBackend::Sqlite => None,
    };
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // NamedTempFile owns cleanup on every error before publication. Keeping it
    // beside the destination makes publication stay on the same filesystem.
    let mut staged = tempfile::NamedTempFile::with_prefix_in(".orders-restore-", parent)?;
    let _sidecars = RestoreSidecars(staged.path().to_owned());
    std::io::copy(&mut std::fs::File::open(source)?, staged.as_file_mut())?;
    staged.as_file().sync_all()?;
    require_no_sidecars(source)?;
    backend.validate(staged.path()).await?;
    drop(guard);

    let store = backend.open(staged.path()).await?;
    let prepared: Result<(), StoreError> = async {
        let sessions = Sessions::new(&store).await?;
        sessions.clear().await?;
        orders_fullstack::exports::clear_stored(&store).await?;
        store.rotate_generation().await?;
        Ok(())
    }
    .await;
    // Close even when preparation fails, before the staging guards clean up.
    store.close().await;
    prepared?;
    #[cfg(feature = "sqlite")]
    if matches!(backend, DatabaseBackend::Sqlite) {
        finish_sqlite_restore(staged.path()).await?;
    }
    backend.validate(staged.path()).await?;
    require_no_sidecars(staged.path())?;
    staged.as_file().sync_all()?;
    // Preparation can take time. Recheck sidecars, then atomically refuse any
    // destination created since the check instead of overwriting it.
    require_new_destination(destination)?;
    staged
        .persist_noclobber(destination)
        .map_err(|error| error.error)?;
    #[cfg(unix)]
    {
        std::fs::File::open(parent)?.sync_all()?;
    }
    println!("Restore validated; all restored sessions were invalidated");
    Ok(())
}

#[cfg(feature = "sqlite")]
async fn finish_sqlite_restore(path: &Path) -> Result<(), StoreError> {
    use sqlx::{
        Connection, SqliteConnection,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    };

    // This is only the owned, already validated staging file. Leaving WAL mode
    // checkpoints committed writes into it; deleting sidecars ourselves could
    // lose the session clearing or generation rotation. DELETE mode also keeps
    // the final read-only validation from creating another WAL/SHM pair.
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Delete)
            .synchronous(SqliteSynchronous::Full),
    )
    .await?;
    let mode = sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
        .fetch_one(&mut connection)
        .await;
    let closed = connection.close().await;
    if !mode?.eq_ignore_ascii_case("delete") {
        return Err("Restore staging database could not leave WAL mode".into());
    }
    closed?;
    Ok(())
}

fn appended_path(path: &Path, suffix: &str) -> PathBuf {
    let mut path = path.as_os_str().to_owned();
    path.push(suffix);
    path.into()
}

fn path_entry_exists(path: &Path) -> Result<bool, std::io::Error> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn require_no_sidecars(path: &Path) -> Result<(), StoreError> {
    for suffix in ["-wal", "-shm", "-journal"] {
        if path_entry_exists(&appended_path(path, suffix))? {
            return Err("Restore source has sidecars; create a self-contained backup first".into());
        }
    }
    Ok(())
}

fn require_new_destination(path: &Path) -> Result<(), StoreError> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        if path_entry_exists(&appended_path(path, suffix))? {
            return Err("Restore destination or sidecars already exist; choose a new path".into());
        }
    }
    Ok(())
}

struct RestoreSidecars(PathBuf);

impl Drop for RestoreSidecars {
    fn drop(&mut self) {
        for suffix in ["-wal", "-shm", "-journal"] {
            let _ = std::fs::remove_file(appended_path(&self.0, suffix));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn cli_configuration_and_password_boundary() {
        Cli::command().debug_assert();
        assert!(
            Cli::try_parse_from([
                "orders",
                "user-add",
                "--tenant",
                "team",
                "--username",
                "user",
                "--display-name",
                "User"
            ])
            .is_ok()
        );
        assert!(Cli::try_parse_from(["orders", "user-add", "--password", "exposed"]).is_err());
        assert!(Cli::try_parse_from(["orders", "restore"]).is_err());
    }
}
