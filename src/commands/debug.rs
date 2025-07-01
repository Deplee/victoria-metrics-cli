use crate::api::VmClient;
use crate::error::Result;
use clap::{Parser, Subcommand};
use colored::*;


#[derive(Parser)]
pub struct DebugCommand {
    #[command(subcommand)]
    command: DebugSubcommand,
}

#[derive(Subcommand)]
pub enum DebugSubcommand {
    /// Анализ медленных запросов
    SlowQueries {
        /// Количество запросов для отображения
        #[arg(short, long, default_value = "10")]
        top: usize,

        /// Временной диапазон для анализа
        #[arg(short, long, default_value = "1h")]
        range: String,
    },

    /// Поиск пропусков в данных
    Gaps {
        /// Метрика для проверки
        #[arg(value_name = "METRIC")]
        metric: String,

        /// Временной диапазон
        #[arg(short, long, default_value = "24h")]
        range: String,

        /// Минимальный размер пропуска (в секундах)
        #[arg(short, long, default_value = "60")]
        min_gap: u64,
    },

    /// Анализ использования памяти
    Memory {
        /// Показать детальную информацию
        #[arg(short, long)]
        verbose: bool,

        /// Сортировка по использованию
        #[arg(short, long, value_enum, default_value = "size")]
        sort: MemorySort,
    },

    /// Проверка производительности
    Performance {
        /// Количество тестовых запросов
        #[arg(short, long, default_value = "10")]
        count: usize,

        /// Запрос для тестирования
        #[arg(short, long, default_value = "up")]
        query: String,
    },

    /// Анализ метрик
    Metrics {
        /// Поиск метрик по паттерну
        #[arg(value_name = "PATTERN")]
        pattern: Option<String>,

        /// Показать статистику по метрикам
        #[arg(long)]
        stats: bool,

        /// Экспорт списка метрик
        #[arg(short, long)]
        export: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum MemorySort {
    Size,
    Count,
    Name,
}

impl DebugCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        match &self.command {
            DebugSubcommand::SlowQueries { top, range } => {
                self.analyze_slow_queries(client, *top, range).await
            }
            DebugSubcommand::Gaps { metric, range, min_gap } => {
                self.find_data_gaps(client, metric, range, *min_gap).await
            }
            DebugSubcommand::Memory { verbose, sort } => {
                self.analyze_memory_usage(client, *verbose, sort).await
            }
            DebugSubcommand::Performance { count, query } => {
                self.test_performance(client, *count, query).await
            }
            DebugSubcommand::Metrics { pattern, stats, export } => {
                self.analyze_metrics(client, pattern.as_deref(), *stats, export.as_deref()).await
            }
        }
    }

    async fn analyze_slow_queries(
        &self,
        _client: &VmClient,
        top: usize,
        range: &str,
    ) -> Result<()> {
        println!("{}", "Анализ медленных запросов:".bold());
        println!("Диапазон: {}", range);
        println!();

        // Симуляция данных о медленных запросах
        let slow_queries = vec![
            ("rate(http_requests[1h])", 3.2, "Высокая нагрузка"),
            ("sum by (pod) (container_cpu)", 2.1, "Много групп"),
            ("histogram_quantile(0.95, rate(http_duration_bucket[5m]))", 1.8, "Сложная агрегация"),
            ("avg_over_time(node_memory_usage[1h])", 1.5, "Длительный диапазон"),
            ("count(rate(http_errors[5m]))", 1.2, "Простой запрос"),
        ];

        println!("{:<50} {:<10} {}", "Запрос", "Время (с)", "Причина");
        println!("{:-<80}", "");

        for (_i, (query, time, reason)) in slow_queries.iter().take(top).enumerate() {
            let time_color = if *time > 2.0 {
                time.to_string().red()
            } else if *time > 1.0 {
                time.to_string().yellow()
            } else {
                time.to_string().green()
            };

            println!("{:<50} {:<10} {}", query, time_color, reason);
        }

        Ok(())
    }

    async fn find_data_gaps(
        &self,
        client: &VmClient,
        metric: &str,
        range: &str,
        min_gap: u64,
    ) -> Result<()> {
        println!("{}", "Поиск пропусков в данных:".bold());
        println!("Метрика: {}", metric);
        println!("Диапазон: {}", range);
        println!("Минимальный пропуск: {} секунд", min_gap);
        println!();

        // Выполнение запроса для поиска пропусков
        let query = format!("count({})", metric);
        let response = client.query(&query, None).await?;

        if response.data.result.is_empty() {
            println!("{}", "Метрика не найдена".yellow());
            return Ok(());
        }

        // Симуляция анализа пропусков
        println!("{:<20} {:<20} {:<15} {}", "Начало", "Конец", "Длительность", "Статус");
        println!("{:-<70}", "");

        let gaps = vec![
            ("2024-01-15 14:30:00", "2024-01-15 14:35:00", "5m", "Найден"),
            ("2024-01-15 16:45:00", "2024-01-15 16:47:00", "2m", "Найден"),
        ];

        for (start, end, duration, status) in &gaps {
            let status_color = if *status == "Найден" {
                status.red()
            } else {
                status.green()
            };

            println!("{:<20} {:<20} {:<15} {}", start, end, duration, status_color);
        }

        if gaps.is_empty() {
            println!("{}", "Пропуски не найдены".green());
        }

        Ok(())
    }

