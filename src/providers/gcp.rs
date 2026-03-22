// Copyright 2025 OpenObserve Inc.

//! Google Cloud Platform (GCP) Billing Provider
//!
//! Implements billing integration with GCP using the Cloud Billing API v1.
//! Authentication uses service account JSON credentials with OAuth2 JWT exchange.

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{BillingError, Result};

const GCP_BILLING_ENDPOINT: &str = "https://cloudbilling.googleapis.com/v1";
const GCP_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GCP_BILLING_SCOPE: &str =
    "https://www.googleapis.com/auth/cloud-billing.readonly \
     https://www.googleapis.com/auth/cloud-platform \
     https://www.googleapis.com/auth/bigquery.readonly";
const GCP_BIGQUERY_ENDPOINT: &str = "https://bigquery.googleapis.com/bigquery/v2";

/// GCP Billing Client
pub struct GcpBillingClient {
    project_id: String,
    access_token: String,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct GcpBillingAccount {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub open: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GcpBillingAccountList {
    #[serde(rename = "billingAccounts", default)]
    pub billing_accounts: Vec<GcpBillingAccount>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GcpProjectBillingInfo {
    #[serde(rename = "projectId")]
    pub project_id: Option<String>,
    #[serde(rename = "billingAccountName")]
    pub billing_account_name: Option<String>,
    #[serde(rename = "billingEnabled")]
    pub billing_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GcpService {
    pub name: String,
    #[serde(rename = "serviceId")]
    pub service_id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GcpServiceList {
    #[serde(default)]
    pub services: Vec<GcpService>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GcpCostItem {
    pub service_name: String,
    pub service_id: String,
    pub cost: f64,
    pub currency: String,
    pub usage_amount: Option<f64>,
    pub usage_unit: Option<String>,
}

// ── Client implementation ───────────────────────────────────────────────

impl GcpBillingClient {
    /// Create a new GCP Billing Client using service account JSON
    pub async fn new(project_id: String, service_account_json: String) -> Result<Self> {
        tracing::info!("Initializing GCP Billing client for project: {}", project_id);

        let sa_key: ServiceAccountKey =
            serde_json::from_str(&service_account_json).map_err(|e| {
                BillingError::InvalidCredentials(format!(
                    "Failed to parse service account JSON: {}",
                    e
                ))
            })?;

        let access_token = Self::get_access_token(&sa_key).await?;

        Ok(Self {
            project_id,
            access_token,
            http_client: Client::new(),
        })
    }

    /// Create from GOOGLE_APPLICATION_CREDENTIALS file path
    pub async fn new_from_credentials_file(project_id: String) -> Result<Self> {
        let creds_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
            BillingError::InvalidCredentials(
                "GOOGLE_APPLICATION_CREDENTIALS not set".to_string(),
            )
        })?;

        let json = std::fs::read_to_string(&creds_path).map_err(|e| {
            BillingError::InvalidCredentials(format!(
                "Failed to read credentials file {}: {}",
                creds_path, e
            ))
        })?;

        Self::new(project_id, json).await
    }

    /// Exchange service account JWT for access token via OAuth2
    async fn get_access_token(sa_key: &ServiceAccountKey) -> Result<String> {
        let now = Utc::now().timestamp();
        let exp = now + 3600;
        let token_uri = sa_key
            .token_uri
            .as_deref()
            .unwrap_or(GCP_TOKEN_ENDPOINT);

        // Build JWT: header.claims.signature
        let header = base64url_json(&serde_json::json!({"alg": "RS256", "typ": "JWT"}))?;
        let claims = base64url_json(&serde_json::json!({
            "iss": sa_key.client_email,
            "scope": GCP_BILLING_SCOPE,
            "aud": token_uri,
            "iat": now,
            "exp": exp,
        }))?;

        let signing_input = format!("{}.{}", header, claims);
        let signature = rsa_sha256_sign(&sa_key.private_key, signing_input.as_bytes())?;
        let jwt = format!("{}.{}", signing_input, base64url_encode(&signature));

        // Token exchange
        let client = Client::new();
        let response = client
            .post(token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| BillingError::HttpError(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::InvalidCredentials(format!(
                "OAuth2 token exchange failed: {}",
                body
            )));
        }

        let resp: Value = response.json().await.map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse token response: {}", e))
        })?;

        resp.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                BillingError::InvalidCredentials("No access_token in response".to_string())
            })
    }

