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
    SlowQueries {
        #[arg(short, long, default_value = "10")]
        top: usize,
        #[arg(short, long, default_value = "1h")]
        range: String,
    },

    Gaps {
        #[arg(value_name = "METRIC")]
        metric: String,
        #[arg(short, long, default_value = "24h")]
        range: String,

        #[arg(short, long, default_value = "60")]
        min_gap: u64,
    },

    Memory {
        #[arg(short, long)]
        verbose: bool,

        #[arg(short, long, value_enum, default_value = "size")]
        sort: MemorySort,
    },

    Performance {
        #[arg(short, long, default_value = "10")]
        count: usize,

        #[arg(short, long, default_value = "up")]
        query: String,
    },

    Metrics {
        #[arg(value_name = "PATTERN")]
        pattern: Option<String>,

        #[arg(long)]
        stats: bool,

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
        client: &VmClient,
        top: usize,
        range: &str,
    ) -> Result<()> {
        println!("{}", "Анализ медленных запросов:".bold());
        println!("Диапазон: {}", range);
        println!();

        match client.get_slow_queries().await {
            Ok(slow_queries) => {
                if slow_queries.is_empty() {
                    println!("{}", "Медленные запросы не обнаружены".green());
                    return Ok(());
                }

                println!("{:<30} {:<10} {}", "Тип проблемы", "Время (с)", "Причина");
                println!("{:-<60}", "");

                for query_info in slow_queries.iter().take(top) {
                    let time_color = if query_info.duration > 2.0 {
                        format!("{:.2}", query_info.duration).red()
                    } else if query_info.duration > 1.0 {
                        format!("{:.2}", query_info.duration).yellow()
                    } else {
                        format!("{:.2}", query_info.duration).green()
                    };

                    println!("{:<30} {:<10} {}", 
                        query_info.query, 
                        time_color, 
                        query_info.reason);
                }
            }
            Err(e) => {
                println!("{}", "Ошибка получения данных о медленных запросах:".red().bold());
                println!("{}", e);
            }
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

        let query = format!("count({})", metric);
        let response = client.query(&query, None).await?;

        if response.data.result.is_empty() {
            println!("{}", "Метрика не найдена".yellow());
            return Ok(());
        }

        println!("{:<20} {:<20} {:<15} {}", "Начало", "Конец", "Длительность", "Статус");
        println!("{:-<70}", "");

        let step = "60s";
        let end_time = chrono::Utc::now();
        let start_time = end_time - chrono::Duration::hours(24);
        
        let start_str = start_time.timestamp().to_string();
        let end_str = end_time.timestamp().to_string();
        
        let range_query = format!("{}", metric);
        match client.query_range(&range_query, &start_str, &end_str, step).await {
            Ok(range_response) => {
                let mut gaps = Vec::new();
                
                for result in &range_response.data.result {
                    if let Some(values) = &result.values {
                        if values.len() > 1 {
                            for i in 1..values.len() {
                                let prev_time = values[i-1].0;
                                let curr_time = values[i].0;
                                let gap_duration = curr_time - prev_time;
                                
                                if gap_duration > min_gap as f64 {
                                    let start_dt = chrono::DateTime::from_timestamp(prev_time as i64, 0)
                                        .unwrap_or_default()
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string();
                                    let end_dt = chrono::DateTime::from_timestamp(curr_time as i64, 0)
                                        .unwrap_or_default()
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string();
                                    
                                    let duration_str = if gap_duration > 3600.0 {
                                        format!("{:.0}h", gap_duration / 3600.0)
                                    } else if gap_duration > 60.0 {
                                        format!("{:.0}m", gap_duration / 60.0)
                                    } else {
                                        format!("{:.0}s", gap_duration)
                                    };
                                    
                                    gaps.push((start_dt, end_dt, duration_str, "Найден".to_string()));
                                }
                            }
                        }
                    }
                }
                
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
            }
            Err(e) => {
                println!("{}", "Ошибка при поиске пропусков:".red().bold());
                println!("{}", e);
            }
        }

        Ok(())
    }

    async fn analyze_memory_usage(
        &self,
        client: &VmClient,
        verbose: bool,
        _sort: &MemorySort,
    ) -> Result<()> {
        println!("{}", "Анализ использования памяти:".bold());
        println!();

        match client.get_metrics_info().await {
            Ok(metrics) => {
                if let Some(metrics_obj) = metrics.as_object() {
                    if verbose {
                        println!("{:<30} {:<15} {:<15} {:<15}", "Компонент", "Использовано", "Выделено", "Процент");
                        println!("{:-<80}", "");

                        let mut memory_data = Vec::new();
                        
                        if let Some(process_resident_memory_bytes) = metrics_obj.get("process_resident_memory_bytes") {
                            if let Some(used_mb) = process_resident_memory_bytes.as_f64() {
                                let used_gb = used_mb / 1_000_000_000.0;
                                memory_data.push(("Процесс", format!("{:.2} GB", used_gb), "N/A".to_string(), "N/A".to_string()));
                            }
                        }
                        
                        if let Some(vm_cache_size_bytes) = metrics_obj.get("vm_cache_size_bytes") {
                            if let Some(cache_mb) = vm_cache_size_bytes.as_f64() {
                                let cache_gb = cache_mb / 1_000_000_000.0;
                                memory_data.push(("Кэш", format!("{:.2} GB", cache_gb), "N/A".to_string(), "N/A".to_string()));
                            }
                        }
                        
                        if let Some(go_memstats_heap_alloc_bytes) = metrics_obj.get("go_memstats_heap_alloc_bytes") {
                            if let Some(heap_mb) = go_memstats_heap_alloc_bytes.as_f64() {
                                let heap_gb = heap_mb / 1_000_000_000.0;
                                memory_data.push(("Heap", format!("{:.2} GB", heap_gb), "N/A".to_string(), "N/A".to_string()));
                            }
                        }

                        for (component, used, allocated, percent) in memory_data {
                            println!("{:<30} {:<15} {:<15} {:<15}", component, used, allocated, percent);
                        }
                    } else {
                        let mut total_used = 0.0;
                        
                        if let Some(process_resident_memory_bytes) = metrics_obj.get("process_resident_memory_bytes") {
                            if let Some(used_mb) = process_resident_memory_bytes.as_f64() {
                                total_used = used_mb / 1_000_000_000.0;
                            }
                        }
                        
                        println!("Использование памяти процессом: {:.2} GB", total_used);
                        
                        if let Some(vm_cache_size_bytes) = metrics_obj.get("vm_cache_size_bytes") {
                            if let Some(cache_mb) = vm_cache_size_bytes.as_f64() {
                                let cache_gb = cache_mb / 1_000_000_000.0;
                                println!("Размер кэша: {:.2} GB", cache_gb);
                            }
                        }
                    }
                } else {
                    println!("{}", "Не удалось получить метрики памяти".yellow());
                }
            }
            Err(e) => {
                println!("{}", "Ошибка получения метрик памяти:".red().bold());
                println!("{}", e);
            }
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
            let content = metrics.data.join("\n");
            std::fs::write(export_path, content)
                .map_err(|e| crate::error::VmCliError::IoError(e))?;
            println!("Список метрик экспортирован в: {}", export_path);
        }

        Ok(())
    }
}