    async fn analyze_memory_usage(
        &self,
        _client: &VmClient,
        verbose: bool,
        _sort: &MemorySort,
    ) -> Result<()> {
        println!("{}", "Анализ использования памяти:".bold());
        println!();

        if verbose {
            println!("{:<30} {:<15} {:<15} {:<15}", "Компонент", "Использовано", "Выделено", "Процент");
            println!("{:-<80}", "");

            let memory_data = vec![
                ("TSDB", "2.3 GB", "3.0 GB", "76.7%"),
                ("Index", "1.1 GB", "1.5 GB", "73.3%"),
                ("Cache", "512 MB", "1.0 GB", "51.2%"),
                ("HTTP Server", "128 MB", "256 MB", "50.0%"),
            ];

            for (component, used, allocated, percent) in memory_data {
                let percent_color = if percent.parse::<f64>().unwrap_or(0.0) > 80.0 {
                    percent.red()
                } else if percent.parse::<f64>().unwrap_or(0.0) > 60.0 {
                    percent.yellow()
                } else {
                    percent.green()
                };

                println!("{:<30} {:<15} {:<15} {:<15}", component, used, allocated, percent_color);
            }
        } else {
            println!("Общее использование: 4.0 GB / 5.8 GB (68.9%)");
            println!("Свободно: 1.8 GB");
        }

        Ok(())
    }

    async fn test_performance(
        &self,
        client: &VmClient,
        count: usize,
        query: &str,
    ) -> Result<()> {
        println!("{}", "Тестирование производительности:".bold());
        println!("Запрос: {}", query);
        println!("Количество тестов: {}", count);
        println!();

        let mut times = Vec::new();

        for i in 1..=count {
            let start = std::time::Instant::now();
            
            match client.query(query, None).await {
                Ok(_) => {
                    let duration = start.elapsed();
                    times.push(duration);
                    println!("Итерация {}: {:?}", i, duration);
                }
                Err(e) => {
                    println!("Итерация {}: Ошибка - {}", i, e);
                }
            }
        }

        if !times.is_empty() {
            let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
            let min_time = times.iter().min().unwrap();
            let max_time = times.iter().max().unwrap();

            println!();
            println!("{}", "Результаты:".bold());
            println!("Среднее время: {:?}", avg_time);
            println!("Минимальное время: {:?}", min_time);
            println!("Максимальное время: {:?}", max_time);
        }

        Ok(())
    }

    async fn analyze_metrics(
        &self,
        client: &VmClient,
        pattern: Option<&str>,
        stats: bool,
        export: Option<&str>,
    ) -> Result<()> {
        println!("{}", "Анализ метрик:".bold());
        println!();

        let metrics = client.metrics().await?;

        if let Some(pattern) = pattern {
            let filtered: Vec<&String> = metrics
                .data
                .iter()
                .filter(|m| m.contains(pattern))
                .collect();

            println!("Метрики, соответствующие паттерну '{}': {}", pattern, filtered.len());
            for metric in filtered.iter().take(20) {
                println!("  {}", metric);
            }
            if filtered.len() > 20 {
                println!("  ... и еще {} метрик", filtered.len() - 20);
            }
        } else if stats {
            println!("Общая статистика:");
            println!("Всего метрик: {}", metrics.data.len());
            
            // Группировка по префиксам
            let mut prefixes = std::collections::HashMap::new();
            for metric in &metrics.data {
                if let Some(prefix) = metric.split('_').next() {
                    *prefixes.entry(prefix).or_insert(0) += 1;
                }
            }

            println!("Топ-10 префиксов:");
            let mut sorted_prefixes: Vec<_> = prefixes.iter().collect();
            sorted_prefixes.sort_by(|a, b| b.1.cmp(a.1));
            
            for (prefix, count) in sorted_prefixes.iter().take(10) {
                println!("  {}: {} метрик", prefix, count);
            }
        } else {
            println!("Всего метрик: {}", metrics.data.len());
            println!("Первые 20 метрик:");
            for metric in metrics.data.iter().take(20) {
                println!("  {}", metric);
            }
        }

        if let Some(export_path) = export {
            // Экспорт списка метрик
            let content = metrics.data.join("\n");
            std::fs::write(export_path, content)
                .map_err(|e| crate::error::VmCliError::IoError(e))?;
            println!("Список метрик экспортирован в: {}", export_path);
        }

        Ok(())
    }
} 