use crate::error::{Result, VmCliError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct VmClient {
    client: Client,
    base_url: String,
    cluster_config: Option<crate::config::ClusterConfig>,
}

pub struct VmInsertClient {
    client: Client,
    base_url: String,
}

pub struct VmStorageClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub status: String,
    pub data: QueryData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryData {
    pub result_type: Option<String>,
    pub result: Vec<QueryResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub metric: std::collections::HashMap<String, String>,
    pub value: Option<(f64, String)>,
    pub values: Option<Vec<(f64, String)>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: Option<String>,
    pub uptime: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsList {
    pub status: String,
    pub data: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub created_at: String,
    pub size: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetentionInfo {
    pub current_retention: String,
    pub used_space: String,
    pub total_space: String,
    pub metrics_older_than_30d: Option<String>,
    pub metrics_older_than_60d: Option<String>,
    pub metrics_older_than_90d: Option<String>,
}

impl VmClient {
    pub fn new(host: &str, timeout: u64, cluster_config: Option<crate::config::ClusterConfig>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| VmCliError::HttpError(reqwest::Error::from(e)))?;

        Ok(Self {
            client,
            base_url: host.to_string(),
            cluster_config,
        })
    }

    fn get_endpoint(&self, default_endpoint: &str) -> String {
        if let Some(cluster) = &self.cluster_config {
            if cluster.use_select_endpoint {
                // Для API endpoints используем select формат
                if default_endpoint.starts_with("/api/") {
                    format!("/select/{}/prometheus{}", cluster.select_account_id, default_endpoint)
                } else {
                    // Для других endpoints (например, /health) используем прямой путь
                    default_endpoint.to_string()
                }
            } else {
                cluster.query_endpoint.clone()
            }
        } else {
            default_endpoint.to_string()
        }
    }

    pub fn create_insert_client(&self, timeout: u64) -> Result<VmInsertClient> {
        let insert_host = if let Some(cluster) = &self.cluster_config {
            cluster.vminsert_host.as_ref().unwrap_or(&self.base_url)
        } else {
            &self.base_url
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| VmCliError::HttpError(reqwest::Error::from(e)))?;

        Ok(VmInsertClient {
            client,
            base_url: insert_host.to_string(),
        })
    }

    pub fn create_storage_client(&self, timeout: u64) -> Result<VmStorageClient> {
        let storage_host = if let Some(cluster) = &self.cluster_config {
            cluster.vmstorage_host.as_ref().unwrap_or(&self.base_url)
        } else {
            &self.base_url
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| VmCliError::HttpError(reqwest::Error::from(e)))?;

        Ok(VmStorageClient {
            client,
            base_url: storage_host.to_string(),
        })
    }

    pub async fn query(&self, query: &str, time: Option<&str>) -> Result<QueryResponse> {
        let endpoint = self.get_endpoint("/api/v1/query");
        let url = format!("{}{}", self.base_url, endpoint);
        let mut params = vec![("query", query)];
        
        if let Some(t) = time {
            params.push(("time", t));
        }

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        debug!("Query response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let query_response: QueryResponse = response.json().await?;
        Ok(query_response)
    }

    pub async fn query_range(
        &self,
        query: &str,
        start: &str,
        end: &str,
        step: &str,
    ) -> Result<QueryResponse> {
        let endpoint = self.get_endpoint("/api/v1/query_range");
        let url = format!("{}{}", self.base_url, endpoint);
        let params = vec![
            ("query", query),
            ("start", start),
            ("end", end),
            ("step", step),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        debug!("Query range response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let query_response: QueryResponse = response.json().await?;
        Ok(query_response)
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        let endpoint = self.get_endpoint("/health");
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self.client.get(&url).send().await?;
        
        debug!("Health response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let health_text = response.text().await?;
        let health_response = HealthResponse {
            status: health_text.trim().to_string(),
            version: None,
            uptime: None,
        };
        Ok(health_response)
    }

    pub async fn metrics(&self) -> Result<MetricsList> {
        let endpoint = self.get_endpoint("/api/v1/label/__name__/values");
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self.client.get(&url).send().await?;
        
        debug!("Metrics response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let metrics_list: MetricsList = response.json().await?;
        Ok(metrics_list)
    }

    pub async fn delete_series(&self, match_: &str, start: Option<&str>, end: Option<&str>) -> Result<()> {
        let url = format!("{}/api/v1/admin/tsdb/delete_series", self.base_url);
        let mut params = vec![("match[]", match_)];
        
        if let Some(s) = start {
            params.push(("start", s));
        }
        if let Some(e) = end {
            params.push(("end", e));
        }

        let response = self
            .client
            .post(&url)
            .query(&params)
            .send()
            .await?;

        debug!("Delete series response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        Ok(())
    }

    pub async fn export(&self, match_: &str, start: Option<&str>, end: Option<&str>) -> Result<String> {
        let url = format!("{}/api/v1/export", self.base_url);
        let mut params = vec![("match[]", match_)];
        
        if let Some(s) = start {
            params.push(("start", s));
        }
        if let Some(e) = end {
            params.push(("end", e));
        }

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        debug!("Export response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let export_data = response.text().await?;
        Ok(export_data)
    }

    pub async fn import(&self, data: &str) -> Result<()> {
        let url = format!("{}/api/v1/import", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .body(data.to_string())
            .send()
            .await?;

        debug!("Import response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        Ok(())
    }

    // Методы для работы со снепшотами
    pub async fn create_snapshot(&self, name: &str) -> Result<String> {
        let url = if let Some(cluster_config) = &self.cluster_config {
            // В кластерной версии используем vmstorage
            if let Some(vmstorage_host) = &cluster_config.vmstorage_host {
                format!("{}/snapshot/create", vmstorage_host)
            } else {
                return Err(VmCliError::ApiError {
                    message: "vmstorage_host не настроен в конфигурации кластера".to_string(),
                    status: None,
                });
            }
        } else {
            format!("{}/snapshot/create", self.base_url)
        };
        
        let response = self
            .client
            .post(&url)
            .query(&[("snapshot", name)])
            .send()
            .await?;

        debug!("Create snapshot response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let snapshot_id = response.text().await?;
        Ok(snapshot_id)
    }

    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let url = if let Some(cluster_config) = &self.cluster_config {
            // В кластерной версии используем vmstorage
            if let Some(vmstorage_host) = &cluster_config.vmstorage_host {
                format!("{}/snapshot/list", vmstorage_host)
            } else {
                return Err(VmCliError::ApiError {
                    message: "vmstorage_host не настроен в конфигурации кластера".to_string(),
                    status: None,
                });
            }
        } else {
            format!("{}/snapshot/list", self.base_url)
        };
        
        let response = self.client.get(&url).send().await?;

        debug!("List snapshots response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        // Парсим ответ в формате {"status":"ok","snapshots":[...]}
        let response_text = response.text().await?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        
        if let Some(snapshots_array) = response_json.get("snapshots").and_then(|s| s.as_array()) {
            let mut snapshots = Vec::new();
            for snapshot_value in snapshots_array {
                if let Some(snapshot_obj) = snapshot_value.as_object() {
                    let snapshot = SnapshotInfo {
                        name: snapshot_obj.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                        created_at: snapshot_obj.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                        size: snapshot_obj.get("size").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        status: snapshot_obj.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                    };
                    snapshots.push(snapshot);
                }
            }
            Ok(snapshots)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn delete_snapshot(&self, name: &str) -> Result<()> {
        let url = if let Some(cluster_config) = &self.cluster_config {
            // В кластерной версии используем vmstorage
            if let Some(vmstorage_host) = &cluster_config.vmstorage_host {
                format!("{}/snapshot/delete", vmstorage_host)
            } else {
                return Err(VmCliError::ApiError {
                    message: "vmstorage_host не настроен в конфигурации кластера".to_string(),
                    status: None,
                });
            }
        } else {
            format!("{}/snapshot/delete", self.base_url)
        };
        
        let response = self
            .client
            .post(&url)
            .query(&[("snapshot", name)])
            .send()
            .await?;

        debug!("Delete snapshot response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        Ok(())
    }

    pub async fn restore_snapshot(&self, name: &str) -> Result<()> {
        let url = if let Some(cluster_config) = &self.cluster_config {
            // В кластерной версии используем vmstorage
            if let Some(vmstorage_host) = &cluster_config.vmstorage_host {
                format!("{}/snapshot/restore", vmstorage_host)
            } else {
                return Err(VmCliError::ApiError {
                    message: "vmstorage_host не настроен в конфигурации кластера".to_string(),
                    status: None,
                });
            }
        } else {
            format!("{}/snapshot/restore", self.base_url)
        };
        
        let response = self
            .client
            .post(&url)
            .query(&[("snapshot", name)])
            .send()
            .await?;

        debug!("Restore snapshot response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        Ok(())
    }

    // Методы для работы с retention
    pub async fn get_retention_info(&self) -> Result<RetentionInfo> {
        let url = format!("{}/admin/tsdb/retention", self.base_url);
        
        let response = self.client.get(&url).send().await?;

        debug!("Retention info response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let retention_info: RetentionInfo = response.json().await?;
        Ok(retention_info)
    }

    pub async fn set_retention(&self, retention: &str) -> Result<()> {
        let url = format!("{}/admin/tsdb/retention", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .query(&[("retention", retention)])
            .send()
            .await?;

        debug!("Set retention response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        Ok(())
    }



    // Методы для работы с режимами работы
    pub async fn get_flags(&self) -> Result<serde_json::Value> {
        let url = format!("{}/flags", self.base_url);
        
        let response = self.client.get(&url).send().await?;

        debug!("Get flags response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let flags: serde_json::Value = response.json().await?;
        Ok(flags)
    }

    pub async fn get_build_info(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/status/buildinfo", self.base_url);
        
        let response = self.client.get(&url).send().await?;

        debug!("Get build info response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmCliError::ApiError {
                message: error_text,
                status: Some(status),
            });
        }

        let build_info: serde_json::Value = response.json().await?;
        Ok(build_info)
    }
} 