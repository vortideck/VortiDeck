use diesel::prelude::*;
use diesel::r2d2::CustomizeConnection;
use diesel::sql_types::Text;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dirs::data_dir;
use dotenvy::dotenv;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::{Path, PathBuf};
// ⚡ PLATFORM-AGNOSTIC: No Tauri dependencies!
// The adapter layer (Tauri app, server, CLI) will pass the db path to us

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Custom connection configuration for SQLite to optimize concurrent access
///
/// For high-throughput scenarios (100+ messages/sec), we use:
/// - WAL mode: Allows concurrent reads during writes
/// - Short busy timeout (500ms): Fail fast if locked, messages stay in memory
/// - NORMAL synchronous: Balance between performance and safety
#[derive(Debug, Clone, Copy)]
struct SqliteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqliteConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::RunQueryDsl;

        // Short busy timeout (500ms) - fail fast if locked
        // Messages stay in memory if DB write fails (not a critical error)
        diesel::sql_query("PRAGMA busy_timeout = 500;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Use NORMAL synchronous mode for better performance (still safe with WAL)
        diesel::sql_query("PRAGMA synchronous = NORMAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Increase cache size to 2MB for better performance
        diesel::sql_query("PRAGMA cache_size = -2000;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        Ok(())
    }
}

// Migration path - embed_migrations looks relative to the CRATE ROOT (where Cargo.toml with [package] is)
// This is vortideck-core/Cargo.toml, so path is from vortideck-core/
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");

/// Resolve VortiDeck's writable data root.
///
/// Packaged acceptance builds may use an isolated directory. The override is
/// not compiled into normal development or production binaries.
pub fn get_vortideck_data_root() -> PathBuf {
    #[cfg(feature = "acceptance-test-hooks")]
    if let Some(path) = env::var_os("VORTIDECK_ACCEPTANCE_DATA_DIR") {
        return PathBuf::from(path);
    }

    data_dir()
        .expect("Could not determine AppData directory")
        .join("VortiDeck")
}

/// Get the path for the SQLite database in the user's AppData directory
pub fn get_database_path() -> PathBuf {
    get_vortideck_data_root().join("vortideck.db")
}

fn get_env_file_path() -> PathBuf {
    get_vortideck_data_root().join(".env")
}

/// Ensure the DATABASE_URL is written to the `.env` file if not already set

fn set_database_url_in_env(db_path: &Path) -> Result<(), String> {
    let db_url = format!(
        "sqlite://{}",
        db_path.to_str().unwrap_or_default().replace('\\', "\\\\")
    );

    // Use the writable path for the `.env` file
    let env_file_path = get_env_file_path();

    // Ensure the directory exists
    if let Some(parent_dir) = env_file_path.parent() {
        fs::create_dir_all(parent_dir).map_err(|error| {
            format!(
                "failed to create database configuration directory {}: {error}",
                parent_dir.display()
            )
        })?;
    }

    let mut env_lines = Vec::new();

    // Read existing lines if the file exists
    if env_file_path.exists() {
        let env_file = std::fs::File::open(&env_file_path)
            .map_err(|error| format!("failed to open {}: {error}", env_file_path.display()))?;
        let reader = BufReader::new(env_file);

        for line in reader.lines() {
            let line = line
                .map_err(|error| format!("failed to read {}: {error}", env_file_path.display()))?;
            if line.starts_with("DATABASE_URL=") {
                env_lines.push(format!("DATABASE_URL={}", db_url));
            } else {
                env_lines.push(line);
            }
        }
    }

    // Add `DATABASE_URL` if it's missing
    if !env_lines
        .iter()
        .any(|line| line.starts_with("DATABASE_URL="))
    {
        env_lines.push(format!("DATABASE_URL={}", db_url));
    }

    // Write the updated lines back to the `.env` file
    let mut env_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&env_file_path)
        .map_err(|error| format!("failed to write {}: {error}", env_file_path.display()))?;
    writeln!(env_file, "{}", env_lines.join("\n"))
        .map_err(|error| format!("failed to write {}: {error}", env_file_path.display()))?;

    // Set the environment variable
    unsafe {
        env::set_var("DATABASE_URL", db_url.clone());
    }
    log::info!(
        "Updated database configuration at {}",
        env_file_path.display()
    );
    Ok(())
}

