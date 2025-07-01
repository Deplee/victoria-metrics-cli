# VM-CLI: CLI инструмент для VictoriaMetrics

Универсальный командный интерфейс для работы с VictoriaMetrics (включая кластерную версию), написанный на Rust. Предоставляет удобные команды для мониторинга, администрирования, анализа данных и отладки.

## 🚀 Возможности

- **Поддержка кластерной версии** - работа с vmselect, vminsert, vmstorage
- **Запросы метрик** - выполнение PromQL запросов с красивым форматированием
- **Проверка здоровья** - мониторинг состояния кластера VictoriaMetrics
- **Экспорт/Импорт** - работа с данными в различных форматах (Prometheus, JSON, CSV)
- **Администрирование** - управление retention, снепшотами, режимами работы
- **Отладка** - анализ производительности, поиск проблем, диагностика
- **Цветной вывод** - удобное отображение результатов с цветовой индикацией
- **Гибкая конфигурация** - поддержка файлов конфигурации и переменных окружения



## 📦 Установка

### Требования

- Rust 1.70+ 
- VictoriaMetrics (standalone или кластерная версия)
- Для кластерной версии: vmselect (порт 8481), vminsert (порт 8480), vmstorage (порт 8482)

### Из исходного кода

```bash
git clone https://github.com/Deplee/victoria-metrics-cli.git
cd vm-cli
cargo build --release
cargo install --path .
```

### Из релизов

