use crate::api::VmClient;
use crate::config::OutputFormat;
use crate::error::Result;
use crate::utils::{format_output, parse_time_range, validate_promql_query};
use clap::Parser;
use colored::*;
use tracing::info;

#[derive(Parser)]
pub struct QueryCommand {
    /// PromQL запрос
    #[arg(value_name = "QUERY")]
    query: String,

    /// Время выполнения запроса (Unix timestamp или RFC3339)
    #[arg(short, long)]
    time: Option<String>,

    /// Диапазон времени для запроса (например: 1h, 6h, 1d, 7d)
    #[arg(short, long)]
    range: Option<String>,

    /// Шаг для range запросов
    #[arg(short, long, default_value = "1m")]
    step: String,

    /// Формат вывода
    #[arg(short, long, value_enum, default_value = "table")]
    format: OutputFormat,

    /// Показать только количество результатов
    #[arg(long)]
    count: bool,

    /// Показать только метрики без значений
    #[arg(long)]
    metrics_only: bool,
}

impl QueryCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        info!("Выполнение запроса: {}", self.query);

        // Валидация запроса
        validate_promql_query(&self.query)
            .map_err(|e| crate::error::VmCliError::InvalidQuery(e))?;

        let response = if let Some(range) = &self.range {
            // Range запрос
            let (start, end) = parse_time_range(range)
                .map_err(|e| crate::error::VmCliError::TimeParseError(e))?;
            
            info!("Range запрос: {} - {}", start, end);
            client.query_range(&self.query, &start, &end, &self.step).await?
        } else {
            // Мгновенный запрос
            client.query(&self.query, self.time.as_deref()).await?
        };

        // Обработка результатов
        if self.count {
            println!("{}", response.data.result.len());
            return Ok(());
        }

        if self.metrics_only {
            for result in &response.data.result {
                for (key, value) in &result.metric {
                    if key != "__name__" {
                        println!("{}={}", key, value);
                    }
                }
            }
            return Ok(());
        }

        // Форматированный вывод
        let output = format_output(&response, &self.format);
        println!("{}", output);

        // Дополнительная информация
        if self.format == OutputFormat::Table {
            println!(
                "\n{} {} результатов",
                "Найдено:".blue().bold(),
                response.data.result.len()
            );
        }

        Ok(())
    }
} 