use crate::db::models::{AppConfig, NewAppConfig, NewButton, NewDeck}; // Import your AppConfig models
use crate::db::schema::app_config; // Import your schema for app_config
use diesel::prelude::*; // Import Diesel's prelude for the necessary traits

/// Seed the database with default general settings on the first run
pub fn seed_general_settings(pool: &DbPool) {
    let mut conn = pool.get().expect("Failed to get DB connection");

    // Check if any AppConfig records exist
    let config_exists = diesel::select(diesel::dsl::exists(
        crate::db::schema::app_config::table.select(crate::db::schema::app_config::key),
    ))
    .get_result::<bool>(&mut conn)
    .expect("Error checking if AppConfig exists");

    // If no configuration exists, insert the default general_settings
    // Platform-agnostic locale detection using sys-locale
    let locale = sys_locale::get_locale();
    let language = locale.as_ref().map(|loc| {
        if let Some(idx) = loc.find('-') {
            loc[0..idx].to_string()
        } else {
            loc.clone()
        }
    });
    println!("Platform Locale: {:?}", locale);
    println!("Language: {:?}", language);
    if !config_exists {
        // Generate desktop_id on first app launch (persists for lifetime of installation)
        let desktop_id = uuid::Uuid::new_v4().to_string();
        println!("📱 Generated desktop_id: {}", desktop_id);

        let initial_config = NewAppConfig {
            key: "general_settings",
            tls_enabled: false, // Default TLS setting
            language: language.as_deref(),
            theme: Some("dark"),
            api_url: Some("https://api.example.com"),
            log_level: Some("info"),
            timezone: Some("UTC"), // Default timezone
            show_instruction: true,
            mcp_enabled: false,
            plugin_hot_reload_enabled: false,
            desktop_id: Some(&desktop_id), // Persistent relay desktop identifier
            max_concurrent_actions: 10,    // Default: 10 concurrent action executions
            max_action_timeout_ms: 300000, // Default: 5 minute timeout (300,000ms)
            relay_enabled: true,
            streaming_enabled: true,        // Default: streaming enabled
            analytics_enabled: false,       // Default: analytics disabled (opt-in consent model)
            analytics_consent_given: false, // Default: user hasn't made a choice yet
        };

        diesel::insert_into(app_config::table)
            .values(&initial_config)
            .execute(&mut conn)
            .expect("Error inserting initial configuration");

        println!("Inserted default general_settings configuration with desktop_id.");
    } else {
        println!("Configuration already exists in the database.");

        // Query the app_config table for the tls_enabled value where the key matches the provided config_key

        // use crate::db::schema::app_config::dsl::*;

        // diesel::update(app_config.filter(key.eq("general_settings")))
        //     .set(tls_enabled.eq(false))
        //     .execute(&mut conn);
    }
}

