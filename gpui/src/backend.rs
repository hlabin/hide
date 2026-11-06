//! Native Rust integration seam: reuse lpm's existing CLI/socket protocols.
//! The Tauri app remains the process owner; this client does not silently
//! initialize another daemon or duplicate project mutations.
use serde::Deserialize;
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
    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Invalid lpm {} response: {error}", action.verb()))?;
    if response.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(format!("lpm {} did not confirm success", action.verb()));
    }
    Ok(())
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
