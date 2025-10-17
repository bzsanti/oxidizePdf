//! Data Aggregation DSL
//!
//! Provides a simple, fluent API for common data aggregation operations
//! used in dashboard reporting.

use std::collections::HashMap;

/// Data aggregation builder for dashboard components
#[derive(Debug, Clone)]
pub struct DataAggregator {
    data: Vec<HashMap<String, String>>,
}

impl DataAggregator {
    /// Create a new data aggregator from raw data
    pub fn new(data: Vec<HashMap<String, String>>) -> Self {
        Self { data }
    }

    /// Group data by a field
    pub fn group_by(&self, field: &str) -> GroupedData {
        let mut groups: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();

        for record in &self.data {
            if let Some(value) = record.get(field) {
                groups
                    .entry(value.clone())
                    .or_insert_with(Vec::new)
                    .push(record.clone());
            }
        }

        GroupedData {
            groups,
            group_field: field.to_string(),
        }
    }

    /// Sum a numeric field
    pub fn sum(&self, field: &str) -> f64 {
        self.data
            .iter()
            .filter_map(|record| record.get(field))
            .filter_map(|value| value.parse::<f64>().ok())
            .sum()
    }

    /// Calculate average of a numeric field
    pub fn avg(&self, field: &str) -> f64 {
        let values: Vec<f64> = self
            .data
            .iter()
            .filter_map(|record| record.get(field))
            .filter_map(|value| value.parse::<f64>().ok())
            .collect();

        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        }
    }

    /// Count records
    pub fn count(&self) -> usize {
        self.data.len()
    }

    /// Get minimum value of a numeric field
    pub fn min(&self, field: &str) -> Option<f64> {
        self.data
            .iter()
            .filter_map(|record| record.get(field))
            .filter_map(|value| value.parse::<f64>().ok())
            .filter(|v| !v.is_nan()) // Filter out NaN values before comparison
            .min_by(|a, b| {
                a.partial_cmp(b)
                    .unwrap_or(std::cmp::Ordering::Equal) // NaN values treated as equal (shouldn't happen after filter)
            })
    }

    /// Get maximum value of a numeric field
    pub fn max(&self, field: &str) -> Option<f64> {
        self.data
            .iter()
            .filter_map(|record| record.get(field))
            .filter_map(|value| value.parse::<f64>().ok())
            .filter(|v| !v.is_nan()) // Filter out NaN values before comparison
            .max_by(|a, b| {
                a.partial_cmp(b)
                    .unwrap_or(std::cmp::Ordering::Equal) // NaN values treated as equal (shouldn't happen after filter)
            })
    }

    /// Filter data by a condition
    pub fn filter<F>(&self, predicate: F) -> DataAggregator
    where
        F: Fn(&HashMap<String, String>) -> bool,
    {
        DataAggregator {
            data: self.data.iter().filter(|r| predicate(r)).cloned().collect(),
        }
    }
}

/// Grouped data for aggregation operations
#[derive(Debug, Clone)]
pub struct GroupedData {
    groups: HashMap<String, Vec<HashMap<String, String>>>,
    group_field: String,
}

impl GroupedData {
    /// Aggregate each group with a function
    pub fn aggregate<F>(&self, field: &str, func: AggregateFunc, label: F) -> Vec<(String, f64)>
    where
        F: Fn(&str) -> String,
    {
        self.groups
            .iter()
            .map(|(key, records)| {
                let aggregator = DataAggregator::new(records.clone());
                let value = match func {
                    AggregateFunc::Sum => aggregator.sum(field),
                    AggregateFunc::Avg => aggregator.avg(field),
                    AggregateFunc::Count => aggregator.count() as f64,
                    AggregateFunc::Min => aggregator.min(field).unwrap_or(0.0),
                    AggregateFunc::Max => aggregator.max(field).unwrap_or(0.0),
                };
                (label(key), value)
            })
            .collect()
    }

    /// Sum each group
    pub fn sum(&self, field: &str) -> Vec<(String, f64)> {
        self.aggregate(field, AggregateFunc::Sum, |k| k.to_string())
    }

    /// Average each group
    pub fn avg(&self, field: &str) -> Vec<(String, f64)> {
        self.aggregate(field, AggregateFunc::Avg, |k| k.to_string())
    }

    /// Count each group
    pub fn count(&self) -> Vec<(String, f64)> {
        self.groups
            .iter()
            .map(|(key, records)| (key.clone(), records.len() as f64))
            .collect()
    }
}

/// Aggregate function types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunc {
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<HashMap<String, String>> {
        vec![
            [
                ("region".to_string(), "North".to_string()),
                ("amount".to_string(), "100".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
            [
                ("region".to_string(), "North".to_string()),
                ("amount".to_string(), "150".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
            [
                ("region".to_string(), "South".to_string()),
                ("amount".to_string(), "200".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        ]
    }

    #[test]
    fn test_sum() {
        let agg = DataAggregator::new(sample_data());
        assert_eq!(agg.sum("amount"), 450.0);
    }

    #[test]
    fn test_avg() {
        let agg = DataAggregator::new(sample_data());
        assert_eq!(agg.avg("amount"), 150.0);
    }

    #[test]
    fn test_count() {
        let agg = DataAggregator::new(sample_data());
        assert_eq!(agg.count(), 3);
    }

    #[test]
    fn test_min_max() {
        let agg = DataAggregator::new(sample_data());
        assert_eq!(agg.min("amount"), Some(100.0));
        assert_eq!(agg.max("amount"), Some(200.0));
    }

    #[test]
    fn test_group_by_sum() {
        let agg = DataAggregator::new(sample_data());
        let grouped = agg.group_by("region").sum("amount");

        assert_eq!(grouped.len(), 2);
        assert!(grouped.iter().any(|(k, v)| k == "North" && *v == 250.0));
        assert!(grouped.iter().any(|(k, v)| k == "South" && *v == 200.0));
    }

    #[test]
    fn test_group_by_count() {
        let agg = DataAggregator::new(sample_data());
        let grouped = agg.group_by("region").count();

        assert_eq!(grouped.len(), 2);
        assert!(grouped.iter().any(|(k, v)| k == "North" && *v == 2.0));
        assert!(grouped.iter().any(|(k, v)| k == "South" && *v == 1.0));
    }

    #[test]
    fn test_filter() {
        let agg = DataAggregator::new(sample_data());
        let filtered = agg.filter(|r| r.get("region") == Some(&"North".to_string()));

        assert_eq!(filtered.count(), 2);
        assert_eq!(filtered.sum("amount"), 250.0);
    }
}
