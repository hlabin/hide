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

fn parse_projects(bytes: &[u8]) -> Result<Vec<Project>, String> {
    serde_json::from_slice::<ProjectList>(bytes)
        .map(|response| response.projects)
        .map_err(|error| format!("Invalid lpm list response: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