pub fn seed_example_deck(pool: &DbPool) {
    let mut conn = pool.get().expect("Failed to get DB connection");

    use crate::db::schema::active_deck;
    use crate::db::schema::{buttons, deck_settings, decks};
    use crate::db::NewDeckSettings;
    use crate::db::UpdateActiveDeck;
    use crate::db::{Action, NewActiveDeck};
    use diesel::dsl::count_star;
    // Check if there are any decks in the database
    let deck_count: i64 = decks::dsl::decks
        .select(count_star())
        .first(&mut conn)
        .expect("Error counting decks");

    if deck_count > 0 {
        println!("Decks already exist in the database. Skipping example deck seed.");
        return;
    }

    // Insert the example deck
    let new_deck = NewDeck {
        name: "Example Deck",
    };

    let deck_id: i32 = diesel::insert_into(decks::dsl::decks)
        .values(&new_deck)
        .returning(decks::dsl::id)
        .get_result(&mut conn)
        .expect("Error inserting example deck");

    log::info!("Inserted Example Deck with ID: {}", deck_id);

    // Create default deck settings (inlined from utils)
    let new_deck_settings = NewDeckSettings {
        button_spacing: 10.0,
        grid_margin: 10.0,
        group_buttons: false,
        maximize_button_space: false,
        grid: "3,5".to_string(),
        deck_id,
        buttons_padding: 4.0,
        sides_enabled: false,
        orientation_sync_enabled: true,
        active_theme: None,
    };

    diesel::insert_into(deck_settings::dsl::deck_settings)
        .values(&new_deck_settings)
        .execute(&mut conn)
        .expect("Error inserting example deck settings");

    log::info!("Inserted Example Deck Settings with ID: {}", deck_id);

    // Parse the grid string to extract rows and columns
    let grid_parts: Vec<&str> = new_deck_settings.grid.split(',').collect();
    if grid_parts.len() == 2 {
        // Convert to numbers (assuming valid input)
        let grid_rows: usize = grid_parts[0].parse().expect("Invalid number of rows");
        let grid_columns: usize = grid_parts[1].parse().expect("Invalid number of columns");

        let total_buttons = grid_rows * grid_columns; // Total number of buttons needed

        // Create buttons dynamically
        let mut example_buttons = Vec::new();
        for i in 1..=total_buttons {
            let label = "".to_string();
            let icon = format!("icon{}.png", i);
            // let action_id = Action::Keypress(format!("Action {}", i)); // Example of using the Keypress variant
            let background = format!("button{}_bg.png", i);
            let settings = format!("settings{}.json", i);

            // Serialize the action to a JSON string
            // let serialized_action = serde_json::to_string(&action).expect("Failed to serialize action");

            example_buttons.push(NewButton {
                deck_id,
                label,                        // Passing owned String
                icon: None,                   // Passing owned String
                action: None,                 // Passing the serialized JSON string of the action
                background: Some(background), // Passing owned String
                position: i as i32,           // Position can be the index in the grid
                mode: None,
                execution_mode: Some("run_once".to_string()),
                execution_config: None,
            });
        }

        // Insert the dynamically created buttons into the database
        diesel::insert_into(buttons::dsl::buttons)
            .values(&example_buttons)
            .execute(&mut conn)
            .expect("Error inserting example buttons");

        println!(
            "Inserted {} buttons for Example Deck ID: {}",
            example_buttons.len(),
            deck_id
        );
    } else {
        println!("Invalid grid format");
    }

    // Set the Example Deck as the active deck (Assuming only one active deck exists with id = 1)
    let active_deck_entry = NewActiveDeck {
        deck_id: &deck_id, // Set the new deck as active
    };

    // Insert the active deck, assuming it is a new entry
    diesel::insert_into(active_deck::dsl::active_deck)
        .values(&active_deck_entry)
        .execute(&mut conn)
        .expect("Error inserting active deck");

    println!("Set Example Deck with ID: {} as active deck", deck_id);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseMigrationOutcome {
    pub applied_versions: Vec<String>,
    pub backup_path: Option<PathBuf>,
}

// diesel's QueryableByName derive generates `field: field` initialisers, which
// newer Clippy reports as redundant_field_names. The allow has to cover the
// module because the generated impl sits beside the struct, not inside it.
#[allow(clippy::redundant_field_names)]
mod rows {
    use super::*;

    #[derive(QueryableByName)]
    pub(super) struct IntegrityCheckRow {
        #[diesel(sql_type = Text)]
        pub(super) quick_check: String,
    }
}
use rows::IntegrityCheckRow;

fn verify_database_integrity(connection: &mut SqliteConnection) -> Result<(), String> {
    let rows = diesel::sql_query("PRAGMA quick_check")
        .load::<IntegrityCheckRow>(connection)
        .map_err(|error| format!("could not check SQLite database integrity: {error}"))?;

    if rows.len() == 1 && rows[0].quick_check.eq_ignore_ascii_case("ok") {
        return Ok(());
    }

    let details = rows
        .into_iter()
        .map(|row| row.quick_check)
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!("SQLite database integrity check failed: {details}"))
}

