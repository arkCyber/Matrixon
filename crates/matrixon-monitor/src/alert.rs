//! Alert Management Module
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: Implements alert rule management, evaluation, and notification for the Matrixon monitoring system. Supports custom alert rules, severity levels, and notification channels.
//!
//! All code is documented in English, with detailed function documentation, error handling, and performance characteristics.

use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{info, instrument, warn};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::sync::RwLock;

use crate::config::{AlertRule, AlertCondition, NotificationChannel};
use super::error::{Result, MonitorError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub rule: AlertRule,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub status: AlertStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

/// Alert manager for handling alert rules and notifications
///
/// The AlertManager struct manages alert rules, evaluates conditions, and sends notifications.
///
/// # Example
/// ```rust
/// let config = AlertConfig::default();
/// let manager = AlertManager::new(config).await.unwrap();
/// manager.start().await.unwrap();
/// ```
#[derive(Debug)]
pub struct AlertManager {
    rules: Arc<RwLock<Vec<AlertRule>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    metrics: Arc<RwLock<HashMap<String, f64>>>,
}

impl AlertManager {
    /// Initialize the alert manager
    ///
    /// Sets up alert rule evaluation and notification tasks.
    ///
    /// # Arguments
    /// * `config` - Alert configuration
    ///
    /// # Returns
    /// * `Result<Self, String>`
    #[instrument(level = "debug")]
    pub async fn new() -> Result<Self> {
        Ok(Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the alert manager
    ///
    /// Starts alert rule evaluation and notification tasks.
    ///
    /// # Returns
    /// * `Result<(), String>`
    #[instrument(level = "debug", skip(self))]
    pub async fn start(&mut self) -> Result<(), MonitorError> {
        info!("🔧 Starting alert monitoring");
        Ok(())
    }

    /// Stop the alert manager
    ///
    /// Stops alert rule evaluation and notification tasks.
    ///
    /// # Returns
    /// * `Result<(), String>`
    #[instrument(level = "debug", skip(self))]
    pub async fn stop(&mut self) -> Result<(), MonitorError> {
        info!("🛑 Stopping alert monitoring");
        self.alerts.write().await.clear();
        Ok(())
    }

    /// Evaluate all alert rules against current metrics
    #[instrument(skip(self, metrics), level = "debug")]
    pub async fn evaluate_rules(&mut self, metrics: &HashMap<String, f64>) -> Result<(), MonitorError> {
        let rules = self.rules.read().await;
        for rule in rules.iter() {
            if !rule.enabled { continue; }
            if self.should_trigger(rule, metrics) {
                self.trigger_alert(rule, metrics).await?;
            }
        }
        Ok(())
    }

    /// Check if an alert should be triggered
    fn should_trigger(&self, rule: &AlertRule, metrics: &HashMap<String, f64>) -> bool {
        let value = match &rule.condition {
            AlertCondition::MemoryUsagePercent => metrics.get("memory_usage_percent"),
            AlertCondition::CpuUsagePercent => metrics.get("cpu_usage_percent"),
            AlertCondition::DiskUsagePercent => metrics.get("disk_usage_percent"),
            AlertCondition::ResponseTimeMs => metrics.get("response_time_ms"),
            AlertCondition::ErrorRate => metrics.get("error_rate"),
            AlertCondition::FederationFailures => metrics.get("federation_failures"),
            AlertCondition::DatabaseConnections => metrics.get("database_connections"),
            AlertCondition::ActiveUsers => metrics.get("active_users"),
        };
        if let Some(&v) = value {
            v >= rule.threshold
        } else {
            false
        }
    }

    /// Get active alerts
    ///
    /// Returns a list of currently active alerts.
    ///
    /// # Returns
    /// * `Result<Vec<Alert>, String>`
    #[instrument(level = "debug", skip(self))]
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        Ok(self.alerts.read().await.clone())
    }

    /// Get alert history
    pub async fn get_alert_history(&self) -> Result<Vec<Alert>, MonitorError> {
        Ok(self.alerts.read().await.clone())
    }

    pub async fn check_alerts(&self) -> Result<(), MonitorError> {
        let metrics = self.metrics.read().await;
        let rules = self.rules.read().await;

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            if let Some(value) = metrics.get(&rule.name) {
                let should_alert = match rule.condition {
                    AlertCondition::MemoryUsagePercent => value > &rule.threshold,
                    AlertCondition::CpuUsagePercent => value > &rule.threshold,
                    AlertCondition::DiskUsagePercent => value > &rule.threshold,
                    AlertCondition::ResponseTimeMs => value > &rule.threshold,
                    AlertCondition::ErrorRate => value > &rule.threshold,
                    AlertCondition::FederationFailures => value > &rule.threshold,
                    AlertCondition::DatabaseConnections => value > &rule.threshold,
                    AlertCondition::ActiveUsers => value > &rule.threshold,
                };

                if should_alert {
                    let alert = Alert {
                        id: Uuid::new_v4(),
                        rule: rule.clone(),
                        value: *value,
                        timestamp: Utc::now(),
                        status: AlertStatus::Active,
                    };

                    self.send_notification(&alert).await?;
                    self.store_alert(&alert).await?;
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn trigger_alert(&self, rule: &AlertRule, metrics: &HashMap<String, f64>) -> Result<(), MonitorError> {
        let value = match metrics.get(&rule.name) {
            Some(v) => *v,
            None => return Ok(()),
        };

        if value >= rule.threshold {
            let alert = Alert {
                id: Uuid::new_v4(),
                rule: rule.clone(),
                value,
                timestamp: Utc::now(),
                status: AlertStatus::Active,
            };

            self.send_notification(&alert).await?;
            self.store_alert(&alert).await?;
        }

        Ok(())
    }

    async fn send_notification(&self, alert: &Alert) -> Result<(), MonitorError> {
        for channel_name in &alert.rule.channels {
            if let Some(channel) = self.find_notification_channel(channel_name).await {
                match channel.channel_type {
                    crate::config::ChannelType::Email => self.send_email_notification(alert, &channel).await?,
                    crate::config::ChannelType::Slack => self.send_slack_notification(alert, &channel).await?,
                    crate::config::ChannelType::Webhook => self.send_webhook_notification(alert, &channel).await?,
                    _ => warn!("Unsupported notification channel type: {:?}", channel.channel_type),
                }
            }
        }
        Ok(())
    }

    async fn find_notification_channel(&self, name: &str) -> Option<NotificationChannel> {
        // TODO: Implement channel lookup
        None
    }

    async fn send_email_notification(&self, alert: &Alert, channel: &NotificationChannel) -> Result<(), MonitorError> {
        // TODO: Implement email notification
        Ok(())
    }

    async fn send_slack_notification(&self, alert: &Alert, channel: &NotificationChannel) -> Result<(), MonitorError> {
        // TODO: Implement Slack notification
        Ok(())
    }

    async fn send_webhook_notification(&self, alert: &Alert, channel: &NotificationChannel) -> Result<(), MonitorError> {
        // TODO: Implement webhook notification
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn acknowledge_alert(&self, rule_name: &str) -> Result<(), MonitorError> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.rule.name == rule_name) {
            alert.status = AlertStatus::Acknowledged;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn resolve_alert(&self, rule_name: &str) -> Result<(), MonitorError> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.rule.name == rule_name) {
            alert.status = AlertStatus::Resolved;
            let mut alerts = self.alerts.write().await;
            alerts.retain(|a| a.rule.name != rule_name);
        }
        Ok(())
    }

    pub async fn add_rule(&self, rule: AlertRule) {
        let mut rules = self.rules.write().await;
        rules.push(rule);
    }

    async fn store_alert(&self, alert: &Alert) -> Result<(), MonitorError> {
        let mut alerts = self.alerts.write().await;
        alerts.push(alert.clone());
        Ok(())
    }
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            AlertRule, AlertCondition, AlertSeverity, AlertConfig, MetricsConfig,
            ChannelType, ChannelConfig
        },
        MetricsManager
    };

    #[tokio::test]
    async fn test_alert_manager() -> Result<()> {
        let email_channel = NotificationChannel {
            name: "email".to_string(),
            channel_type: ChannelType::Email,
            config: ChannelConfig {
                url: None,
                token: None,
                recipients: vec!["test@example.com".to_string()],
                custom_fields: HashMap::new(),
            },
            enabled: true,
        };

        let rule = AlertRule {
            name: "High CPU".to_string(),
            condition: AlertCondition::CpuUsagePercent,
            threshold: 80.0,
            duration_minutes: 1,
            severity: AlertSeverity::Critical,
            channels: vec!["email".to_string()],
            enabled: true,
        };
        let config = AlertConfig {
            enabled: true,
            rules: vec![rule.clone()],
            channels: vec![],
            cooldown_minutes: 5,
        };
        let metrics_config = MetricsConfig::default();
        let metrics_manager = Arc::new(MetricsManager::new(metrics_config)?);
        let mut manager = AlertManager::new().await?;
        manager.start().await?;
        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage_percent".to_string(), 85.0);
        manager.evaluate_rules(&metrics).await?;
        let active = manager.get_active_alerts().await?;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].rule.name, "High CPU");
        manager.stop().await?;
        Ok(())
    }
}
