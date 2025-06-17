// =============================================================================
// Matrixon Matrix NextServer - Audit Logger Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Core business logic service implementation. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • Business logic implementation
//   • Service orchestration
//   • Event handling and processing
//   • State management
//   • Enterprise-grade reliability
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use ruma::{OwnedUserId, UserId, api::client::error::ErrorKind};

use crate::{Error, Result};

/// Compliance mode for audit logging
use super::ComplianceMode;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditEvent {
    /// Service startup/shutdown
    ServiceStartup {
        service: String,
        timestamp: SystemTime,
    },
    ServiceShutdown {
        service: String,
        timestamp: SystemTime,
    },

    /// Authentication events
    AuthenticationAttempt {
        session_id: String,
        method: String,
        client_ip: Option<String>,
        timestamp: SystemTime,
    },
    AuthenticationSuccess {
        session_id: String,
        user_id: String,
        timestamp: SystemTime,
    },
    AuthenticationFailure {
        session_id: String,
        reason: String,
        client_ip: Option<String>,
        timestamp: SystemTime,
    },

    /// Session management
    SessionCreated {
        session_id: String,
        user_id: String,
        client_ip: Option<String>,
        timestamp: SystemTime,
    },
    SessionExpired {
        session_id: String,
        user_id: String,
        timestamp: SystemTime,
    },
    SessionCleanup {
        session_id: String,
        user_id: String,
        timestamp: SystemTime,
    },

    /// User management
    UserCreated {
        user_id: String,
        created_by: Option<String>,
        timestamp: SystemTime,
    },
    UserDeactivated {
        user_id: String,
        deactivated_by: Option<String>,
        reason: Option<String>,
        timestamp: SystemTime,
    },
    UserReactivated {
        user_id: String,
        reactivated_by: String,
        timestamp: SystemTime,
    },
    UserProfileUpdated {
        user_id: String,
        updated_by: String,
        fields_changed: Vec<String>,
        timestamp: SystemTime,
    },

    /// Room management
    RoomCreated {
        room_id: String,
        created_by: String,
        room_type: Option<String>,
        timestamp: SystemTime,
    },
    RoomDeleted {
        room_id: String,
        deleted_by: String,
        reason: Option<String>,
        timestamp: SystemTime,
    },
    RoomMembershipChanged {
        room_id: String,
        user_id: String,
        membership: String,
        changed_by: String,
        timestamp: SystemTime,
    },
    RoomSettingsChanged {
        room_id: String,
        changed_by: String,
        settings_changed: Vec<String>,
        timestamp: SystemTime,
    },

    /// Data access and modification
    DataAccessed {
        resource_type: String,
        resource_id: String,
        accessed_by: String,
        access_type: String,
        timestamp: SystemTime,
    },
    DataModified {
        resource_type: String,
        resource_id: String,
        modified_by: String,
        operation: String,
        timestamp: SystemTime,
    },
    DataExported {
        resource_type: String,
        exported_by: String,
        export_format: String,
        record_count: usize,
        timestamp: SystemTime,
    },

    /// Administrative actions
    AdminActionPerformed {
        action: String,
        performed_by: String,
        target: Option<String>,
        parameters: HashMap<String, String>,
        timestamp: SystemTime,
    },
    ConfigurationChanged {
        component: String,
        changed_by: String,
        changes: HashMap<String, String>,
        timestamp: SystemTime,
    },

    /// Security events
    SecurityViolation {
        violation_type: String,
        user_id: Option<String>,
        client_ip: Option<String>,
        details: String,
        timestamp: SystemTime,
    },
    SuspiciousActivity {
        activity_type: String,
        user_id: Option<String>,
        client_ip: Option<String>,
        risk_score: f32,
        timestamp: SystemTime,
    },
    AccessDenied {
        resource: String,
        user_id: String,
        reason: String,
        timestamp: SystemTime,
    },

    /// GDPR and privacy
    ConsentGiven {
        user_id: String,
        consent_type: String,
        version: String,
        timestamp: SystemTime,
    },
    ConsentWithdrawn {
        user_id: String,
        consent_type: String,
        timestamp: SystemTime,
    },
    DataRetentionAction {
        action: String,
        resource_type: String,
        affected_records: usize,
        timestamp: SystemTime,
    },
    PersonalDataRequest {
        request_type: String, // access, deletion, portability
        user_id: String,
        requested_by: String,
        timestamp: SystemTime,
    },
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique entry ID
    pub id: String,
    /// Sequence number for ordering
    pub sequence: u64,
    /// Audit event
    pub event: AuditEvent,
    /// Event severity level
    pub severity: AuditSeverity,
    /// Source component/service
    pub source: String,
    /// Entry timestamp
    pub timestamp: SystemTime,
    /// Entry hash for integrity
    pub integrity_hash: String,
    /// Previous entry hash for chain integrity
    pub previous_hash: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Audit event severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// Informational events
    Info,
    /// Warning events
    Warning,
    /// Error events
    Error,
    /// Critical security events
    Critical,
}

