use serde::{Deserialize, Serialize};

// ── Database Models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ScanJob {
    pub id: i64,
    pub scan_type: String,
    pub target: String,
    pub target_source: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub total_findings: i64,
    pub critical_count: i64,
    pub high_count: i64,
    pub medium_count: i64,
    pub low_count: i64,
    pub info_count: i64,
    pub tools_run: Option<String>,
    pub file_tree: Option<String>,
    pub current_tool: Option<String>,
    pub tools_total: Option<i64>,
    pub tools_completed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Finding {
    pub id: i64,
    pub scan_job_id: i64,
    pub tool: String,
    pub severity: String,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<i64>,
    pub cwe_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub raw_output: Option<String>,
    pub recommendation: Option<String>,
    pub text_range_start: Option<i64>,
    pub text_range_end: Option<i64>,
    pub status: Option<String>,
    pub author: Option<String>,
    pub rule_url: Option<String>,
    pub data_flow: Option<String>,
    pub issue_type: Option<String>,
}

/// Options sent from the frontend when generating a custom PDF report.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfExportOptions {
    /// "all" or a specific tool name — matches the active tool filter
    pub tool_filter: Option<String>,
    /// "all" or a specific severity — matches the active severity filter
    pub severity_filter: Option<String>,
    /// "all" or a specific issue type — matches the active issue-type filter
    pub issue_type_filter: Option<String>,
    /// Free-text search applied to title/tool/file
    pub search_query: Option<String>,
    /// Which columns to include.  None / empty = include all.
    /// Valid values: "severity", "tool", "title", "file", "cwe", "type", "cvss", "description"
    pub columns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Report {
    pub id: i64,
    pub scan_job_id: i64,
    pub format: String,
    pub file_path: String,
    pub created_at: String,
    pub emailed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ScanLog {
    pub id: i64,
    pub scan_job_id: i64,
    pub timestamp: String,
    pub level: String,
    pub tool: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Setting {
    pub id: i64,
    pub key: String,
    pub value: String,
}

// ── API Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_scans: i64,
    pub total_findings: i64,
    pub critical_findings: i64,
    pub high_findings: i64,
    pub medium_findings: i64,
    pub low_findings: i64,
    pub info_findings: i64,
    pub scans_today: i64,
    pub avg_duration: i64,
    pub most_common_tool: String,
    pub recent_scans: Vec<ScanJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatusResponse {
    pub status: String,
    pub current_tool: Option<String>,
    pub tools_total: Option<i64>,
    pub tools_completed: Option<i64>,
    pub total_findings: i64,
    pub critical_count: i64,
    pub high_count: i64,
    pub medium_count: i64,
    pub low_count: i64,
    pub info_count: i64,
    pub duration_seconds: Option<i64>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub tool: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub scan_type: String,
    pub target: String,
    pub target_source: String,
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseFoldersRequest {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseFoldersResponse {
    pub current_path: String,
    pub parent: Option<String>,
    pub entries: Vec<FolderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub letter: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub available: bool,
    pub category: String,
    pub web_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPreset {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFinding {
    pub tool: String,
    pub severity: String,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<i64>,
    pub cwe_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub recommendation: Option<String>,
    pub issue_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub info: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolScore {
    pub tool: String,
    pub score: i64,
    pub findings: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub info: i64,
    pub grade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanScoreResponse {
    pub overall_score: i64,
    pub overall_grade: String,
    pub tool_scores: Vec<ToolScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSettings {
    pub azure_devops_org: String,
    pub azure_devops_pat: String,
    pub azure_devops_project: String,
    pub smtp_server: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub email_from: String,
    pub email_to: String,
    pub sonarqube_url: String,
    pub sonarqube_token: String,
    pub sonarqube_project_key: String,
    pub sonarqube_exclusions: String,
    pub sonarqube_quality_profile: String,
    pub openvas_url: String,
    pub openvas_username: String,
    pub openvas_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSettingsRequest {
    pub settings: Vec<SettingPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    pub key: String,
    pub name: String,
    pub language: String,
    pub language_name: String,
    pub is_default: bool,
    pub active_rule_count: i64,
    pub is_built_in: bool,
}
