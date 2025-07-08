use crate::api::VmClient;
use crate::error::Result;
use clap::{Parser, Subcommand};
use colored::*;
use tracing::info;

#[derive(Parser)]
pub struct AdminCommand {
    #[command(subcommand)]
    command: AdminSubcommand,
}

#[derive(Subcommand)]
pub enum AdminSubcommand {
    Delete {
        #[arg(value_name = "MATCH")]
        match_: String,

        #[arg(short, long)]
        start: Option<String>,

        #[arg(short, long)]
        end: Option<String>,

        #[arg(long)]
        confirm: bool,
    },

    Retention {
        #[arg(short, long)]
        set: Option<String>,

        #[arg(long)]
        show: bool,

        #[arg(long)]
        check: bool,
    },

    Snapshot {
        #[arg(short, long)]
        name: Option<String>,

        #[arg(long)]
        list: bool,

        #[arg(short, long)]
        restore: Option<String>,

        #[arg(long)]
        delete: Option<String>,
    },

    Mode {
        #[arg(long)]
        readonly: bool,

        #[arg(long)]
        maintenance: bool,

        #[arg(long)]
        show: bool,
    },
}

impl AdminCommand {
    pub async fn execute(&self, client: &VmClient) -> Result<()> {
        match &self.command {
            AdminSubcommand::Delete { match_, start, end, confirm } => {
                self.delete_metrics(client, match_, start.as_deref(), end.as_deref(), *confirm).await
            }
            AdminSubcommand::Retention { set, show, check } => {
                self.manage_retention(client, set.as_deref(), *show, *check).await
            }
            AdminSubcommand::Snapshot { name, list, restore, delete } => {
                self.manage_snapshots(client, name.as_deref(), *list, restore.as_deref(), delete.as_deref()).await
            }
            AdminSubcommand::Mode { readonly, maintenance, show } => {
                self.manage_mode(client, *readonly, *maintenance, *show).await
            }
        }
    }

    async fn delete_metrics(
        &self,
        client: &VmClient,
        match_: &str,
        start: Option<&str>,
        end: Option<&str>,
        confirm: bool,
    ) -> Result<()> {
        info!("Удаление метрик: {}", match_);

        if !confirm {
            println!("{}", "ВНИМАНИЕ: Это действие необратимо!".red().bold());
            println!("Метрики, соответствующие фильтру '{}', будут удалены.", match_);
            
            if let Some(start_time) = start {
                println!("Начальное время: {}", start_time);
            }
            if let Some(end_time) = end {
                println!("Конечное время: {}", end_time);
            }
            
            println!("Для подтверждения используйте флаг --confirm");
            return Ok(());
        }

        println!("{}", "Удаление метрик...".yellow());
        client.delete_series(match_, start, end).await?;
        
        println!("{}", "Метрики успешно удалены".green().bold());
        Ok(())
    }

    async fn manage_retention(
        &self,
        client: &VmClient,
        set: Option<&str>,
        show: bool,
        check: bool,
    ) -> Result<()> {
        if show {
            println!("{}", "Информация о retention:".bold());
            match client.get_retention_info().await {
                Ok(info) => {
                    println!("Текущий retention: {}", info.current_retention);
                    println!("Используемое место: {}", info.used_space);
                    println!("Общее место: {}", info.total_space);
                }
                Err(e) => {
                    println!("{}", "Ошибка получения информации о retention:".red().bold());
                    println!("{}", e);
                }
            }
        } else if let Some(duration) = set {
            println!("{} retention на {}", "Установка:".yellow().bold(), duration);
            match client.set_retention(duration).await {
                Ok(_) => println!("{}", "Retention успешно обновлен".green()),
                Err(e) => {
                    println!("{}", "Ошибка установки retention:".red().bold());
                    println!("{}", e);
                }
            }
        } else if check {
            println!("{}", "Анализ retention:".bold());
            match client.get_retention_info().await {
                Ok(info) => {
                    if let Some(size_30d) = info.metrics_older_than_30d {
                        println!("Метрики старше 30 дней: {}", size_30d);
                    }
                    if let Some(size_60d) = info.metrics_older_than_60d {
                        println!("Метрики старше 60 дней: {}", size_60d);
                    }
                    if let Some(size_90d) = info.metrics_older_than_90d {
                        println!("Метрики старше 90 дней: {}", size_90d);
                    }
                }
                Err(e) => {
                    println!("{}", "Ошибка анализа retention:".red().bold());
                    println!("{}", e);
                }
            }
        } else {
            println!("{}", "Используйте --show, --set или --check".yellow());
        }
        
        Ok(())
    }