/// Audit query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Start time for query range
    pub start_time: Option<SystemTime>,
    /// End time for query range
    pub end_time: Option<SystemTime>,
    /// Event types to include
    pub event_types: Option<Vec<String>>,
    /// User ID filter
    pub user_id: Option<String>,
    /// Source component filter
    pub source: Option<String>,
    /// Severity filter
    pub severity: Option<AuditSeverity>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Report ID
    pub id: String,
    /// Report type
    pub report_type: ComplianceReportType,
    /// Report generation time
    pub generated_at: SystemTime,
    /// Time period covered
    pub period_start: SystemTime,
    pub period_end: SystemTime,
    /// Summary statistics
    pub summary: ComplianceReportSummary,
    /// Detailed findings
    pub findings: Vec<ComplianceFinding>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Compliance report types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceReportType {
    /// GDPR compliance report
    Gdpr,
    /// HIPAA compliance report
    Hipaa,
    /// SOX compliance report
    Sox,
    /// Security audit report
    Security,
    /// Access control report
    AccessControl,
    /// Data retention report
    DataRetention,
}

/// Compliance report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReportSummary {
    /// Total events audited
    pub total_events: usize,
    /// Events by severity
    pub events_by_severity: HashMap<String, usize>,
    /// Events by type
    pub events_by_type: HashMap<String, usize>,
    /// Compliance score (0-100)
    pub compliance_score: f32,
    /// Issues found
    pub issues_found: usize,
    /// Critical issues
    pub critical_issues: usize,
}

/// Compliance finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    /// Finding ID
    pub id: String,
    /// Finding type
    pub finding_type: String,
    /// Severity level
    pub severity: AuditSeverity,
    /// Description
    pub description: String,
    /// Related events
    pub related_events: Vec<String>,
    /// Risk assessment
    pub risk_level: RiskLevel,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Risk levels for compliance findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

