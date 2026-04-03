// Copyright 2025 OpenObserve Inc.

//! AWS Cost Explorer Billing Provider
//!
//! This module implements billing integration with AWS using Cost Explorer API via official AWS SDK

use aws_config::{BehaviorVersion, Region};
use aws_sdk_costexplorer::{
    types::{DateInterval, Granularity, GroupDefinition, Metric},
    Client as CostExplorerClient,
};
use serde::{Deserialize, Serialize};

use super::super::{BillingError, Result};

/// AWS Billing Client using official AWS SDK
pub struct AwsBillingClient {
    client: CostExplorerClient,
}

/// Response structures for compatibility with existing code
#[derive(Debug, Serialize, Deserialize)]
pub struct AwsCostResponse {
    pub results_by_time: Option<Vec<AwsCostResult>>,
}

/// A single time-period result from the AWS Cost Explorer API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwsCostResult {
    pub time_period: AwsTimePeriod,
    pub total: Option<serde_json::Map<String, serde_json::Value>>,
    pub groups: Option<Vec<AwsCostGroup>>,
    pub estimated: Option<bool>,
}

/// Start/end date range for an AWS cost query.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwsTimePeriod {
    pub start: String,
    pub end: String,
}

/// A grouped cost entry from the AWS Cost Explorer API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwsCostGroup {
    pub keys: Vec<String>,
    pub metrics: serde_json::Map<String, serde_json::Value>,
}

impl AwsBillingClient {
    /// Create a new AWS Billing Client with explicit credentials
    /// Note: Cost Explorer API is a global service that must use us-east-1 region
    pub async fn new(
        access_key_id: String,
        secret_access_key: String,
        _region: String, // Ignored - Cost Explorer requires us-east-1
    ) -> Result<Self> {
        // AWS Cost Explorer is a global service that MUST use us-east-1
        let ce_region = "us-east-1";

        tracing::info!(
            "Initializing AWS Cost Explorer client (forced region: {})",
            ce_region
        );

        // Create credentials provider
        let credentials = aws_sdk_costexplorer::config::Credentials::new(
            access_key_id,
            secret_access_key,
            None, // session token
            None, // expiry
            "cloud-billing",
        );

        // Build AWS config with explicit credentials and us-east-1 region
        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(ce_region.to_string()))
            .load()
            .await;

        let client = CostExplorerClient::new(&config);

