// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Shared Aggregation Logic
//!
//! Extracts the repeated ProductAggregator pattern into a single reusable function.
//! All provider adapters produce `Vec<RawBillItem>`, and this module converts them
//! into deduplicated `Vec<ProductCost>` entries with region details and instance counts.

use std::collections::{HashMap, HashSet};

use crate::service::{ProductCost, RegionDetail};

use super::traits::RawBillItem;

/// Aggregator: product_key -> (total_cost, unique_instance_ids, region -> (cost, count))
type ProductAggregator = HashMap<String, (f64, HashSet<String>, HashMap<String, (f64, u32)>)>;

/// Aggregate raw bill items into deduplicated ProductCost entries.
///
/// Returns `(products, total_cost)` where products are sorted by cost descending.
pub fn aggregate_bill_items(
    items: Vec<RawBillItem>,
    account_id: Option<&str>,
    account_name: Option<&str>,
) -> (Vec<ProductCost>, f64) {
    let mut products_map: ProductAggregator = HashMap::new();
    let mut total_cost = 0.0;

    for item in items {
        total_cost += item.cost;

        let key = format!("{}:{}", item.product_code, item.product_name);
        let entry = products_map
            .entry(key)
            .or_insert_with(|| (0.0, HashSet::new(), HashMap::new()));
        entry.0 += item.cost;
        if !item.instance_id.is_empty() {
            entry.1.insert(item.instance_id);
        }
        if !item.region.is_empty() {
            let r = entry.2.entry(item.region).or_insert((0.0, 0));
            r.0 += item.cost;
            r.1 += 1;
        }
    }

    let mut products: Vec<ProductCost> = products_map
        .into_iter()
        .map(|(key, (cost, instances, regions_map))| {
            let parts: Vec<&str> = key.splitn(2, ':').collect();
            let count = if instances.is_empty() {
                None
            } else {
                Some(instances.len() as u32)
            };
            let mut regions: Vec<String> = regions_map.keys().cloned().collect();
            regions.sort();
            let mut region_details: Vec<RegionDetail> = regions_map
                .into_iter()
                .map(|(r, (c, n))| RegionDetail {
                    region: r,
                    cost: c,
                    count: n,
                })
                .collect();
            region_details.sort_by(|a, b| {
                b.cost
                    .partial_cmp(&a.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            ProductCost {
                product_code: parts.first().unwrap_or(&"unknown").to_string(),
                product_name: parts.get(1).unwrap_or(&"Unknown").to_string(),
                cost,
                usage: None,
                unit: None,
                account_id: account_id.map(|s| s.to_string()),
                account_name: account_name.map(|s| s.to_string()),
                count,
                regions,
                region_details,
            }
        })
        .collect();

    products.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    (products, total_cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_empty() {
        let (products, total) = aggregate_bill_items(vec![], None, None);
        assert!(products.is_empty());
        assert_eq!(total, 0.0);
    }

    #[test]
    fn test_aggregate_single_item() {
        let items = vec![RawBillItem {
            product_code: "ecs".to_string(),
            product_name: "Elastic Compute Service".to_string(),
            cost: 100.0,
            region: "cn-hangzhou".to_string(),
            instance_id: "i-123".to_string(),
            usage: None,
            unit: None,
        }];

        let (products, total) = aggregate_bill_items(items, Some("acct1"), Some("Account 1"));
        assert_eq!(products.len(), 1);
        assert_eq!(total, 100.0);
        assert_eq!(products[0].product_code, "ecs");
        assert_eq!(products[0].product_name, "Elastic Compute Service");
        assert_eq!(products[0].cost, 100.0);
        assert_eq!(products[0].count, Some(1));
        assert_eq!(products[0].regions, vec!["cn-hangzhou".to_string()]);
        assert_eq!(products[0].account_id.as_deref(), Some("acct1"));
        assert_eq!(products[0].account_name.as_deref(), Some("Account 1"));
    }

    #[test]
    fn test_aggregate_multiple_products() {
        let items = vec![
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 200.0,
                region: String::new(),
                instance_id: String::new(),
                usage: None,
                unit: None,
            },
            RawBillItem {
                product_code: "rds".to_string(),
                product_name: "RDS".to_string(),
                cost: 300.0,
                region: String::new(),
                instance_id: String::new(),
                usage: None,
                unit: None,
            },
        ];

        let (products, total) = aggregate_bill_items(items, None, None);
        assert_eq!(products.len(), 2);
        assert_eq!(total, 500.0);
        // Should be sorted by cost desc
        assert_eq!(products[0].product_code, "rds");
        assert_eq!(products[1].product_code, "ecs");
    }

    #[test]
    fn test_aggregate_regions() {
        let items = vec![
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 100.0,
                region: "cn-hangzhou".to_string(),
                instance_id: "i-1".to_string(),
                usage: None,
                unit: None,
            },
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 200.0,
                region: "cn-beijing".to_string(),
                instance_id: "i-2".to_string(),
                usage: None,
                unit: None,
            },
        ];

        let (products, total) = aggregate_bill_items(items, None, None);
        assert_eq!(products.len(), 1);
        assert_eq!(total, 300.0);
        assert_eq!(products[0].cost, 300.0);
        assert_eq!(products[0].regions.len(), 2);
        assert!(products[0].regions.contains(&"cn-hangzhou".to_string()));
        assert!(products[0].regions.contains(&"cn-beijing".to_string()));
        assert_eq!(products[0].region_details.len(), 2);
        // region_details sorted by cost desc
        assert_eq!(products[0].region_details[0].region, "cn-beijing");
        assert_eq!(products[0].region_details[0].cost, 200.0);
    }

    #[test]
    fn test_aggregate_instance_count() {
        let items = vec![
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 50.0,
                region: "cn-hangzhou".to_string(),
                instance_id: "i-1".to_string(),
                usage: None,
                unit: None,
            },
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 50.0,
                region: "cn-hangzhou".to_string(),
                instance_id: "i-2".to_string(),
                usage: None,
                unit: None,
            },
            RawBillItem {
                product_code: "ecs".to_string(),
                product_name: "ECS".to_string(),
                cost: 25.0,
                region: "cn-hangzhou".to_string(),
                instance_id: "i-1".to_string(), // duplicate
                usage: None,
                unit: None,
            },
        ];

        let (products, total) = aggregate_bill_items(items, None, None);
        assert_eq!(products.len(), 1);
        assert_eq!(total, 125.0);
        // i-1 appears twice but should be counted once
        assert_eq!(products[0].count, Some(2));
    }
}
