use serde::{Deserialize, Serialize};
use std::path::Path;
use config::builder::DefaultState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub timeout: u64,
    pub auth: Option<AuthConfig>,
    pub output: OutputConfig,
    pub cluster: Option<ClusterConfig>,
    pub logging: Option<LoggingConfig>,
    pub export: Option<ExportConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub color: bool,
    pub pretty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "yaml")]
    Yaml,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "csv")]
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    #[serde(default = "default_query_endpoint")]
    pub query_endpoint: String,
    #[serde(default = "default_query_range_endpoint")]
    pub query_range_endpoint: String,
    #[serde(default = "default_health_endpoint")]
    pub health_endpoint: String,
    #[serde(default = "default_metrics_endpoint")]
    pub metrics_endpoint: String,
    #[serde(default)]
    pub use_select_endpoint: bool,
    #[serde(default = "default_account_id")]
    pub select_account_id: String,
    #[serde(default = "default_project_id")]
    pub select_project_id: String,
    pub vminsert_host: Option<String>,
    pub vmstorage_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    #[serde(default = "default_export_format")]
    pub default_format: String,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

fn default_query_endpoint() -> String { "/api/v1/query".to_string() }
fn default_query_range_endpoint() -> String { "/api/v1/query_range".to_string() }
fn default_health_endpoint() -> String { "/health".to_string() }
fn default_metrics_endpoint() -> String { "/api/v1/label/__name__/values".to_string() }
fn default_account_id() -> String { "0".to_string() }
fn default_project_id() -> String { "0".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_export_format() -> String { "prometheus".to_string() }
fn default_chunk_size() -> usize { 1000 }

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "http://localhost:8428".to_string(),
            timeout: 30,
            auth: None,
            output: OutputConfig {
                format: OutputFormat::Table,
                color: true,
                pretty: true,
            },
            cluster: None,
            logging: None,
            export: None,
        }
    }
}

impl Config {
    pub fn load(config_path: Option<&str>) -> crate::error::Result<Self> {
        let mut builder: config::ConfigBuilder<DefaultState> = config::ConfigBuilder::default();

        builder = builder.set_default("host", "http://localhost:8428")?;
        builder = builder.set_default("timeout", 30)?;
        builder = builder.set_default("output.format", "table")?;
        builder = builder.set_default("output.color", true)?;
        builder = builder.set_default("output.pretty", true)?;

        if let Some(path) = config_path {
            if Path::new(path).exists() {
                builder = builder.add_source(config::File::with_name(path));
            }
        } else {
            let config_dirs = vec![
                dirs::config_dir().map(|p| p.join("vm-cli").join("config.toml")),
                Some(std::env::current_dir()?.join(".vm-cli.toml")),
                Some(std::env::current_dir()?.join("vm-cli.toml")),
            ];

            for config_path in config_dirs {
                if let Some(path) = config_path {
                    if path.exists() {
                        builder = builder.add_source(config::File::from(path));
                        break;
                    }
                }
            }
        }

        builder = builder.add_source(config::Environment::with_prefix("VM"));

        let config = builder.build()?;
        let config: Config = config.try_deserialize()?;
        
        tracing::debug!("Загружена конфигурация: host={}", config.host);
        
        Ok(config)
    }

    pub fn save(&self, path: &str) -> crate::error::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::VmCliError::Unknown(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