    async fn manage_snapshots(
        &self,
        client: &VmClient,
        name: Option<&str>,
        list: bool,
        restore: Option<&str>,
        delete: Option<&str>,
    ) -> Result<()> {
        if list {
            println!("{}", "Доступные снепшоты:".bold());
            match client.list_snapshots().await {
                Ok(snapshots) => {
                    if snapshots.is_empty() {
                        println!("Снепшоты не найдены");
                    } else {
                        println!("{:<30} {:<20} {:<10} {}", 
                            "Имя".bold(), 
                            "Создан".bold(), 
                            "Размер".bold(), 
                            "Статус".bold());
                        println!("{}", "-".repeat(80));
                        for snapshot in snapshots {
                            println!("{:<30} {:<20} {:<10} {}", 
                                snapshot.name, 
                                snapshot.created_at, 
                                snapshot.size, 
                                snapshot.status);
                        }
                    }
                }
                Err(e) => {
                    println!("{}", "Ошибка получения списка снепшотов:".red().bold());
                    println!("{}", e);
                }
            }
        } else if let Some(snapshot_name) = name {
            println!("{} снепшота: {}", "Создание:".yellow().bold(), snapshot_name);
            match client.create_snapshot(snapshot_name).await {
                Ok(snapshot_id) => {
                    println!("{}", "Снепшот успешно создан".green());
                    println!("ID снепшота: {}", snapshot_id);
                }
                Err(e) => {
                    println!("{}", "Ошибка создания снепшота:".red().bold());
                    println!("{}", e);
                }
            }
        } else if let Some(snapshot_name) = restore {
            println!("{} снепшота: {}", "Восстановление:".yellow().bold(), snapshot_name);
            match client.restore_snapshot(snapshot_name).await {
                Ok(_) => println!("{}", "Снепшот успешно восстановлен".green()),
                Err(e) => {
                    println!("{}", "Ошибка восстановления снепшота:".red().bold());
                    println!("{}", e);
                }
            }
        } else if let Some(snapshot_name) = delete {
            println!("{} снепшота: {}", "Удаление:".yellow().bold(), snapshot_name);
            match client.delete_snapshot(snapshot_name).await {
                Ok(_) => println!("{}", "Снепшот успешно удален".green()),
                Err(e) => {
                    println!("{}", "Ошибка удаления снепшота:".red().bold());
                    println!("{}", e);
                }
            }
        } else {
            println!("{}", "Используйте --list, --name, --restore или --delete".yellow());
        }
        
        Ok(())
    }

    async fn manage_mode(
        &self,
        client: &VmClient,
        readonly: bool,
        maintenance: bool,
        show: bool,
    ) -> Result<()> {
        if show {
            println!("{}", "Информация о VictoriaMetrics:".bold());
            println!();
            
            println!("{}", "Флаги запуска:".bold());
            match client.get_flags().await {
                Ok(flags_data) => {
                    if let Some(flags_obj) = flags_data.as_object() {
                        for (key, value) in flags_obj {
                            println!("  {}: {}", key, value);
                        }
                    } else {
                        println!("  {}", flags_data);
                    }
                }
                Err(e) => {
                    println!("  {}: {}", "Ошибка получения флагов".red(), e);
                }
            }
            
            println!();
            println!("{}", "Информация о сборке:".bold());
            match client.get_build_info().await {
                Ok(build_data) => {
                    if let Some(data) = build_data.get("data") {
                        if let Some(result) = data.as_array().and_then(|arr| arr.first()) {
                            if let Some(result_obj) = result.as_object() {
                                for (key, value) in result_obj {
                                    println!("  {}: {}", key, value);
                                }
                            }
                        }
                    } else {
                        println!("  {}", build_data);
                    }
                }
                Err(e) => {
                    println!("  {}: {}", "Ошибка получения информации о сборке".red(), e);
                }
            }
        } else if readonly {
            println!("{}", "Включение режима только для чтения...".yellow().bold());
            println!("{}", "Для включения режима только для чтения используйте флаг -readonly при запуске VictoriaMetrics".yellow());
            println!("Пример: ./victoria-metrics -readonly");
        } else if maintenance {
            println!("{}", "Включение режима обслуживания...".yellow().bold());
            println!("{}", "Для включения режима обслуживания используйте флаг -maintenance при запуске VictoriaMetrics".yellow());
            println!("Пример: ./victoria-metrics -maintenance");
        } else {
            println!("{}", "Используйте --show, --readonly или --maintenance".yellow());
        }
        
        Ok(())
    }
}
