use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
use std::path::Path;
use anyhow::Result;
use super::config::AppConfig;

#[derive(Clone)]
pub struct Databases {
    pub users: Pool<Sqlite>,
    pub transactions: Pool<Sqlite>,
    pub stats: Pool<Sqlite>,
}

impl Databases {
    /// Initializes the databases specified in the `AppConfig`.
    ///
    /// This function will create the database files if they don't exist, and then connect to them.
    /// It will also set some default PRAGMA values.
    ///
    /// The function will return a `Databases` object containing the connected database pools.
    ///
    /// # Errors
    ///
    /// This function will return an error if any of the following operations fail:
    /// - Creating a database file
    /// - Connecting to a database
    /// - Setting PRAGMA values for a database
    pub async fn init(cfg: &AppConfig) -> Result<Self> {
        for path in cfg.databases.values() {
            let p = Path::new(path);
            if !p.exists() {
                std::fs::File::create(p)?;
                println!("🆕 Created empty DB file {:?}", p);
            }
        }

        let mut pools = vec![];
        for name in ["users", "transactions", "stats"] {
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&format!("sqlite://{}", cfg.databases[name]))
                .await?;
            
            for pragma in [
                "PRAGMA journal_mode=WAL;",
                "PRAGMA synchronous=NORMAL;",
                "PRAGMA temp_store=MEMORY;",
                "PRAGMA cache_size=-10000;",
                "PRAGMA foreign_keys=ON;"
            ] {
                sqlx::query(pragma).execute(&pool).await?;
            }

            println!("✅ Connected to {} database at {} with {} connections", name, cfg.databases[name], pool.size());

            pools.push(pool);
        }
        
        Ok(Self {
            users: pools[0].clone(),
            transactions: pools[1].clone(),
            stats: pools[2].clone(),
        })
    }
}
