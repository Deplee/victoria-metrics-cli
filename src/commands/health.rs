use crate::api::VmClient;
use crate::error::Result;
use crate::utils::{format_health_status, format_uptime};
use clap::Parser;
use colored::*;
use tracing::info;

#[derive(Parser)]
pub struct HealthCommand {
    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    status_only: bool,
}

impl HealthCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        info!("Проверка здоровья VictoriaMetrics");

        let health = client.health().await?;

        if self.status_only {
            println!("{}", health.status);
            return Ok(());
        }

        let status_display = format_health_status(&health.status);
        println!("{} {}", "Статус:".bold(), status_display);

        if self.verbose {
            if let Some(version) = &health.version {
                println!("{} {}", "Версия:".bold(), version);
            }

            if let Some(uptime) = &health.uptime {
                let formatted_uptime = format_uptime(uptime);
                println!("{} {}", "Время работы:".bold(), formatted_uptime);
            }

            self.check_additional_health(client).await?;
        }

        match health.status.to_lowercase().as_str() {
            "ok" | "healthy" => {
                println!("{}", "✓ VictoriaMetrics работает нормально".green());
            }
            "error" | "unhealthy" => {
                println!("{}", "✗ VictoriaMetrics имеет проблемы".red());
            }
            _ => {
                println!("{}", "? Статус VictoriaMetrics неопределен".yellow());
            }
        }

        Ok(())
    }

    async fn check_additional_health(&self, client: &VmClient) -> Result<()> {
        info!("Выполнение дополнительных проверок");

        match client.query("up", None).await {
            Ok(_) => println!("{} {}", "API:".bold(), "Доступен".green()),
            Err(e) => println!("{} {}: {}", "API:".bold(), "Ошибка".red(), e),
        }

        match client.metrics().await {
            Ok(metrics) => {
                println!(
                    "{} {} метрик",
                    "Метрики:".bold(),
                    metrics.data.len().to_string().blue()
                );
            }
            Err(e) => println!("{} {}: {}", "Метрики:".bold(), "Ошибка".red(), e),
        }

        Ok(())
    }
}