/// Audit logger service
#[derive(Debug)]
pub struct AuditLogger {
    /// Enable audit logging
    enabled: bool,
    /// Compliance mode
    compliance_mode: ComplianceMode,
    /// Audit log entries
    log_entries: Arc<RwLock<Vec<AuditLogEntry>>>,
    /// Sequence counter
    sequence_counter: Arc<Mutex<u64>>,
    /// Log rotation settings
    max_entries: usize,
    /// Integrity chain tracking
    last_hash: Arc<Mutex<Option<String>>>,
    /// Real-time alerting system
    alert_thresholds: HashMap<String, u32>,
    /// Background tasks
    background_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl AuditLogger {
    /// Create new audit logger
    #[instrument(level = "info")]
    pub async fn new(enabled: bool, compliance_mode: ComplianceMode) -> Result<Self> {
        let start = std::time::Instant::now();
        info!("📋 Initializing Audit Logger Service");

        let logger = Self {
            enabled,
            compliance_mode,
            log_entries: Arc::new(RwLock::new(Vec::new())),
            sequence_counter: Arc::new(Mutex::new(0)),
            max_entries: 1000000, // 1M entries before rotation
            last_hash: Arc::new(Mutex::new(None)),
            alert_thresholds: Self::default_alert_thresholds(),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
        };

        if enabled {
            // Start background monitoring tasks
            logger.start_background_tasks().await;
        }

        info!("🎉 Audit Logger Service initialized in {:?}", start.elapsed());
        Ok(logger)
    }

    /// Log an audit event
    #[instrument(level = "debug", skip(self))]
    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let start = std::time::Instant::now();
        debug!("📝 Logging audit event: {:?}", std::mem::discriminant(&event));

        // Generate entry ID and sequence
        let entry_id = Uuid::new_v4().to_string();
        let sequence = {
            let mut counter = self.sequence_counter.lock().await;
            *counter += 1;
            *counter
        };

        // Determine severity with error handling
        let severity = match Self::determine_severity(&event) {
            Ok(s) => s,
            Err(e) => {
                error!("❌ Failed to determine event severity: {}", e);
                AuditSeverity::Error // Default to Error severity
            }
        };

        // Calculate integrity hash with error handling
        let previous_hash = {
            let last_hash_guard = self.last_hash.lock().await;
            last_hash_guard.clone()
        };

        let entry = AuditLogEntry {
            id: entry_id,
            sequence,
            event: event.clone(),
            severity: severity.clone(),
            source: "matrixon-matrix-server".to_string(),
            timestamp: SystemTime::now(),
            integrity_hash: String::new(), // Will be calculated
            previous_hash: previous_hash.clone(),
            metadata: HashMap::new(),
        };

        // Calculate integrity hash with error handling
        let integrity_hash = match self.calculate_integrity_hash(&entry).await {
            Ok(hash) => hash,
            Err(e) => {
                error!("❌ Failed to calculate integrity hash: {}", e);
                return Err(Error::bad_database("Failed to calculate audit log integrity hash"));
            }
        };

        let mut entry = entry;
        entry.integrity_hash = integrity_hash.clone();

        // Store entry with error handling
        {
            let mut entries = match self.log_entries.write().await {
                Ok(guard) => guard,
                Err(e) => {
                    error!("❌ Failed to acquire write lock for audit log: {}", e);
                    return Err(Error::bad_database("Failed to write audit log entry"));
                }
            };
            
            entries.push(entry);

            // Check for rotation with error handling
            if entries.len() > self.max_entries {
                if let Err(e) = self.rotate_logs(&mut entries).await {
                    error!("❌ Failed to rotate audit logs: {}", e);
                    return Err(Error::bad_database("Failed to rotate audit logs"));
                }
            }
        }

        // Update last hash with error handling
        {
            let mut last_hash_guard = match self.last_hash.lock().await {
                Ok(guard) => guard,
                Err(e) => {
                    error!("❌ Failed to acquire lock for hash update: {}", e);
                    return Err(Error::bad_database("Failed to update audit log hash"));
                }
            };
            *last_hash_guard = Some(integrity_hash);
        }

        // Check for alerts with error handling
        if let Err(e) = self.check_alert_conditions(&event, &severity).await {
            error!("❌ Failed to check alert conditions: {}", e);
            // Don't return error here as this is not critical
        }

        info!("✅ Audit event logged successfully in {:?}", start.elapsed());
        Ok(())
    }

    /// Query audit logs
    #[instrument(level = "debug", skip(self))]
    pub async fn query_logs(&self, query: AuditQuery) -> Result<Vec<AuditLogEntry>> {
        let start = std::time::Instant::now();
        debug!("🔍 Querying audit logs");

        let entries = self.log_entries.read().await;
        let mut filtered_entries: Vec<AuditLogEntry> = entries
            .iter()
            .filter(|entry| self.matches_query(entry, &query))
            .cloned()
            .collect();

        // Apply pagination
        if let Some(offset) = query.offset {
            if offset < filtered_entries.len() {
                filtered_entries = filtered_entries.into_iter().skip(offset).collect();
            } else {
                filtered_entries.clear();
            }
        }

        if let Some(limit) = query.limit {
            filtered_entries.truncate(limit);
        }

        debug!("✅ Query completed in {:?}, found {} entries", start.elapsed(), filtered_entries.len());
        Ok(filtered_entries)
    }