    async fn call_api(&self, url: &str) -> Result<Value> {
        let response = self
            .http_client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| BillingError::HttpError(format!("GCP API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ApiError(format!(
                "GCP API error {}: {}",
                status, body
            )));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| {
                BillingError::SerializationError(format!("Failed to parse response: {}", e))
            })
    }

    async fn call_api_post(&self, url: &str, body: &Value) -> Result<Value> {
        let response = self
            .http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| BillingError::HttpError(format!("GCP API POST failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ApiError(format!(
                "GCP API error {}: {}",
                status, body
            )));
        }

        response.json::<Value>().await.map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse response: {}", e))
        })
    }

    /// Query billing costs from BigQuery billing export
    ///
    /// Requires that billing export to BigQuery is enabled in the GCP project.
    /// The dataset and table name follow the standard export format.
    ///
    /// `dataset` should be like "billing_dataset" (the dataset you configured in billing export).
    /// `billing_table` should be like "gcp_billing_export_v1_XXXXXX_XXXXXX_XXXXXX".
    /// If not provided, we try to auto-detect.
    pub async fn query_billing_costs(
        &self,
        billing_cycle: &str,
        dataset: &str,
        billing_table: Option<&str>,
    ) -> Result<Vec<GcpCostItem>> {
        tracing::info!(
            "Querying GCP BigQuery billing for project={} cycle={}",
            self.project_id,
            billing_cycle
        );

        // If table not specified, try to find it
        let table = if let Some(t) = billing_table {
            t.to_string()
        } else {
            self.find_billing_table(dataset).await?
        };

        // Build SQL query — aggregate cost by service for the given month
        let sql = format!(
            r#"SELECT
                service.description AS service_name,
                service.id AS service_id,
                SUM(cost) AS total_cost,
                currency,
                SUM(usage.amount) AS total_usage,
                usage.unit AS usage_unit
            FROM `{}.{}.{}`
            WHERE invoice.month = '{}'
            GROUP BY service.description, service.id, currency, usage.unit
            ORDER BY total_cost DESC"#,
            self.project_id,
            dataset,
            table,
            billing_cycle.replace('-', ""), // "2026-03" -> "202603"
        );

        let url = format!(
            "{}/projects/{}/queries",
            GCP_BIGQUERY_ENDPOINT, self.project_id
        );

        let body = serde_json::json!({
            "query": sql,
            "useLegacySql": false,
            "maxResults": 10000,
        });

        let resp = self.call_api_post(&url, &body).await?;

        // Parse BigQuery response
        let rows = resp
            .get("rows")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut items = Vec::new();
        for row in &rows {
            let fields = match row.get("f").and_then(|v| v.as_array()) {
                Some(f) => f,
                None => continue,
            };

            let get_str = |idx: usize| -> String {
                fields
                    .get(idx)
                    .and_then(|f| f.get("v"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let get_f64 = |idx: usize| -> f64 {
                fields
                    .get(idx)
                    .and_then(|f| f.get("v"))
                    .and_then(|v| v.as_str().or_else(|| v.as_f64().map(|_| "")))
                    .and_then(|s| {
                        if s.is_empty() {
                            fields
                                .get(idx)
                                .and_then(|f| f.get("v"))
                                .and_then(|v| v.as_f64())
                        } else {
                            s.parse::<f64>().ok()
                        }
                    })
                    .unwrap_or(0.0)
            };

            items.push(GcpCostItem {
                service_name: get_str(0),
                service_id: get_str(1),
                cost: get_f64(2),
                currency: get_str(3),
                usage_amount: Some(get_f64(4)),
                usage_unit: Some(get_str(5)),
            });
        }

        Ok(items)
    }

    /// Auto-detect the billing export table name in the dataset
    async fn find_billing_table(&self, dataset: &str) -> Result<String> {
        let url = format!(
            "{}/projects/{}/datasets/{}/tables?maxResults=100",
            GCP_BIGQUERY_ENDPOINT, self.project_id, dataset
        );

        let resp = self.call_api(&url).await?;
        let tables = resp
            .get("tables")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Look for standard billing export table name pattern
        for table in &tables {
            if let Some(id) = table
                .get("tableReference")
                .and_then(|tr| tr.get("tableId"))
                .and_then(|v| v.as_str())
            {
                if id.starts_with("gcp_billing_export") {
                    tracing::info!("Found billing export table: {}", id);
                    return Ok(id.to_string());
                }
            }
        }

        Err(BillingError::ServiceError(format!(
            "No billing export table found in dataset '{}'. \
             Enable billing export to BigQuery in GCP Console.",
            dataset
        )))
    }

    /// List billing accounts
    pub async fn list_billing_accounts(&self) -> Result<GcpBillingAccountList> {
        tracing::info!("Listing GCP billing accounts");
        let url = format!("{}/billingAccounts", GCP_BILLING_ENDPOINT);
        let resp = self.call_api(&url).await?;
        serde_json::from_value(resp).map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse billing accounts: {}", e))
        })
    }

    /// Get billing info for the project
    pub async fn get_project_billing_info(&self) -> Result<GcpProjectBillingInfo> {
        let url = format!(
            "{}/projects/{}/billingInfo",
            GCP_BILLING_ENDPOINT, self.project_id
        );
        let resp = self.call_api(&url).await?;
        serde_json::from_value(resp).map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse billing info: {}", e))
        })
    }

    /// List all GCP services with pagination
    pub async fn list_services(&self) -> Result<Vec<GcpService>> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let url = match &page_token {
                Some(tok) => format!(
                    "{}/services?pageSize=200&pageToken={}",
                    GCP_BILLING_ENDPOINT, tok
                ),
                None => format!("{}/services?pageSize=200", GCP_BILLING_ENDPOINT),
            };

            let resp = self.call_api(&url).await?;
            let list: GcpServiceList = serde_json::from_value(resp).map_err(|e| {
                BillingError::SerializationError(format!("Failed to parse services: {}", e))
            })?;

            all.extend(list.services);
            match list.next_page_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }

        Ok(all)
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        match self.list_billing_accounts().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// ── Crypto helpers ──────────────────────────────────────────────────────

fn base64url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn base64url_json(value: &Value) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|e| {
        BillingError::SerializationError(format!("JSON encode error: {}", e))
    })?;
    Ok(base64url_encode(&bytes))
}

/// RSA-SHA256 signing using the `rsa` crate
fn rsa_sha256_sign(private_key_pem: &str, data: &[u8]) -> Result<Vec<u8>> {
    use pkcs8::DecodePrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::SignerMut;

    let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|e| {
        BillingError::InvalidCredentials(format!("Failed to parse private key: {}", e))
    })?;

    let mut signing_key = SigningKey::<sha2::Sha256>::new(private_key);
    let signature = signing_key.sign(data);

    use rsa::signature::SignatureEncoding;
    Ok(signature.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64url_encode() {
        assert_eq!(base64url_encode(b"hello"), "aGVsbG8");
    }
}
