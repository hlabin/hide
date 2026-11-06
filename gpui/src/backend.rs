//! Native Rust integration seam: reuse lpm's existing CLI/socket protocols.
//! The Tauri app remains the process owner; this client does not silently
//! initialize another daemon or duplicate project mutations.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub services_running: usize,
    #[serde(default)]
    pub services_total: usize,
    #[serde(default)]
    pub is_remote: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub agents: Option<BTreeMap<String, usize>>,
    #[serde(default)]
    pub worktree: bool,
}

#[derive(Debug, Deserialize)]
struct ProjectList {
    projects: Vec<Project>,
}

pub fn list_projects() -> Result<Vec<Project>, String> {
    let executable = std::env::var_os("HIDE_LPM_CLI").unwrap_or_else(|| "lpm".into());
    let output = Command::new(executable)
        .args(["list", "--json"])
        .output()
        .map_err(|error| format!("Cannot launch lpm CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "lpm list failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_projects(&output.stdout)
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAction {
    Start,
    Stop,
}

impl ProjectAction {
    fn verb(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
        }
    }
}

/// Request a project state change through the existing lpm control socket.
///
/// The CLI delegates mutations to the running desktop app, which remains the
/// single owner of sessions and services. Never invoke a shell or interpolate
/// the project name into a command string.
pub fn change_project_state(name: &str, action: ProjectAction) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Cannot control a project with an empty name".into());
    }
    let executable = std::env::var_os("HIDE_LPM_CLI").unwrap_or_else(|| "lpm".into());
    let output = Command::new(executable)
        .args([action.verb(), name, "--json"])
        .output()
        .map_err(|error| format!("Cannot launch lpm CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "lpm {} failed ({}): {}",
            action.verb(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    confirm_project_action(&output.stdout, name, action)
}

/// Only accept a successful response for the project we actually requested.
/// This also protects against accidentally trusting unrelated CLI JSON.
fn confirm_project_action(bytes: &[u8], name: &str, action: ProjectAction) -> Result<(), String> {
    let response: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid lpm {} response: {error}", action.verb()))?;
    if response.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(format!("lpm {} did not confirm success", action.verb()));
    }
    if response.get("project").and_then(serde_json::Value::as_str) != Some(name) {
        return Err(format!("lpm {} confirmed a different project", action.verb()));
    }
    Ok(())
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub name: String,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub port: i64,
    #[serde(default)]
    pub running_source: String,
}

#[derive(Debug, Deserialize)]
struct ProjectDetails {
    name: String,
    #[serde(default)]
    services: Vec<Service>,
    #[serde(default)]
    panes: Vec<Pane>,
}

/// Live terminal pane metadata from `lpm project --json`.
/// No PTY contents are exposed by this endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pane {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub current_command: String,
    #[serde(default)]
    pub current_path: String,
    #[serde(default)]
    pub ended: bool,
    #[serde(default)]
    pub pid: i64,
}

#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub services: Vec<Service>,
    pub panes: Vec<Pane>,
}