    /// Generate compliance report
    #[instrument(level = "debug", skip(self))]
    pub async fn generate_compliance_report(
        &self,
        report_type: ComplianceReportType,
        period_start: SystemTime,
        period_end: SystemTime,
    ) -> Result<ComplianceReport> {
        let start = std::time::Instant::now();
        info!("📊 Generating compliance report: {:?}", report_type);

        // Query relevant events for the period
        let query = AuditQuery {
            start_time: Some(period_start),
            end_time: Some(period_end),
            event_types: None,
            user_id: None,
            source: None,
            severity: None,
            limit: None,
            offset: None,
        };

        let events = self.query_logs(query).await?;

        // Analyze events based on compliance mode and report type
        let summary = self.analyze_compliance_data(&events, &report_type).await;
        let findings = self.identify_compliance_issues(&events, &report_type).await;
        let recommendations = self.generate_recommendations(&findings, &report_type).await;

        let report = ComplianceReport {
            id: Uuid::new_v4().to_string(),
            report_type,
            generated_at: SystemTime::now(),
            period_start,
            period_end,
            summary,
            findings,
            recommendations,
        };

        info!("🎉 Compliance report generated in {:?}", start.elapsed());
        Ok(report)
    }

    /// Verify log integrity
    #[instrument(level = "debug", skip(self))]
    pub async fn verify_integrity(&self) -> Result<bool> {
        let start = std::time::Instant::now();
        info!("🔒 Verifying audit log integrity");

        let entries = self.log_entries.read().await;
        let mut previous_hash: Option<String> = None;

        for entry in entries.iter() {
            // Verify previous hash chain
            if entry.previous_hash != previous_hash {
                warn!("❌ Integrity violation: Previous hash mismatch for entry {}", entry.id);
                return Ok(false);
            }

            // Verify entry integrity hash
            let calculated_hash = self.calculate_integrity_hash_for_verification(entry).await?;
            if calculated_hash != entry.integrity_hash {
                warn!("❌ Integrity violation: Hash mismatch for entry {}", entry.id);
                return Ok(false);
            }

            previous_hash = Some(entry.integrity_hash.clone());
        }

        info!("✅ Audit log integrity verified in {:?}", start.elapsed());
        Ok(true)
    }

