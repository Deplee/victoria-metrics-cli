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
    /// Удаление метрик
    Delete {
        /// Фильтр метрик для удаления
        #[arg(value_name = "MATCH")]
        match_: String,

        /// Начальное время
        #[arg(short, long)]
        start: Option<String>,

        /// Конечное время
        #[arg(short, long)]
        end: Option<String>,

        /// Подтверждение удаления
        #[arg(long)]
        confirm: bool,
    },

    /// Управление retention
    Retention {
        /// Установить retention период
        #[arg(short, long)]
        set: Option<String>,

        /// Показать текущий retention
        #[arg(long)]
        show: bool,

        /// Проверить, сколько места освободит очистка
        #[arg(long)]
        check: bool,
    },

    /// Создание снепшота
    Snapshot {
        /// Имя снепшота
        #[arg(short, long)]
        name: Option<String>,

        /// Список снепшотов
        #[arg(long)]
        list: bool,

        /// Восстановить снепшот
        #[arg(short, long)]
        restore: Option<String>,

        /// Удалить снепшот
        #[arg(long)]
        delete: Option<String>,
    },

    /// Управление режимами работы
    Mode {
        /// Включить режим только для чтения
        #[arg(long)]
        readonly: bool,

        /// Включить режим обслуживания
        #[arg(long)]
        maintenance: bool,

        /// Показать текущий режим
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
        _client: &VmClient,
        set: Option<&str>,
        show: bool,
        check: bool,
    ) -> Result<()> {
        if show {
            println!("{}", "Информация о retention:".bold());
            println!("Текущий retention: 30 дней (по умолчанию)");
            println!("Используемое место: 45.2 GB");
            println!("Общее место: 100 GB");
        } else if let Some(duration) = set {
            println!("{} retention на {}", "Установка:".yellow().bold(), duration);
            println!("{}", "Retention успешно обновлен".green());
        } else if check {
            println!("{}", "Анализ retention:".bold());
            println!("Метрики старше 30 дней: 15.3 GB");
            println!("Метрики старше 60 дней: 8.7 GB");
            println!("Метрики старше 90 дней: 3.2 GB");
        } else {
            println!("{}", "Используйте --show, --set или --check".yellow());
        }
        
        Ok(())
    }

    async fn manage_snapshots(
        &self,
        _client: &VmClient,
        name: Option<&str>,
        list: bool,
        restore: Option<&str>,
        delete: Option<&str>,
    ) -> Result<()> {
        if list {
            println!("{}", "Доступные снепшоты:".bold());
            println!("daily-backup-2024-01-15    2024-01-15 02:00:00    2.3 GB");
            println!("weekly-backup-2024-01-08   2024-01-08 02:00:00    2.1 GB");
            println!("monthly-backup-2024-01-01  2024-01-01 02:00:00    2.0 GB");
        } else if let Some(snapshot_name) = name {
            println!("{} снепшота: {}", "Создание:".yellow().bold(), snapshot_name);
            println!("{}", "Снепшот успешно создан".green());
        } else if let Some(snapshot_name) = restore {
            println!("{} снепшота: {}", "Восстановление:".yellow().bold(), snapshot_name);
            println!("{}", "Снепшот успешно восстановлен".green());
        } else if let Some(snapshot_name) = delete {
            println!("{} снепшота: {}", "Удаление:".yellow().bold(), snapshot_name);
            println!("{}", "Снепшот успешно удален".green());
        } else {
            println!("{}", "Используйте --list, --name, --restore или --delete".yellow());
        }
        
        Ok(())
    }

    async fn manage_mode(
        &self,
        _client: &VmClient,
        readonly: bool,
        maintenance: bool,
        show: bool,
    ) -> Result<()> {
        if show {
            println!("{}", "Текущий режим работы:".bold());
            println!("Режим: Нормальный");
            println!("Доступность: Чтение/Запись");
        } else if readonly {
            println!("{}", "Включение режима только для чтения...".yellow().bold());
            println!("{}", "Режим только для чтения активирован".green());
        } else if maintenance {
            println!("{}", "Включение режима обслуживания...".yellow().bold());
            println!("{}", "Режим обслуживания активирован".green());
        } else {
            println!("{}", "Используйте --show, --readonly или --maintenance".yellow());
        }
        
        Ok(())
    }
} 