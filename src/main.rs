use clap::{Parser, Subcommand};
use colored::*;
use tracing::{error, info};

mod api;
mod commands;
mod config;
mod error;
mod utils;

use commands::{
    admin::AdminCommand, debug::DebugCommand, export::ExportCommand, health::HealthCommand,
    import::ImportCommand, query::QueryCommand,
};
use config::Config;
use error::VmCliError;

#[derive(Parser)]
#[command(
    name = "vm-cli",
    about = "CLI инструмент для работы с VictoriaMetrics",
    version,
    long_about = "Универсальный CLI-инструмент для мониторинга, администрирования и анализа данных VictoriaMetrics"
)]
struct Cli {
    #[arg(long, default_value = "http://localhost:8428")]
    host: String,

    #[arg(short, long, default_value = "30")]
    timeout: u64,

    #[arg(short, long)]
    config: Option<String>,

    #[arg(long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Выполнение запросов к VictoriaMetrics
    Query(QueryCommand),
    
    /// Проверка здоровья кластера
    Health(HealthCommand),
    
    /// Экспорт данных
    Export(ExportCommand),
    
    /// Импорт данных
    Import(ImportCommand),
    
    /// Администрирование VictoriaMetrics
    Admin(AdminCommand),
    
    /// Отладка и диагностика
    Debug(DebugCommand),
}

#[tokio::main]
async fn main() -> Result<(), VmCliError> {
    let mut cli = Cli::parse();

    // Поддержка переменных окружения
    if cli.host == "http://localhost:8428" {
        if let Ok(env_host) = std::env::var("VM_HOST") {
            cli.host = env_host;
        }
    }
    
    if cli.timeout == 30 {
        if let Ok(env_timeout) = std::env::var("VM_TIMEOUT") {
            if let Ok(timeout) = env_timeout.parse() {
                cli.timeout = timeout;
            }
        }
    }
    
    if cli.config.is_none() {
        if let Ok(env_config) = std::env::var("VM_CONFIG") {
            cli.config = Some(env_config);
        }
    }
    
    if !cli.verbose {
        if let Ok(env_verbose) = std::env::var("VM_VERBOSE") {
            cli.verbose = env_verbose == "1" || env_verbose.to_lowercase() == "true";
        }
    }

    // Загрузка конфигурации
    let config = Config::load(cli.config.as_deref())?;
    
    // Настройка логирования
    let log_level = if cli.verbose {
        "debug"
    } else {
        &config.logging.as_ref().and_then(|l| Some(l.level.clone())).unwrap_or_else(|| "info".to_string())
    };
    
    // Настраиваем логирование
    if let Some(logging_config) = &config.logging {
        if let Some(log_file) = &logging_config.file {
            // Создаем директорию для логов, если её нет
            if let Some(parent) = std::path::Path::new(log_file).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            
            // Создаем простой файловый writer
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)
                .expect("Не удалось открыть файл для логирования");
            
            // Настраиваем логирование с выводом в файл и консоль
            tracing_subscriber::fmt()
                .with_env_filter(format!("vm_cli={}", log_level))
                .with_writer(file)
                .init();
        } else {
            // Только консольное логирование
            tracing_subscriber::fmt()
                .with_env_filter(format!("vm_cli={}", log_level))
                .init();
        }
    } else {
        // Только консольное логирование
        tracing_subscriber::fmt()
            .with_env_filter(format!("vm_cli={}", log_level))
            .init();
    }

    info!("Запуск vm-cli v{}", env!("CARGO_PKG_VERSION"));
    
    if cli.verbose {
        info!("Загружена конфигурация: host={}, timeout={}", config.host, config.timeout);
        if let Some(cluster) = &config.cluster {
            info!("Конфигурация кластера: query_endpoint={}", cluster.query_endpoint);
        }
    }
    
    // Создание API клиента
    let api_client = api::VmClient::new(&config.host, config.timeout, config.cluster)?;

    // Выполнение команды
    let result = match cli.command {
        Commands::Query(cmd) => cmd.execute(&api_client).await,
        Commands::Health(cmd) => cmd.execute(&api_client).await,
        Commands::Export(cmd) => cmd.execute(&api_client).await,
        Commands::Import(cmd) => cmd.execute(&api_client).await,
        Commands::Admin(cmd) => cmd.execute(&api_client).await,
        Commands::Debug(cmd) => cmd.execute(&api_client).await,
    };

    match result {
        Ok(_) => {
            info!("Команда выполнена успешно");
            Ok(())
        }
        Err(e) => {
            error!("Ошибка выполнения команды: {}", e);
            eprintln!("{} {}", "ОШИБКА:".red().bold(), e);
            Err(e)
        }
    }
}