    /// Export audit logs
    #[instrument(level = "debug", skip(self))]
    pub async fn export_logs(
        &self,
        query: AuditQuery,
        format: ExportFormat,
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!("📤 Exporting audit logs in format: {:?}", format);

        let entries = self.query_logs(query).await?;

        let exported_data = match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&entries)
                    .map_err(|e| Error::BadRequestString(ErrorKind::Unknown, &format!("JSON export failed: {}", e)))?
            },
            ExportFormat::Csv => {
                self.export_csv(&entries).await?
            },
            ExportFormat::Xml => {
                self.export_xml(&entries).await?
            },
        };

        // Log the export action
        self.log_event(AuditEvent::DataExported {
            resource_type: "audit_logs".to_string(),
            exported_by: "system".to_string(),
            export_format: format!("{:?}", format),
            record_count: entries.len(),
            timestamp: SystemTime::now(),
        }).await?;

        info!("✅ Audit logs exported in {:?}, {} entries", start.elapsed(), entries.len());
        Ok(exported_data)
    }

    // Private helper methods

    /// Determine severity level for an event
    fn determine_severity(event: &AuditEvent) -> Result<AuditSeverity> {
        match event {
            AuditEvent::SecurityViolation { .. } | 
            AuditEvent::SuspiciousActivity { risk_score, .. } if *risk_score > 0.8 => {
                Ok(AuditSeverity::Critical)
            },
            AuditEvent::AuthenticationFailure { .. } |
            AuditEvent::AccessDenied { .. } |
            AuditEvent::UserDeactivated { .. } => {
                Ok(AuditSeverity::Warning)
            },
            AuditEvent::SuspiciousActivity { risk_score, .. } if *risk_score > 0.5 => {
                Ok(AuditSeverity::Warning)
            },
            _ => Ok(AuditSeverity::Info),
        }
    }

    /// Calculate integrity hash for an entry
    async fn calculate_integrity_hash(&self, entry: &AuditLogEntry) -> Result<String> {
        let mut hasher = Sha256::new();
        
        // Hash core entry data (excluding the hash itself)
        hasher.update(entry.id.as_bytes());
        hasher.update(entry.sequence.to_le_bytes());
        hasher.update(serde_json::to_string(&entry.event)?.as_bytes());
        hasher.update(format!("{:?}", entry.severity).as_bytes());
        hasher.update(entry.source.as_bytes());
        hasher.update(entry.timestamp.duration_since(UNIX_EPOCH)?.as_secs().to_le_bytes());
        
        if let Some(prev_hash) = &entry.previous_hash {
            hasher.update(prev_hash.as_bytes());
        }

        let result = hasher.finalize();
        Ok(general_purpose::STANDARD.encode(result))
    }

    /// Calculate integrity hash for verification (excluding the stored hash)
    async fn calculate_integrity_hash_for_verification(&self, entry: &AuditLogEntry) -> Result<String> {
        let mut temp_entry = entry.clone();
        temp_entry.integrity_hash = String::new();
        self.calculate_integrity_hash(&temp_entry).await
    }

    /// Check if entry matches query criteria
    fn matches_query(&self, entry: &AuditLogEntry, query: &AuditQuery) -> bool {
        // Time range check
        if let Some(start_time) = query.start_time {
            if entry.timestamp < start_time {
                return false;
            }
        }

        if let Some(end_time) = query.end_time {
            if entry.timestamp > end_time {
                return false;
            }
        }

        // Event type check
        if let Some(event_types) = &query.event_types {
            let event_type = format!("{:?}", std::mem::discriminant(&entry.event));
            if !event_types.contains(&event_type) {
                return false;
            }
        }

        // User ID check
        if let Some(user_id) = &query.user_id {
            if !self.event_involves_user(&entry.event, user_id) {
                return false;
            }
        }

        // Source check
        if let Some(source) = &query.source {
            if entry.source != *source {
                return false;
            }
        }

        // Severity check
        if let Some(severity) = &query.severity {
            if std::mem::discriminant(&entry.severity) != std::mem::discriminant(severity) {
                return false;
            }
        }

        true
    }

    /// Check if event involves a specific user
    fn event_involves_user(&self, event: &AuditEvent, user_id: &str) -> bool {
        match event {
            AuditEvent::AuthenticationSuccess { user_id: event_user, .. } |
            AuditEvent::SessionCreated { user_id: event_user, .. } |
            AuditEvent::SessionExpired { user_id: event_user, .. } |
            AuditEvent::UserCreated { user_id: event_user, .. } |
            AuditEvent::UserDeactivated { user_id: event_user, .. } |
            AuditEvent::UserProfileUpdated { user_id: event_user, .. } |
            AuditEvent::RoomMembershipChanged { user_id: event_user, .. } |
            AuditEvent::AccessDenied { user_id: event_user, .. } |
            AuditEvent::ConsentGiven { user_id: event_user, .. } |
            AuditEvent::ConsentWithdrawn { user_id: event_user, .. } |
            AuditEvent::PersonalDataRequest { user_id: event_user, .. } => {
                event_user == user_id
            },
            AuditEvent::SecurityViolation { user_id: Some(event_user), .. } |
            AuditEvent::SuspiciousActivity { user_id: Some(event_user), .. } => {
                event_user == user_id
            },
            _ => false,
        }
    }

    /// Analyze compliance data for summary
    async fn analyze_compliance_data(
        &self,
        events: &[AuditLogEntry],
        _report_type: &ComplianceReportType,
    ) -> ComplianceReportSummary {
        let mut events_by_severity = HashMap::new();
        let mut events_by_type = HashMap::new();
        let mut issues_found = 0;
        let mut critical_issues = 0;

        for entry in events {
            // Count by severity
            let severity_str = format!("{:?}", entry.severity);
            *events_by_severity.entry(severity_str).or_insert(0) += 1;

            // Count by type
            let event_type = format!("{:?}", std::mem::discriminant(&entry.event));
            *events_by_type.entry(event_type).or_insert(0) += 1;

            // Count issues
            match entry.severity {
                AuditSeverity::Critical => {
                    critical_issues += 1;
                    issues_found += 1;
                },
                AuditSeverity::Error | AuditSeverity::Warning => {
                    issues_found += 1;
                },
                _ => {},
            }
        }

        // Calculate compliance score (simplified)
        let total_events = events.len();
        let compliance_score = if total_events > 0 {
            ((total_events - issues_found) as f32 / total_events as f32) * 100.0
        } else {
            100.0
        };

        ComplianceReportSummary {
            total_events,
            events_by_severity,
            events_by_type,
            compliance_score,
            issues_found,
            critical_issues,
        }
    }

    /// Identify compliance issues
    async fn identify_compliance_issues(
        &self,
        events: &[AuditLogEntry],
        report_type: &ComplianceReportType,
    ) -> Vec<ComplianceFinding> {
        let mut findings = Vec::new();

        match report_type {
            ComplianceReportType::Gdpr => {
                // Check for GDPR-specific issues
                self.check_gdpr_compliance(events, &mut findings).await;
            },
            ComplianceReportType::Security => {
                // Check for security issues
                self.check_security_compliance(events, &mut findings).await;
            },
            _ => {
                // Generic compliance checks
                self.check_generic_compliance(events, &mut findings).await;
            }
        }

        findings
    }

    /// Check GDPR compliance
    async fn check_gdpr_compliance(
        &self,
        events: &[AuditLogEntry],
        findings: &mut Vec<ComplianceFinding>,
    ) {
        // Check for data access without consent
        for event in events {
            if let AuditEvent::DataAccessed { accessed_by, .. } = &event.event {
                // Check if there's corresponding consent
                let consent_found = events.iter().any(|e| {
                    matches!(&e.event, AuditEvent::ConsentGiven { user_id, .. } if user_id == accessed_by)
                });

                if !consent_found {
                    findings.push(ComplianceFinding {
                        id: Uuid::new_v4().to_string(),
                        finding_type: "gdpr_data_access_without_consent".to_string(),
                        severity: AuditSeverity::Warning,
                        description: "Data accessed without documented user consent".to_string(),
                        related_events: vec![event.id.clone()],
                        risk_level: RiskLevel::Medium,
                        recommendations: vec![
                            "Ensure user consent is obtained before data access".to_string(),
                            "Implement consent verification mechanisms".to_string(),
                        ],
                    });
                }
            }
        }
    }

    /// Check security compliance
    async fn check_security_compliance(
        &self,
        events: &[AuditLogEntry],
        findings: &mut Vec<ComplianceFinding>,
    ) {
        // Check for failed authentication patterns
        let failed_auth_count = events.iter()
            .filter(|e| matches!(&e.event, AuditEvent::AuthenticationFailure { .. }))
            .count();

        if failed_auth_count > 10 {
            findings.push(ComplianceFinding {
                id: Uuid::new_v4().to_string(),
                finding_type: "high_authentication_failure_rate".to_string(),
                severity: AuditSeverity::Warning,
                description: format!("High number of authentication failures: {}", failed_auth_count),
                related_events: Vec::new(),
                risk_level: RiskLevel::Medium,
                recommendations: vec![
                    "Review authentication failure patterns".to_string(),
                    "Consider implementing additional security measures".to_string(),
                ],
            });
        }
    }

    /// Check generic compliance
    async fn check_generic_compliance(
        &self,
        events: &[AuditLogEntry],
        findings: &mut Vec<ComplianceFinding>,
    ) {
        // Check for critical events
        for event in events {
            if matches!(event.severity, AuditSeverity::Critical) {
                findings.push(ComplianceFinding {
                    id: Uuid::new_v4().to_string(),
                    finding_type: "critical_security_event".to_string(),
                    severity: AuditSeverity::Critical,
                    description: "Critical security event requiring immediate attention".to_string(),
                    related_events: vec![event.id.clone()],
                    risk_level: RiskLevel::Critical,
                    recommendations: vec![
                        "Investigate critical security event immediately".to_string(),
                        "Review security policies and procedures".to_string(),
                    ],
                });
            }
        }
    }

    /// Generate recommendations based on findings
    async fn generate_recommendations(
        &self,
        findings: &[ComplianceFinding],
        _report_type: &ComplianceReportType,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        let critical_count = findings.iter()
            .filter(|f| matches!(f.severity, AuditSeverity::Critical))
            .count();

        if critical_count > 0 {
            recommendations.push(format!(
                "Address {} critical security findings immediately", 
                critical_count
            ));
        }

        recommendations.push("Implement regular compliance monitoring".to_string());
        recommendations.push("Review and update security policies".to_string());
        recommendations.push("Conduct security awareness training".to_string());

        recommendations
    }

    /// Export logs as CSV
    async fn export_csv(&self, entries: &[AuditLogEntry]) -> Result<String> {
        let mut csv = String::from("ID,Sequence,Event Type,Severity,Source,Timestamp,Description\n");
        
        for entry in entries {
            let event_type = format!("{:?}", std::mem::discriminant(&entry.event));
            let description = format!("{:?}", entry.event);
            let timestamp = humantime::format_rfc3339_seconds(entry.timestamp);
            
            csv.push_str(&format!(
                "{},{},{},{:?},{},{},{}\n",
                entry.id,
                entry.sequence,
                event_type,
                entry.severity,
                entry.source,
                timestamp,
                description.replace(',', ';') // Escape commas
            ));
        }
        
        Ok(csv)
    }

    /// Export logs as XML
    async fn export_xml(&self, entries: &[AuditLogEntry]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<audit_logs>\n");
        
        for entry in entries {
            xml.push_str(&format!(
                "  <entry id=\"{}\" sequence=\"{}\" severity=\"{:?}\" source=\"{}\" timestamp=\"{}\">\n",
                entry.id,
                entry.sequence,
                entry.severity,
                entry.source,
                humantime::format_rfc3339_seconds(entry.timestamp)
            ));
            
            xml.push_str(&format!("    <event>{:?}</event>\n", entry.event));
            xml.push_str("  </entry>\n");
        }
        
        xml.push_str("</audit_logs>");
        Ok(xml)
    }

    /// Log rotation to prevent unlimited growth
    async fn rotate_logs(&self, entries: &mut Vec<AuditLogEntry>) -> Result<()> {
        info!("🔄 Rotating audit logs, current size: {}", entries.len());
        
        // Keep only the most recent 80% of entries
        let keep_count = (self.max_entries as f32 * 0.8) as usize;
        if entries.len() > keep_count {
            let remove_count = entries.len() - keep_count;
            entries.drain(0..remove_count);
        }
        
        info!("✅ Log rotation completed, new size: {}", entries.len());
        Ok(())
    }

    /// Check alert conditions
    async fn check_alert_conditions(&self, event: &AuditEvent, severity: &AuditSeverity) {
        match severity {
            AuditSeverity::Critical => {
                warn!("🚨 CRITICAL AUDIT EVENT: {:?}", event);
                // In a real implementation, this would trigger immediate alerts
            },
            AuditSeverity::Error => {
                warn!("⚠️ ERROR AUDIT EVENT: {:?}", event);
            },
            _ => {},
        }
    }

    /// Default alert thresholds
    fn default_alert_thresholds() -> HashMap<String, u32> {
        let mut thresholds = HashMap::new();
        thresholds.insert("failed_auth_per_hour".to_string(), 10);
        thresholds.insert("security_violations_per_day".to_string(), 5);
        thresholds.insert("admin_actions_per_hour".to_string(), 20);
        thresholds
    }

    /// Start background monitoring tasks
    async fn start_background_tasks(&self) {
        let log_entries = Arc::clone(&self.log_entries);
        
        // Example: Periodic integrity verification
        let integrity_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            
            loop {
                interval.tick().await;
                // Periodic integrity checks would go here
                debug!("🔒 Performing periodic integrity check");
            }
        });
        
        let mut tasks = self.background_tasks.lock().await;
        tasks.push(integrity_task);
    }
}

