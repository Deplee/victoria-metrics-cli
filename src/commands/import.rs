use crate::api::VmClient;
use crate::error::Result;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use tracing::info;

#[derive(Parser)]
pub struct ImportCommand {
    #[arg(value_name = "FILE")]
    file: String,

    #[arg(short, long, value_enum, default_value = "prometheus")]
    format: ImportFormat,

    #[arg(long)]
    progress: bool,
    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    skip_errors: bool,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ImportFormat {
    Prometheus,
    Json,
    Csv,
}

impl ImportCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        info!("Импорт данных из файла: {}", self.file);

        if !std::path::Path::new(&self.file).exists() {
            return Err(crate::error::VmCliError::FileNotFound(self.file.clone()));
        }

        let file_content = fs::read_to_string(&self.file)
            .map_err(|e| crate::error::VmCliError::IoError(e))?;

        info!("Размер файла: {} байт", file_content.len());

        let progress_bar = if self.progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {wide_msg}")
                    .unwrap(),
            );
            pb.set_message("Импорт данных...");
            Some(pb)
        } else {
            None
        };

        let import_data = self.prepare_data(&file_content)?;

        if self.dry_run {
            println!("{}", "Режим проверки (dry-run)".yellow().bold());
            println!("{} строк данных готово к импорту", import_data.lines().count());
            return Ok(());
        }

        client.import(&import_data).await?;

        if let Some(pb) = &progress_bar {
            pb.finish_with_message("Импорт завершен");
        }

        println!(
            "{} {}",
            "Импорт успешно завершен:".green().bold(),
            self.file
        );

        Ok(())
    }

    fn prepare_data(&self, content: &str) -> Result<String> {
        match self.format {
            ImportFormat::Prometheus => {
                self.validate_prometheus_format(content)?;
                Ok(content.to_string())
            }
            ImportFormat::Json => {
                self.convert_json_to_prometheus(content)
            }
            ImportFormat::Csv => {
                self.convert_csv_to_prometheus(content)
            }
        }
    }

    fn validate_prometheus_format(&self, content: &str) -> Result<()> {
        let mut line_count = 0;
        let mut error_count = 0;

        for (line_num, line) in content.lines().enumerate() {
            line_count += 1;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if !self.is_valid_prometheus_line(line) {
                error_count += 1;
                if !self.skip_errors {
                    return Err(crate::error::VmCliError::InvalidQuery(format!(
                        "Неверный формат на строке {}: {}",
                        line_num + 1,
                        line
                    )));
                } else {
                    eprintln!(
                        "{} Строка {}: {}",
                        "ПРЕДУПРЕЖДЕНИЕ:".yellow(),
                        line_num + 1,
                        line
                    );
                }
            }
        }

        if error_count > 0 {
            println!(
                "{} {} ошибок из {} строк",
                "Найдено:".yellow().bold(),
                error_count,
                line_count
            );
        }

        Ok(())
    }

    fn is_valid_prometheus_line(&self, line: &str) -> bool {
        if let Some((metric_part, value_part)) = line.rsplit_once(' ') {
            if metric_part.contains('{') {
                if !metric_part.contains('}') {
                    return false;
                }
            }

            if let Some((timestamp, value)) = value_part.rsplit_once(' ') {
                timestamp.parse::<f64>().is_ok() && value.parse::<f64>().is_ok()
            } else {
                value_part.parse::<f64>().is_ok()
            }
        } else {
            false
        }
    }

    fn convert_json_to_prometheus(&self, content: &str) -> Result<String> {
        let json_data: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| crate::error::VmCliError::JsonError(e))?;

        let mut prometheus_data = String::new();

        if let Some(array) = json_data.as_array() {
            for item in array {
                if let (Some(metric), Some(value)) = (item.get("metric"), item.get("value")) {
                    if let (Some(metric_obj), Some(value_array)) = (metric.as_object(), value.as_array()) {
                        if value_array.len() == 2 {
                            if let (Some(timestamp), Some(value)) = (value_array[0].as_f64(), value_array[1].as_f64()) {
                                let mut metric_str = String::new();
                                
                                if let Some(name) = metric_obj.get("__name__") {
                                    metric_str.push_str(&name.as_str().unwrap_or("unknown"));
                                }

                                let labels: Vec<String> = metric_obj
                                    .iter()
                                    .filter(|(k, _)| *k != "__name__")
                                    .map(|(k, v)| format!("{}=\"{}\"", k, v.as_str().unwrap_or("")))
                                    .collect();

                                if !labels.is_empty() {
                                    metric_str.push('{');
                                    metric_str.push_str(&labels.join(","));
                                    metric_str.push('}');
                                }

                                prometheus_data.push_str(&format!("{} {} {}\n", metric_str, value, timestamp as i64));
                            }
                        }
                    }
                }
            }
        }

        Ok(prometheus_data)
    }

    fn convert_csv_to_prometheus(&self, content: &str) -> Result<String> {
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let mut prometheus_data = String::new();

        for result in reader.records() {
            let record = result.map_err(|e| crate::error::VmCliError::CsvError(e))?;
            
            if record.len() >= 3 {
                let timestamp = record.get(0).unwrap_or("");
                let value = record.get(1).unwrap_or("");
                let metric_name = record.get(2).unwrap_or("");

                if let (Ok(ts), Ok(val)) = (timestamp.parse::<f64>(), value.parse::<f64>()) {
                    prometheus_data.push_str(&format!("{} {} {}\n", metric_name, val, ts as i64));
                }
            }
        }

        Ok(prometheus_data)
    }
}