/// Read service details using the CLI's resolved project view, rather than
/// guessing names from the aggregate lpm list output.
pub fn list_project_snapshot(project: &str) -> Result<ProjectSnapshot, String> {
    if project.trim().is_empty() {
        return Err("Cannot list services for an empty project".into());
    }
    let executable = std::env::var_os("HIDE_LPM_CLI").unwrap_or_else(|| "lpm".into());
    let output = Command::new(executable)
        .args(["project", project, "--json"])
        .output()
        .map_err(|error| format!("Cannot launch lpm CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "lpm project failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_project_snapshot(&output.stdout, project)
}

/// Compatibility helper for callers that only need services.
#[allow(dead_code)]
pub fn list_services(project: &str) -> Result<Vec<Service>, String> {
    list_project_snapshot(project).map(|snapshot| snapshot.services)
}

fn parse_project_snapshot(bytes: &[u8], project: &str) -> Result<ProjectSnapshot, String> {
    let details: ProjectDetails = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid lpm project response: {error}"))?;
    if details.name != project {
        return Err("lpm project returned a different project".into());
    }
    Ok(ProjectSnapshot { services: details.services, panes: details.panes })
}

#[cfg(test)]
fn parse_services(bytes: &[u8], project: &str) -> Result<Vec<Service>, String> {
    parse_project_snapshot(bytes, project).map(|snapshot| snapshot.services)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

impl ServiceAction {
    fn verb(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

/// Delegate service mutations to the running desktop app through the existing
/// CLI/socket contract. The UI never becomes a second service owner.
pub fn change_service_state(project: &str, service: &str, action: ServiceAction) -> Result<(), String> {
    if project.trim().is_empty() || service.trim().is_empty() {
        return Err("Project and service names must not be empty".into());
    }
    let executable = std::env::var_os("HIDE_LPM_CLI").unwrap_or_else(|| "lpm".into());
    let output = Command::new(executable)
        .args(["service", service, action.verb(), "-p", project, "--json"])
        .output()
        .map_err(|error| format!("Cannot launch lpm CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "lpm service {} failed ({}): {}",
            action.verb(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    confirm_service_action(&output.stdout, project, service, action)
}

fn confirm_service_action(bytes: &[u8], project: &str, service: &str, action: ServiceAction) -> Result<(), String> {
    let response: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid lpm service response: {error}"))?;
    if response.get("ok").and_then(serde_json::Value::as_bool) != Some(true)
        || response.get("project").and_then(serde_json::Value::as_str) != Some(project)
        || response.get("service").and_then(serde_json::Value::as_str) != Some(service)
        || response.get("op").and_then(serde_json::Value::as_str) != Some(action.verb())
    {
        return Err("lpm service did not confirm the requested operation".into());
    }
    Ok(())
}

impl Project {
    /// Human-readable summary from the same JSON contract as the existing CLI.
    pub fn agent_summary(&self) -> Option<String> {
        let agents = self.agents.as_ref()?;
        if agents.is_empty() { return None; }
        Some(agents.iter().map(|(status, count)| {
            format!("{count} {}", status.to_lowercase())
        }).collect::<Vec<_>>().join(", "))
    }
}

fn parse_projects(bytes: &[u8]) -> Result<Vec<Project>, String> {
    serde_json::from_slice::<ProjectList>(bytes)
        .map(|response| response.projects)
        .map_err(|error| format!("Invalid lpm list response: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_panes_from_existing_cli_contract() {
        let bytes = br#"{"name":"api","panes":[{"id":"%1","pid":812,"title":"api shell","currentCommand":"cargo test","currentPath":"/src/api","ended":false},{"id":"%2","ended":true}],"services":[]}"#;
        let snapshot = parse_project_snapshot(bytes, "api").unwrap();
        assert_eq!(snapshot.panes.len(), 2);
        assert_eq!(snapshot.panes[0].id, "%1");
        assert_eq!(snapshot.panes[0].current_command, "cargo test");
        assert_eq!(snapshot.panes[0].pid, 812);
        assert!(snapshot.panes[1].ended);
        assert_eq!(snapshot.panes[1].title, "");
        assert!(parse_project_snapshot(bytes, "other").is_err());
    }

    #[test]
    fn parses_project_services_from_existing_cli_contract() {
        let bytes = br#"{"name":"api","services":[{"name":"web","running":true,"port":3000,"runningSource":"port"},{"name":"worker","running":false}]}"#;
        let services = parse_services(bytes, "api").unwrap();
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "web");
        assert_eq!(services[0].port, 3000);
        assert!(services[0].running);
        assert!(!services[1].running);
        assert!(parse_services(bytes, "other").is_err());
    }

    #[test]
    fn service_acknowledgement_must_match_all_fields() {
        let bytes = br#"{"ok":true,"project":"api","service":"web","op":"restart"}"#;
        assert!(confirm_service_action(bytes, "api", "web", ServiceAction::Restart).is_ok());
        assert!(confirm_service_action(bytes, "api", "worker", ServiceAction::Restart).is_err());
        assert!(confirm_service_action(bytes, "api", "web", ServiceAction::Stop).is_err());
        assert!(confirm_service_action(b"{}", "api", "web", ServiceAction::Restart).is_err());
        assert!(change_service_state("api", " ", ServiceAction::Start).is_err());
    }

    #[test]
    fn actions_use_existing_cli_verbs() {
        assert_eq!(ProjectAction::Start.verb(), "start");
        assert_eq!(ProjectAction::Stop.verb(), "stop");
    }

    #[test]
    fn rejects_empty_project_names_without_launching_cli() {
        assert!(change_project_state("  ", ProjectAction::Stop).is_err());
    }

    #[test]
    fn rejects_unconfirmed_cli_output() {
        let result: serde_json::Value = serde_json::from_slice(br#"{"ok":false}"#).unwrap();
        assert_ne!(result.get("ok").and_then(serde_json::Value::as_bool), Some(true));
    }

    #[test]
    fn action_response_must_match_project() {
        assert!(confirm_project_action(br#"{"ok":true,"project":"api"}"#, "api", ProjectAction::Start).is_ok());
        assert!(confirm_project_action(br#"{"ok":true,"project":"other"}"#, "api", ProjectAction::Start).is_err());
        assert!(confirm_project_action(br#"{"ok":false,"project":"api"}"#, "api", ProjectAction::Stop).is_err());
        assert!(confirm_project_action(b"not json", "api", ProjectAction::Stop).is_err());
    }

    #[test]
    fn agent_summary_uses_sorted_status_counts() {
        let data = br#"{"projects":[{"name":"api","agents":{"Waiting":1,"Running":2},"worktree":true}]}"#;
        let projects = parse_projects(data).unwrap();
        assert_eq!(projects[0].agent_summary().as_deref(), Some("2 running, 1 waiting"));
        assert!(projects[0].worktree);
    }

    #[test]
    fn parses_real_cli_project_shape() {
        let data = br#"{"projects":[{"name":"api","label":"API","root":"/srv/api","running":true,"servicesRunning":1,"servicesTotal":2,"isRemote":false,"agents":null,"worktree":false,"error":null}]}"#;
        let projects = parse_projects(data).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "api");
        assert!(projects[0].running);
        assert_eq!(projects[0].services_total, 2);
    }

    #[test]
    fn malformed_json_returns_error() {
        assert!(parse_projects(b"{").is_err());
    }

    #[test]
    fn empty_project_list_is_valid() {
        assert!(parse_projects(br#"{"projects":[]}"#).unwrap().is_empty());
    }
}
