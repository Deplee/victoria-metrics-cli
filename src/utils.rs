use crate::api::QueryResponse;
use crate::config::OutputFormat;
use colored::*;

use std::collections::HashMap;
use tabled::{Table, Tabled};

pub fn format_output(data: &QueryResponse, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Json => format_json(data),
        OutputFormat::Table => format_table(data),
        OutputFormat::Csv => format_csv(data),
        OutputFormat::Yaml => format_yaml(data),
    }
}

fn format_json(data: &QueryResponse) -> String {
    serde_json::to_string_pretty(data).unwrap_or_else(|_| "Ошибка форматирования JSON".to_string())
}

fn format_yaml(data: &QueryResponse) -> String {
    serde_yaml::to_string(data).unwrap_or_else(|_| "Ошибка форматирования YAML".to_string())
}

fn format_csv(data: &QueryResponse) -> String {
    let mut csv_data = String::new();
    
    // Заголовок
    if let Some(first_result) = data.data.result.first() {
        let mut headers = vec!["timestamp".to_string(), "value".to_string()];
        headers.extend(first_result.metric.keys().cloned());
        csv_data.push_str(&headers.join(","));
        csv_data.push('\n');
        
        // Данные
        for result in &data.data.result {
            if let Some((timestamp, value)) = &result.value {
                let mut row = vec![timestamp.to_string(), value.to_string()];
                for header in headers.iter().skip(2) {
                    row.push(result.metric.get(header).unwrap_or(&"".to_string()).clone());
                }
                csv_data.push_str(&row.join(","));
                csv_data.push('\n');
            }
        }
    }
    
    csv_data
}

#[derive(Tabled)]
struct MetricRow {
    timestamp: String,
    value: String,
    #[tabled(rename = "labels")]
    labels: String,
}

fn format_table(data: &QueryResponse) -> String {
    let mut rows = Vec::new();
    
    for result in &data.data.result {
        if let Some((timestamp, value)) = &result.value {
            let labels = format_labels(&result.metric);
            rows.push(MetricRow {
                timestamp: timestamp.to_string(),
                value: value.to_string(),
                labels,
            });
        }
    }
    
    if rows.is_empty() {
        return "Нет данных для отображения".yellow().to_string();
    }
    
    Table::new(rows).to_string()
}

fn format_labels(labels: &HashMap<String, String>) -> String {
    let mut formatted = Vec::new();
    for (key, value) in labels {
        formatted.push(format!("{}={}", key, value));
    }
    formatted.join(", ")
}

pub fn format_health_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "ok" | "healthy" => status.green().to_string(),
        "error" | "unhealthy" => status.red().to_string(),
        "warning" => status.yellow().to_string(),
        _ => status.to_string(),
    }
}

pub fn format_uptime(uptime: &str) -> String {
    // Парсинг и форматирование uptime
    if let Ok(seconds) = uptime.parse::<f64>() {
        let days = (seconds / 86400.0) as u64;
        let hours = ((seconds % 86400.0) / 3600.0) as u64;
        let minutes = ((seconds % 3600.0) / 60.0) as u64;
        
        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    } else {
        uptime.to_string()
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

pub fn format_percentage(value: f64, total: f64) -> String {
    if total == 0.0 {
        "0.00%".to_string()
    } else {
        format!("{:.2}%", (value / total) * 100.0)
    }
}

pub fn parse_time_range(range: &str) -> Result<(String, String), String> {
    let now = chrono::Utc::now();
    
    match range {
        "1h" | "1hour" => {
            let start = now - chrono::Duration::hours(1);
            Ok((start.timestamp().to_string(), now.timestamp().to_string()))
        }
        "6h" | "6hours" => {
            let start = now - chrono::Duration::hours(6);
            Ok((start.timestamp().to_string(), now.timestamp().to_string()))
        }
        "24h" | "1d" | "1day" => {
            let start = now - chrono::Duration::days(1);
            Ok((start.timestamp().to_string(), now.timestamp().to_string()))
        }
        "7d" | "7days" => {
            let start = now - chrono::Duration::days(7);
            Ok((start.timestamp().to_string(), now.timestamp().to_string()))
        }
        "30d" | "30days" => {
            let start = now - chrono::Duration::days(30);
            Ok((start.timestamp().to_string(), now.timestamp().to_string()))
        }
        _ => Err(format!("Неизвестный диапазон времени: {}", range)),
    }
}

pub fn validate_promql_query(query: &str) -> Result<(), String> {
    // Простая валидация PromQL запроса
    if query.trim().is_empty() {
        return Err("Запрос не может быть пустым".to_string());
    }
    
    // Проверка на базовые операторы
    let valid_operators = ["+", "-", "*", "/", "%", "==", "!=", ">", "<", ">=", "<="];
    let has_operator = valid_operators.iter().any(|op| query.contains(op));
    
    // Проверка на базовые функции
    let valid_functions = ["sum", "avg", "count", "min", "max", "rate", "increase"];
    let has_function = valid_functions.iter().any(|func| query.contains(func));
    
    // Проверка на простые метрики (например, "up", "node_cpu_seconds_total")
    let is_simple_metric = query.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '{' || c == '}' || c == '=' || c == '"' || c == ',' || c == ' ');
    
    if !has_operator && !has_function && !query.contains('{') && !is_simple_metric {
        return Err("Запрос должен содержать операторы, функции или быть валидной метрикой".to_string());
    }
    
    Ok(())
} 