        Ok(Self { client })
    }

    /// Create a new AWS Billing Client using default credential chain
    /// This will automatically load credentials from:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. ~/.aws/credentials file
    /// 3. IAM role (for EC2/ECS instances)
    ///
    /// Note: Cost Explorer API is a global service that must use us-east-1 region
    pub async fn new_with_default_credentials(_region: String) -> Result<Self> {
        // AWS Cost Explorer is a global service that MUST use us-east-1
        let ce_region = "us-east-1";

        tracing::info!(
            "Initializing AWS Cost Explorer client with default credential chain (forced region: {})",
            ce_region
        );

        // Build AWS config using default credential chain
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(ce_region.to_string()))
            .load()
            .await;

        let client = CostExplorerClient::new(&config);

        Ok(Self { client })
    }

    /// Query cost and usage data
    pub async fn get_cost_and_usage(
        &self,
        start_date: &str,
        end_date: &str,
        granularity: &str,
        metrics: Vec<&str>,
        group_by: Option<Vec<(&str, &str)>>,
    ) -> Result<AwsCostResponse> {
        tracing::info!(
            "Querying AWS costs from {} to {} with granularity {}",
            start_date,
            end_date,
            granularity
        );

        // Parse granularity
        let granularity_enum = match granularity {
            "DAILY" => Granularity::Daily,
            "MONTHLY" => Granularity::Monthly,
            "HOURLY" => Granularity::Hourly,
            _ => {
                return Err(BillingError::ServiceError(format!(
                    "Invalid granularity: {}",
                    granularity
                )));
            }
        };

        // Convert metrics to AWS SDK format
        let metrics_enum: Vec<Metric> = metrics
            .iter()
            .filter_map(|m| match *m {
                "BlendedCost" => Some(Metric::BlendedCost),
                "UnblendedCost" => Some(Metric::UnblendedCost),
                "UsageQuantity" => Some(Metric::UsageQuantity),
                "AmortizedCost" => Some(Metric::AmortizedCost),
                "NetAmortizedCost" => Some(Metric::NetAmortizedCost),
                "NetUnblendedCost" => Some(Metric::NetUnblendedCost),
                "NormalizedUsageAmount" => Some(Metric::NormalizedUsageAmount),
                _ => {
                    tracing::warn!("Unknown metric: {}", m);
                    None
                }
            })
            .collect();

        if metrics_enum.is_empty() {
            return Err(BillingError::ServiceError(
                "No valid metrics provided".to_string(),
            ));
        }

        // Build the request
        let mut request = self
            .client
            .get_cost_and_usage()
            .time_period(
                DateInterval::builder()
                    .start(start_date)
                    .end(end_date)
                    .build()
                    .map_err(|e| {
                        BillingError::ServiceError(format!("Failed to build date interval: {}", e))
                    })?,
            )
            .granularity(granularity_enum);

        // Add metrics
        for metric in metrics_enum {
            request = request.metrics(metric.as_str());
        }

        // Add group by if provided
        if let Some(group_by_items) = group_by {
            for (typ, key) in group_by_items {
                let group_def = GroupDefinition::builder()
                    .r#type(typ.into())
                    .key(key)
                    .build();

                request = request.group_by(group_def);
            }
        }

        // Execute the request
        let response = request.send().await.map_err(|e| {
            tracing::error!("AWS Cost Explorer API error details: {:?}", e);
            BillingError::ServiceError(format!("AWS Cost Explorer API error: {}", e))
        })?;

        // Convert AWS SDK response to our response format
        let results_by_time = response.results_by_time.map(|results| {
            results
                .into_iter()
                .map(|result| {
                    // Convert time period
                    let time_period = AwsTimePeriod {
                        start: result
                            .time_period
                            .as_ref()
                            .map(|tp| tp.start.clone())
                            .unwrap_or_default(),
                        end: result
                            .time_period
                            .as_ref()
                            .map(|tp| tp.end.clone())
                            .unwrap_or_default(),
                    };

                    // Convert total metrics
                    let total = result.total.as_ref().map(|total_map| {
                        let mut map = serde_json::Map::new();
                        for (key, value) in total_map.iter() {
                            let mut metric_map = serde_json::Map::new();
                            if let Some(amount) = &value.amount {
                                metric_map.insert("Amount".to_string(), serde_json::json!(amount));
                            }
                            if let Some(unit) = &value.unit {
                                metric_map.insert("Unit".to_string(), serde_json::json!(unit));
                            }
                            map.insert(key.clone(), serde_json::json!(metric_map));
                        }
                        map
                    });

                    // Convert groups
                    let groups = result.groups.as_ref().map(|groups_vec| {
                        groups_vec
                            .iter()
                            .map(|group| {
                                let keys = group.keys.clone().unwrap_or_default();

                                let metrics = group
                                    .metrics
                                    .as_ref()
                                    .map(|metrics_map| {
                                        let mut map = serde_json::Map::new();
                                        for (key, value) in metrics_map.iter() {
                                            let mut metric_map = serde_json::Map::new();
                                            if let Some(amount) = &value.amount {
                                                metric_map.insert(
                                                    "Amount".to_string(),
                                                    serde_json::json!(amount),
                                                );
                                            }
                                            if let Some(unit) = &value.unit {
                                                metric_map.insert(
                                                    "Unit".to_string(),
                                                    serde_json::json!(unit),
                                                );
                                            }
                                            map.insert(key.clone(), serde_json::json!(metric_map));
                                        }
                                        map
                                    })
                                    .unwrap_or_default();

                                AwsCostGroup { keys, metrics }
                            })
                            .collect()
                    });

                    let estimated = result.estimated;

                    AwsCostResult {
                        time_period,
                        total,
                        groups,
                        estimated: Some(estimated),
                    }
                })
                .collect()
        });

        Ok(AwsCostResponse { results_by_time })
    }

    /// Check if Cost Explorer API is accessible
    pub async fn check_access(&self) -> Result<bool> {
        // Try to query a small date range to verify access
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        match self
            .get_cost_and_usage(&yesterday, &today, "DAILY", vec!["BlendedCost"], None)
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid AWS credentials
    async fn test_client_creation() {
        let client = AwsBillingClient::new(
            "test_key_id".to_string(),
            "test_secret".to_string(),
            "us-east-1".to_string(),
        )
        .await;

        assert!(client.is_ok());
    }
}
