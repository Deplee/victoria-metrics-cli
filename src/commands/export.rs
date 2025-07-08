use crate::api::VmClient;
use crate::error::Result;
use crate::utils::parse_time_range;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;

use tracing::info;

#[derive(Parser)]
pub struct ExportCommand {
    #[arg(value_name = "MATCH")]
    match_: String,

    #[arg(short, long)]
    start: Option<String>,

    #[arg(short, long)]
    end: Option<String>,

    #[arg(short, long)]
    range: Option<String>,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long, value_enum, default_value = "prometheus")]
    format: ExportFormat,

    #[arg(long)]
    progress: bool,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ExportFormat {
    Prometheus,
    Json,
    Csv,
}

impl ExportCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        info!("Экспорт данных: {}", self.match_);

        let (start, end) = self.determine_time_range()?;

        info!("Временной диапазон: {} - {}", start, end);

        let progress_bar = if self.progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {wide_msg}")
                    .unwrap(),
            );
            pb.set_message("Экспорт данных...");
            Some(pb)
        } else {
            None
        };

        let export_data = client.export(&self.match_, Some(&start), Some(&end)).await?;

        if let Some(pb) = &progress_bar {
            pb.finish_with_message("Экспорт завершен");
        }

        let formatted_data = self.format_data(&export_data)?;

        if let Some(output_path) = &self.output {
            self.save_to_file(&formatted_data, output_path)?;
            println!(
                "{} {}",
                "Экспорт сохранен в:".green().bold(),
                output_path
            );
        } else {
            println!("{}", formatted_data);
        }

        Ok(())
    }

    fn determine_time_range(&self) -> Result<(String, String)> {
        if let Some(range) = &self.range {
            parse_time_range(range)
                .map_err(|e| crate::error::VmCliError::TimeParseError(e))
        } else if let (Some(start), Some(end)) = (&self.start, &self.end) {
            Ok((start.clone(), end.clone()))
        } else {
            parse_time_range("1h")
                .map_err(|e| crate::error::VmCliError::TimeParseError(e))
        }
    }

    fn format_data(&self, data: &str) -> Result<String> {
        match self.format {
            ExportFormat::Prometheus => Ok(data.to_string()),
            ExportFormat::Json => {
                let lines: Vec<&str> = data.lines().collect();
                let mut json_data = Vec::new();

                for line in lines {
                    if line.starts_with('#') || line.trim().is_empty() {
                        continue;
                    }

                    if let Some((metric_part, value_part)) = line.rsplit_once(' ') {
                        let mut metric_info = std::collections::HashMap::new();
                        
                        if let Some((name, labels)) = metric_part.split_once('{') {
                            metric_info.insert("__name__".to_string(), name.to_string());
                            
                            if let Some(labels) = labels.strip_suffix('}') {
                                for label in labels.split(',') {
                                    if let Some((key, value)) = label.split_once('=') {
                                        let clean_value = value.trim_matches('"');
                                        metric_info.insert(key.to_string(), clean_value.to_string());
                                    }
                                }
                            }
                        } else {
                            metric_info.insert("__name__".to_string(), metric_part.to_string());
                        }

                        if let Some((timestamp, value)) = value_part.rsplit_once(' ') {
                            let entry = serde_json::json!({
                                "metric": metric_info,
                                "value": [timestamp.parse::<f64>().unwrap_or(0.0), value.parse::<f64>().unwrap_or(0.0)]
                            });
                            json_data.push(entry);
                        }
                    }
                }

                serde_json::to_string_pretty(&json_data)
                    .map_err(|e| crate::error::VmCliError::JsonError(e))
            }
            ExportFormat::Csv => {
                let mut csv_data = String::new();
                csv_data.push_str("timestamp,value,metric_name");
                
                let lines: Vec<&str> = data.lines().collect();
                for line in lines {
                    if line.starts_with('#') || line.trim().is_empty() {
                        continue;
                    }

                    if let Some((metric_part, value_part)) = line.rsplit_once(' ') {
                        let metric_name = if let Some((name, _)) = metric_part.split_once('{') {
                            name
                        } else {
                            metric_part
                        };

                        if let Some((timestamp, value)) = value_part.rsplit_once(' ') {
                            csv_data.push_str(&format!("\n{},{},{}", timestamp, value, metric_name));
                        }
                    }
                }

                Ok(csv_data)
            }
        }
    }

    fn save_to_file(&self, data: &str, path: &str) -> Result<()> {
        let mut file = File::create(path)
            .map_err(|e| crate::error::VmCliError::IoError(e))?;
        
        file.write_all(data.as_bytes())
            .map_err(|e| crate::error::VmCliError::IoError(e))?;
        
        Ok(())
    }
}