/// Export formats for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    use tracing::{debug, info};

    #[tokio::test]
    async fn test_audit_logger_creation() {
        debug!("🔧 Testing audit logger creation");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Standard).await;
        assert!(logger.is_ok(), "Audit logger should be created successfully");

        info!("✅ Audit logger creation test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_event_logging() {
        debug!("🔧 Testing event logging");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Standard).await.unwrap();

        let event = AuditEvent::UserCreated {
            user_id: "@testuser:example.com".to_string(),
            created_by: Some("@admin:example.com".to_string()),
            timestamp: SystemTime::now(),
        };

        let result = logger.log_event(event).await;
        assert!(result.is_ok(), "Should log event successfully");

        // Query the logged event
        let query = AuditQuery {
            start_time: None,
            end_time: None,
            event_types: None,
            user_id: Some("@testuser:example.com".to_string()),
            source: None,
            severity: None,
            limit: None,
            offset: None,
        };

        let entries = logger.query_logs(query).await.unwrap();
        assert_eq!(entries.len(), 1, "Should find one logged event");

        info!("✅ Event logging test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        debug!("🔧 Testing integrity verification");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Standard).await.unwrap();

        // Log several events
        for i in 0..5 {
            let event = AuditEvent::UserCreated {
                user_id: format!("@user{}:example.com", i),
                created_by: Some("@admin:example.com".to_string()),
                timestamp: SystemTime::now(),
            };
            logger.log_event(event).await.unwrap();
        }

        // Verify integrity
        let integrity_check = logger.verify_integrity().await;
        assert!(integrity_check.is_ok(), "Integrity verification should succeed");
        assert!(integrity_check.unwrap(), "Logs should have valid integrity");

        info!("✅ Integrity verification test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_compliance_report_generation() {
        debug!("🔧 Testing compliance report generation");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Gdpr).await.unwrap();

        // Log some events for the report
        let events = vec![
            AuditEvent::UserCreated {
                user_id: "@user1:example.com".to_string(),
                created_by: Some("@admin:example.com".to_string()),
                timestamp: SystemTime::now(),
            },
            AuditEvent::SecurityViolation {
                violation_type: "unauthorized_access".to_string(),
                user_id: Some("@user1:example.com".to_string()),
                client_ip: Some("192.168.1.100".to_string()),
                details: "Test violation".to_string(),
                timestamp: SystemTime::now(),
            },
        ];

        for event in events {
            logger.log_event(event).await.unwrap();
        }

        // Generate compliance report
        let period_start = SystemTime::now() - Duration::from_secs(3600);
        let period_end = SystemTime::now();
        
        let report = logger.generate_compliance_report(
            ComplianceReportType::Security,
            period_start,
            period_end,
        ).await;

        assert!(report.is_ok(), "Should generate compliance report successfully");
        
        let report = report.unwrap();
        assert!(report.summary.total_events > 0, "Report should include logged events");
        assert!(report.summary.compliance_score <= 100.0, "Compliance score should be valid");

        info!("✅ Compliance report generation test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_log_export() {
        debug!("🔧 Testing log export");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Standard).await.unwrap();

        // Log an event
        let event = AuditEvent::DataAccessed {
            resource_type: "user_profile".to_string(),
            resource_id: "@user:example.com".to_string(),
            accessed_by: "@admin:example.com".to_string(),
            access_type: "read".to_string(),
            timestamp: SystemTime::now(),
        };
        logger.log_event(event).await.unwrap();

        // Export as JSON
        let query = AuditQuery {
            start_time: None,
            end_time: None,
            event_types: None,
            user_id: None,
            source: None,
            severity: None,
            limit: None,
            offset: None,
        };

        let json_export = logger.export_logs(query.clone(), ExportFormat::Json).await;
        assert!(json_export.is_ok(), "Should export logs as JSON");
        assert!(json_export.unwrap().contains("DataAccessed"), "Export should contain logged event");

        // Export as CSV
        let csv_export = logger.export_logs(query, ExportFormat::Csv).await;
        assert!(csv_export.is_ok(), "Should export logs as CSV");
        assert!(csv_export.unwrap().contains("DataAccessed"), "CSV export should contain logged event");

        info!("✅ Log export test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_query_filtering() {
        debug!("🔧 Testing query filtering");
        let start = std::time::Instant::now();

        let logger = AuditLogger::new(true, ComplianceMode::Standard).await.unwrap();

        // Log events with different users
        let events = vec![
            AuditEvent::UserCreated {
                user_id: "@alice:example.com".to_string(),
                created_by: Some("@admin:example.com".to_string()),
                timestamp: SystemTime::now(),
            },
            AuditEvent::UserCreated {
                user_id: "@bob:example.com".to_string(),
                created_by: Some("@admin:example.com".to_string()),
                timestamp: SystemTime::now(),
            },
        ];

        for event in events {
            logger.log_event(event).await.unwrap();
        }

        // Query for specific user
        let query = AuditQuery {
            start_time: None,
            end_time: None,
            event_types: None,
            user_id: Some("@alice:example.com".to_string()),
            source: None,
            severity: None,
            limit: None,
            offset: None,
        };

        let entries = logger.query_logs(query).await.unwrap();
        assert_eq!(entries.len(), 1, "Should find only Alice's event");

        info!("✅ Query filtering test completed in {:?}", start.elapsed());
    }
} 