fn create_pre_migration_backup(
    connection: &mut SqliteConnection,
    db_path: &Path,
) -> Result<PathBuf, String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| format!("database path has no parent: {}", db_path.display()))?;
    let file_stem = db_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("vortideck");
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let backup_path = parent.join(format!("{file_stem}.pre-migration-{timestamp}.db"));

    // VACUUM INTO creates a consistent standalone snapshot, including committed WAL data.
    // SQLite string literals escape a quote by doubling it.
    let escaped_path = backup_path.to_string_lossy().replace('\'', "''");
    diesel::sql_query(format!("VACUUM INTO '{escaped_path}'"))
        .execute(connection)
        .map_err(|error| {
            format!(
                "failed to create pre-migration backup {}: {error}",
                backup_path.display()
            )
        })?;

    Ok(backup_path)
}

pub fn initialize_database_at(db_path: &Path) -> Result<DatabaseMigrationOutcome, String> {
    if let Some(parent_dir) = db_path.parent() {
        fs::create_dir_all(parent_dir).map_err(|error| {
            format!(
                "failed to create database directory {}: {error}",
                parent_dir.display()
            )
        })?;
    }

    let existed_with_data = db_path
        .metadata()
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    let database_url = db_path.to_string_lossy();
    let mut connection = SqliteConnection::establish(&database_url)
        .map_err(|error| format!("could not open database {}: {error}", db_path.display()))?;

    if existed_with_data {
        verify_database_integrity(&mut connection)?;
    }

    let has_pending = connection
        .has_pending_migration(MIGRATIONS)
        .map_err(|error| format!("could not inspect pending database migrations: {error}"))?;
    let backup_path = if has_pending && existed_with_data {
        Some(create_pre_migration_backup(&mut connection, db_path)?)
    } else {
        None
    };

    let applied_versions = connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| {
            let recovery = backup_path
                .as_ref()
                .map(|path| format!(" A recovery snapshot is available at {}.", path.display()))
                .unwrap_or_default();
            format!("database migration failed: {error}.{recovery}")
        })?
        .into_iter()
        .map(|version| version.to_string())
        .collect();

    // Journal mode is a database-wide transition and must happen once before
    // r2d2 opens connections in parallel. Doing this in on_acquire races pool
    // startup and produces transient SQLITE_BUSY failures.
    diesel::sql_query("PRAGMA journal_mode = WAL")
        .execute(&mut connection)
        .map_err(|error| format!("could not enable SQLite WAL mode: {error}"))?;
    verify_database_integrity(&mut connection)?;

    Ok(DatabaseMigrationOutcome {
        applied_versions,
        backup_path,
    })
}

/// Initialize the writable user database using the embedded Diesel migration chain.
/// Existing databases are integrity-checked and snapshotted before pending migrations run.
pub fn initialize_database() -> Result<DatabaseMigrationOutcome, String> {
    let db_path = get_database_path();
    set_database_url_in_env(&db_path)?;
    initialize_database_at(&db_path)
}

/// Establish a connection to the SQLite database
pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
/// Initialize the connection pool for the migrated writable user database.
///
/// ⚡ PLATFORM-AGNOSTIC: Works for desktop, server, CLI
///
/// # Examples
/// ```no_run
/// use vortideck_core::db::db::establish_pool;
///
/// let pool = establish_pool();
/// ```
pub fn establish_pool() -> DbPool {
    establish_pool_at(&get_database_path()).expect("Unable to establish database connection pool")
}