Скачайте последний релиз для вашей платформы с [GitHub Releases](https://github.com/Deplee/victoria-metrics-cli/releases).

## 🔧 Конфигурация

### Standalone vs Кластерная версия

| Функция | Standalone | Кластерная |
|---------|------------|------------|
| **Порт** | 8428 | 8481 (vmselect) |
| **Endpoint** | `/api/v1/query` | `/select/{accountID}/prometheus/api/v1/query` |
| **Аутентификация** | Опционально | Требуется authToken |
| **Масштабируемость** | Ограничена | Высокая |
| **Отказоустойчивость** | Нет | Да |

### Переменные окружения

```bash
export VM_HOST="http://localhost:8481"  # vmselect для кластерной версии
export VM_TIMEOUT="30"
export VM_VERBOSE="true"
```

### Файл конфигурации

#### Для standalone версии

Создайте файл `~/.config/vm-cli/config.toml`:

```toml
host = "http://localhost:8428"
timeout = 30

[output]
format = "table"
color = true
pretty = true

[auth]
username = "admin"
password = "secret"
```

#### Для кластерной версии

Создайте файл `vm-cluster.toml`:

```toml
# Конфигурация для кластерной версии VictoriaMetrics
# vmselect (порт 8481) - для запросов
host = "http://your-cluster:8481"
timeout = 30

# Настройки кластера
[cluster]
# Основные endpoints для vmselect
query_endpoint = "/api/v1/query"
query_range_endpoint = "/api/v1/query_range"
health_endpoint = "/health"
metrics_endpoint = "/api/v1/label/__name__/values"

# Настройки для кластерной версии
use_select_endpoint = true
select_account_id = "0"
select_project_id = "0"

# Дополнительные хосты для кластерной архитектуры
vminsert_host = "http://your-cluster:8480"  # vminsert для записи данных
vmstorage_host = "http://your-cluster:8482"  # vmstorage для администрирования

# Настройки вывода
[output]
format = "table"
color = true
pretty = true

# Настройки логирования
[logging]
level = "info"

# Настройки экспорта
[export]
default_format = "prometheus"
chunk_size = 1000
```

## 📖 Использование

### Основные команды

```bash
# Использование с конфигурационным файлом
vm-cli --config vm-cluster.toml health

# Проверка здоровья
vm-cli --config vm-cluster.toml health

# Выполнение запроса
vm-cli --config vm-cluster.toml query 'sum(rate(http_requests_total[5m])) by (service)'

# Range запрос
vm-cli --config vm-cluster.toml query 'node_cpu_seconds_total' --range '1h' --step '1m'

# Экспорт данных
vm-cli --config vm-cluster.toml export 'http_requests_total' --range '24h' --output data.json

# Импорт данных
vm-cli --config vm-cluster.toml import data.json --format prometheus
```

### Запросы (Query)

```bash
# Простой запрос
vm-cli --config vm-cluster.toml query 'up'

# Запрос с фильтрацией по меткам (используйте кавычки для значений с дефисами)
vm-cli --config vm-cluster.toml query '{instance="fqdn:port"}'

# Запрос с временной меткой
vm-cli --config vm-cluster.toml query 'node_cpu_seconds_total' --time '2024-01-15T10:00:00Z'

# Range запрос
vm-cli --config vm-cluster.toml query 'rate(http_requests_total[5m])' --range '1h' --step '30s'

# Различные форматы вывода
vm-cli --config vm-cluster.toml query 'up' --format json
vm-cli --config vm-cluster.toml query 'up' --format csv
vm-cli --config vm-cluster.toml query 'up' --format yaml

# Только количество результатов
vm-cli --config vm-cluster.toml query 'up' --count

# Только метрики без значений
vm-cli --config vm-cluster.toml query 'up' --metrics-only
```

### Проверка здоровья (Health)

```bash
# Базовая проверка
vm-cli --config vm-cluster.toml health

# Детальная информация
vm-cli --config vm-cluster.toml health --verbose

# Только статус (для скриптов)
vm-cli --config vm-cluster.toml health --status-only
```

### Экспорт (Export)

```bash
# Экспорт в файл
vm-cli --config vm-cluster.toml export 'http_requests_total' --output data.txt

# Экспорт с временным диапазоном
vm-cli --config vm-cluster.toml export 'node_cpu_seconds_total' --range '7d' --output cpu_data.txt

# Экспорт в JSON формате
vm-cli --config vm-cluster.toml export 'up' --format json --output data.json

# Экспорт в CSV
vm-cli --config vm-cluster.toml export 'http_requests_total' --format csv --output data.csv

# С индикатором прогресса
vm-cli --config vm-cluster.toml export 'large_metric' --progress
```

### Импорт (Import)

```bash
# Импорт Prometheus формата
vm-cli --config vm-cluster.toml import data.txt

# Импорт JSON
vm-cli --config vm-cluster.toml import data.json --format json

# Импорт CSV
vm-cli --config vm-cluster.toml import data.csv --format csv

# Проверка без импорта
vm-cli --config vm-cluster.toml import data.txt --dry-run

# Пропуск ошибок
vm-cli --config vm-cluster.toml import data.txt --skip-errors
```

### Администрирование (Admin)

```bash
# Удаление метрик
vm-cli --config vm-cluster.toml admin delete 'old_metric_*' --start '2023-01-01' --end '2023-12-31' --confirm

# Управление retention
vm-cli --config vm-cluster.toml admin retention --show
vm-cli --config vm-cluster.toml admin retention --set '365d'
vm-cli --config vm-cluster.toml admin retention --check

# Снепшоты
vm-cli --config vm-cluster.toml admin snapshot --list
vm-cli --config vm-cluster.toml admin snapshot --name 'daily-backup'
vm-cli --config vm-cluster.toml admin snapshot --restore 'daily-backup'

# Режимы работы
vm-cli --config vm-cluster.toml admin mode --show
vm-cli --config vm-cluster.toml admin mode --readonly
vm-cli --config vm-cluster.toml admin mode --maintenance
```

### Отладка (Debug)

```bash
# Анализ медленных запросов
vm-cli --config vm-cluster.toml debug slow-queries --top 10 --range '1h'

# Поиск пропусков в данных
vm-cli --config vm-cluster.toml debug gaps 'http_requests_total' --range '24h' --min-gap 60

# Анализ использования памяти
vm-cli --config vm-cluster.toml debug memory --verbose

# Тестирование производительности
vm-cli --config vm-cluster.toml debug performance --count 10 --query 'up'

# Анализ метрик
vm-cli --config vm-cluster.toml debug metrics --stats
vm-cli --config vm-cluster.toml debug metrics 'http_*' --export metrics.txt
```

## 🎨 Форматы вывода

### Table (по умолчанию)
```
┌─────────────┬─────────┬─────────────────────────┐
│ timestamp   │ value   │ labels                  │
├─────────────┼─────────┼─────────────────────────┤
│ 1705312800  │ 1       │ instance="localhost"    │
└─────────────┴─────────┴─────────────────────────┘
```

### JSON
```json
{
  "status": "success",
  "data": {
    "result": [
      {
        "metric": {"__name__": "up", "instance": "localhost"},
        "value": [1705312800, "1"]
      }
    ]
  }
}
```

### CSV
```csv
timestamp,value,metric_name
1705312800,1,up
```

## 🔍 Примеры использования

### Работа с кластерной версией VictoriaMetrics

```bash
# Проверка здоровья кластера
vm-cli --config vm-cluster.toml health --verbose

# Получение списка метрик
vm-cli --config vm-cluster.toml debug metrics --stats

# Простой запрос к кластеру
vm-cli --config vm-cluster.toml query 'up'

# Запрос с фильтрацией по инстансам
vm-cli --config vm-cluster.toml query '{instance="dc1-dzz-broker-2-1-01:9710"}'

# Анализ производительности кластера
vm-cli --config vm-cluster.toml debug performance --count 20 --query 'rate(http_requests_total[5m])'
```

### Мониторинг производительности

```bash
# Проверка здоровья кластера
vm-cli --config vm-cluster.toml health --verbose

# Анализ медленных запросов
vm-cli --config vm-cluster.toml debug slow-queries --top 5

# Тестирование производительности
vm-cli --config vm-cluster.toml debug performance --count 20 --query 'rate(http_requests_total[5m])'
```

### Анализ данных

```bash
# Поиск метрик по паттерну
vm-cli --config vm-cluster.toml debug metrics 'http_*' --stats

# Экспорт данных для анализа
vm-cli --config vm-cluster.toml export 'http_requests_total' --range '7d' --format csv --output http_data.csv

# Поиск пропусков в данных
vm-cli --config vm-cluster.toml debug gaps 'node_cpu_seconds_total' --range '24h'
```

### Администрирование

```bash
# Проверка retention
vm-cli --config vm-cluster.toml admin retention --check

# Создание снепшота
vm-cli --config vm-cluster.toml admin snapshot --name 'before-maintenance'

# Удаление старых метрик
vm-cli --config vm-cluster.toml admin delete 'test_metric_*' --start '2023-01-01' --confirm
```

## 🛠️ Разработка

### Сборка

```bash
# Отладочная сборка
cargo build

# Релизная сборка
cargo build --release

# Запуск тестов
cargo test
```

### Структура проекта

```
vm-cli/
├── src/
│   ├── main.rs          # Главный файл с CLI
│   ├── api.rs           # API клиент для VictoriaMetrics
│   ├── config.rs        # Конфигурация
│   ├── error.rs         # Обработка ошибок
│   ├── utils.rs         # Утилиты
│   └── commands/        # Команды
│       ├── mod.rs
│       ├── query.rs     # Запросы
│       ├── health.rs    # Здоровье
│       ├── export.rs    # Экспорт
│       ├── import.rs    # Импорт
│       ├── admin.rs     # Администрирование
│       └── debug.rs     # Отладка
├── Cargo.toml
└── README.md
```

## 🤝 Вклад в проект

1. Форкните репозиторий
2. Создайте ветку для новой функции (`git checkout -b feature/amazing-feature`)
3. Зафиксируйте изменения (`git commit -m 'Add amazing feature'`)
4. Отправьте в ветку (`git push origin feature/amazing-feature`)
5. Откройте Pull Request

## 📄 Лицензия

Этот проект лицензирован под MIT License - см. файл [LICENSE](LICENSE) для деталей.

## 📞 Поддержка

Если у вас есть вопросы или проблемы:

- Создайте [Issue](https://github.com/Deplee/victoria-metrics-cli/issues)
- Напишите на email: dkapitsev@gmail.com
