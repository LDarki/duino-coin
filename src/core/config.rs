use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::Path};
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub tcp_host: String,
    pub tcp_port: u16,
    pub socket_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub bcrypt_rounds: u32,
    pub argon_mem_kb: Option<u32>,
    pub argon_time: Option<u32>,
    pub argon_parallelism: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub base_dir: String,
    pub db_folder: String,
    pub databases: HashMap<String, String>,
    pub server: ServerConfig,
    pub security: SecurityConfig,
}

impl AppConfig {
    pub fn load(path: Option<&str>) -> Result<Self> {
        let base_dir = env::current_dir()?.display().to_string();
        let db_folder = format!("{}/db", base_dir);

        if !Path::new(&db_folder).exists() {
            fs::create_dir_all(&db_folder)?;
            println!("📁 Created DB folder at {}", db_folder);
        }

        let mut databases = HashMap::new();
        databases.insert("users".to_string(), format!("{}/crypto_database.db", db_folder));
        databases.insert("transactions".to_string(), format!("{}/transactions.db", db_folder));
        databases.insert("stats".to_string(), format!("{}/stats.db", db_folder));

        let default_server = ServerConfig {
            tcp_host: "127.0.0.1".to_string(),
            tcp_port: 4000,
            socket_timeout: 60,
        };
        let default_security = SecurityConfig {
            bcrypt_rounds: 6,
            argon_mem_kb: Some(65536),
            argon_time: Some(3),
            argon_parallelism: Some(4),
        };

        let mut cfg = Self {
            base_dir,
            db_folder,
            databases,
            server: default_server,
            security: default_security,
        };

        if let Some(p) = path {
            let s = fs::read_to_string(p)?;
            let file_cfg: AppConfig = toml::from_str(&s)?;
            cfg = file_cfg;
        }

        Ok(cfg)
    }
}