/// Initialize a pool for an explicitly owned database path. Headless hosts and
/// tests use this instead of mutating process-global `DATABASE_URL` state.
pub fn establish_pool_at(db_path: &Path) -> Result<DbPool, String> {
    let database_url = db_path.to_string_lossy().to_string();
    if !Path::new(&database_url).is_file() {
        return Err(format!(
            "database must be initialized before creating its connection pool: {}",
            db_path.display()
        ));
    }

    // Create the connection manager with custom connection configuration
    let manager = ConnectionManager::<SqliteConnection>::new(database_url.clone());

    // Build and return the database connection pool with SQLite optimizations
    let pool = Pool::builder()
        .max_size(15)
        .test_on_check_out(true)
        .connection_customizer(Box::new(SqliteConnectionCustomizer))
        .build(manager)
        .map_err(|error| format!("database pool creation failed: {error}"))?;

    seed_general_settings(&pool);
    seed_example_deck(&pool);
    Ok(pool)
}

/// Helper function to get a connection from the pool
pub fn get_connection(pool: &DbPool) -> PooledConnection<ConnectionManager<SqliteConnection>> {
    match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to get a database connection: {}", err);
            panic!("Database connection error"); // Optionally panic or return a `Result`
        }
    }
}

#[cfg(test)]
// The QueryableByName rows below hit the same derive-generated lint as `rows`.
#[allow(clippy::redundant_field_names)]
mod migration_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::sql_types::{BigInt, Bool, Text};
    use tempfile::TempDir;

    #[derive(QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = BigInt)]
        count: i64,
    }

    #[derive(QueryableByName)]
    struct PermissionGrantRow {
        #[diesel(sql_type = Text)]
        package_id: String,
        #[diesel(sql_type = Text)]
        content_hash: String,
    }

    #[derive(QueryableByName)]
    struct OrientationSyncRow {
        #[diesel(sql_type = Bool)]
        orientation_sync_enabled: bool,
    }

    fn database_path(temp_dir: &TempDir) -> PathBuf {
        temp_dir.path().join("vortideck.db")
    }

    fn connect(path: &Path) -> SqliteConnection {
        SqliteConnection::establish(&path.to_string_lossy()).expect("test database connection")
    }

    fn table_count(connection: &mut SqliteConnection, table_name: &str) -> i64 {
        diesel::sql_query(format!(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = '{}'",
            table_name.replace('\'', "''")
        ))
        .get_result::<CountRow>(connection)
        .expect("table lookup")
        .count
    }

    #[test]
    fn fresh_database_runs_the_complete_migration_chain_without_backup() {
        let temp_dir = tempfile::tempdir().expect("temporary directory");
        let path = database_path(&temp_dir);

        let outcome = initialize_database_at(&path).expect("fresh migration succeeds");

        assert!(!outcome.applied_versions.is_empty());
        assert_eq!(outcome.backup_path, None);
        let mut connection = connect(&path);
        verify_database_integrity(&mut connection).expect("fresh database is valid");
        assert_eq!(table_count(&mut connection, "package_permission_grants"), 1);
        assert_eq!(table_count(&mut connection, "overlay_media_queue"), 1);
        assert_eq!(table_count(&mut connection, "widget_packages"), 0);
        assert!(!connection
            .has_pending_migration(MIGRATIONS)
            .expect("pending migration check"));
    }

    #[test]
    fn orientation_sync_migration_preserves_existing_decks_as_independent() {
        let temp_dir = tempfile::tempdir().expect("temporary directory");
        let path = database_path(&temp_dir);
        let mut connection = connect(&path);
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("initial migration chain");
        connection
            .revert_last_migration(MIGRATIONS)
            .expect("restore schema before orientation sync");
        connection
            .batch_execute(
                "INSERT INTO decks (name) VALUES ('Existing deck');
                 INSERT INTO deck_settings
                 (button_spacing, grid_margin, group_buttons, maximize_button_space, grid,
                  deck_id, buttons_padding, sides_enabled, active_theme)
                 VALUES (10, 10, 0, 0, '3,5', last_insert_rowid(), 4, 0, NULL);",
            )
            .expect("existing deck settings");
        drop(connection);

        let outcome = initialize_database_at(&path).expect("orientation migration succeeds");
        assert!(outcome.backup_path.is_some());
        let mut migrated = connect(&path);
        let settings =
            diesel::sql_query("SELECT orientation_sync_enabled FROM deck_settings LIMIT 1")
                .get_result::<OrientationSyncRow>(&mut migrated)
                .expect("migrated orientation setting");
        assert!(!settings.orientation_sync_enabled);
    }

    #[test]
    fn legacy_database_is_backed_up_and_preserves_permission_grants() {
        let temp_dir = tempfile::tempdir().expect("temporary directory");
        let path = database_path(&temp_dir);
        let mut connection = connect(&path);
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("initial migration chain");
        while table_count(&mut connection, "widget_packages") == 0 {
            connection
                .revert_last_migration(MIGRATIONS)
                .expect("restore pre-permission-generalization schema");
        }
        connection
            .batch_execute(
                "INSERT INTO widget_packages
                 (widget_id, manifest_json, permissions_json, content_hash, trust, granted_at)
                 VALUES
                 ('example.widget', '{}', '[\"overlay.write\"]', 'known-hash', 'community',
                  '2026-08-01T00:00:00Z');",
            )
            .expect("legacy permission grant");
        drop(connection);

        let outcome = initialize_database_at(&path).expect("legacy migration succeeds");
        let backup_path = outcome.backup_path.expect("pre-migration backup");

        assert!(backup_path.is_file());
        assert!(!outcome.applied_versions.is_empty());
        let mut migrated = connect(&path);
        let grant = diesel::sql_query(
            "SELECT package_id, content_hash FROM package_permission_grants
             WHERE package_id = 'example.widget'",
        )
        .get_result::<PermissionGrantRow>(&mut migrated)
        .expect("migrated grant");
        assert_eq!(grant.package_id, "example.widget");
        assert_eq!(grant.content_hash, "known-hash");

        let mut backup = connect(&backup_path);
        assert_eq!(table_count(&mut backup, "widget_packages"), 1);
        assert_eq!(table_count(&mut backup, "package_permission_grants"), 0);
    }

    #[test]
    fn failed_migration_keeps_a_recoverable_snapshot() {
        let temp_dir = tempfile::tempdir().expect("temporary directory");
        let path = database_path(&temp_dir);
        let mut connection = connect(&path);
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("initial migration chain");
        while table_count(&mut connection, "widget_packages") == 0 {
            connection
                .revert_last_migration(MIGRATIONS)
                .expect("restore pre-permission-generalization schema");
        }
        connection
            .batch_execute("DROP TABLE widget_packages;")
            .expect("create migration precondition failure");
        drop(connection);

        let error = initialize_database_at(&path).expect_err("migration must fail");

        assert!(error.contains("database migration failed"));
        assert!(error.contains("recovery snapshot"));
        let backups = fs::read_dir(temp_dir.path())
            .expect("backup directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".pre-migration-"))
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let mut backup = connect(&backups[0]);
        verify_database_integrity(&mut backup).expect("recovery snapshot is valid");
    }

    #[test]
    fn current_database_does_not_create_redundant_backups() {
        let temp_dir = tempfile::tempdir().expect("temporary directory");
        let path = database_path(&temp_dir);
        initialize_database_at(&path).expect("initial migration");

        let outcome = initialize_database_at(&path).expect("second initialization");

        assert!(outcome.applied_versions.is_empty());
        assert_eq!(outcome.backup_path, None);
    }
}
