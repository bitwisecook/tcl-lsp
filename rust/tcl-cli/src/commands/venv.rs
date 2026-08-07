// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `tcl venv` verb group — virtual environment management.
//!
//! Thin wrappers over `tcl_pkg::venv`. Handler-level errors print
//! `error: <msg>` to stderr and return exit code 1.

// Handlers return `anyhow::Result<u8>` for a uniform dispatch signature even
// when a given verb cannot fail; the wrap is the interface contract.
#![allow(clippy::unnecessary_wraps)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tcl_pkg::ui;
use tcl_pkg::venv::{CreateOptions, create_venv, delete_venv, read_venv_config, update_venv};

use crate::cli::VenvCommand;

/// Dispatch a `tcl venv` sub-action.
pub fn run(action: &VenvCommand) -> anyhow::Result<u8> {
    match action {
        VenvCommand::Create {
            path,
            tcl,
            system_site_packages,
            prompt,
            force,
            json,
        } => run_create(
            path,
            tcl.as_deref(),
            *system_site_packages,
            prompt.as_deref(),
            *force,
            *json,
        ),
        VenvCommand::Delete { path, force, json } => run_delete(path, *force, *json),
        VenvCommand::Info { path, json } => run_info(path, *json),
        VenvCommand::Activate { path, shell } => run_activate(path, shell.as_deref()),
        VenvCommand::Deactivate { shell } => run_deactivate(shell.as_deref()),
        VenvCommand::List { json } => run_list(*json),
        VenvCommand::Update { path, tcl, json } => run_update(path, tcl, *json),
        VenvCommand::Run { path, command } => run_in_venv(path, command),
    }
}

fn run_create(
    path: &Path,
    tcl: Option<&str>,
    system_site_packages: bool,
    prompt: Option<&str>,
    force: bool,
    json: bool,
) -> anyhow::Result<u8> {
    let colour = ui::use_colour_for_json(json);
    if force && path.exists() {
        // Only clobber an existing *venv* — an existing directory that isn't a
        // tclpkg venv (a typo'd path to a real project dir) must not be silently
        // `rm -rf`'d, mirroring `delete_venv`'s data-loss guard.
        if path.join("tclvenv.cfg").is_file() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            eprintln!(
                "error: refusing to overwrite {}: not a tclpkg venv (missing tclvenv.cfg)",
                path.display()
            );
            return Ok(1);
        }
    }
    let opts = CreateOptions {
        tcl_version: tcl,
        system_site_packages,
        prompt,
        project_root: None,
    };
    let created = match create_venv(path, &opts) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if json {
        println!(
            "{}",
            ui::json_output(&json!({"path": created.to_string_lossy()}))
        );
    } else {
        println!(
            "{}",
            ui::ok(&format!("created {}", created.display()), colour)
        );
        println!(
            "{}",
            ui::dim(
                &format!("  activate: source {}/bin/activate", created.display()),
                colour
            )
        );
    }
    Ok(0)
}

fn run_delete(path: &Path, force: bool, json: bool) -> anyhow::Result<u8> {
    let colour = ui::use_colour_for_json(json);
    if let Err(e) = delete_venv(path, force) {
        eprintln!("error: {e}");
        return Ok(1);
    }
    if json {
        println!(
            "{}",
            ui::json_output(&json!({"deleted": path.to_string_lossy()}))
        );
    } else {
        println!("{}", ui::ok(&format!("deleted {}", path.display()), colour));
    }
    Ok(0)
}

fn run_info(path: &Path, json: bool) -> anyhow::Result<u8> {
    let config = match read_venv_config(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if json {
        let mut map = serde_json::Map::new();
        for (k, v) in &config {
            map.insert(k.clone(), Value::String(v.clone()));
        }
        println!("{}", ui::json_output(&Value::Object(map)));
    } else {
        let mut sorted = config;
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, value) in &sorted {
            println!("  {key:30}  {value}");
        }
    }
    Ok(0)
}

fn run_activate(path: &Path, shell: Option<&str>) -> anyhow::Result<u8> {
    let venv_path = absolutise(path);
    let script_path = if shell == Some("fish") {
        venv_path.join("bin").join("activate.fish")
    } else {
        venv_path.join("bin").join("activate")
    };
    if !script_path.is_file() {
        eprintln!(
            "error: activation script not found: {}",
            script_path.display()
        );
        return Ok(1);
    }
    match std::fs::read_to_string(&script_path) {
        Ok(text) => {
            println!("{text}");
            Ok(0)
        }
        Err(e) => {
            eprintln!("error: {e}");
            Ok(1)
        }
    }
}

fn run_deactivate(_shell: Option<&str>) -> anyhow::Result<u8> {
    println!("deactivate");
    Ok(0)
}

fn run_list(json: bool) -> anyhow::Result<u8> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut candidates = vec![cwd.join(".venv"), cwd.join("venv")];
    let home_pool = tcl_pkg::venv_pool_dir();
    if home_pool.is_dir()
        && let Ok(entries) = std::fs::read_dir(&home_pool)
    {
        let mut extra: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        extra.sort();
        candidates.extend(extra);
    }

    let active = std::env::var_os("TCL_VENV").map(PathBuf::from);
    let mut found: Vec<(String, String, String, String)> = Vec::new();
    for candidate in candidates {
        if !candidate.join("tclvenv.cfg").is_file() {
            continue;
        }
        let config = read_venv_config(&candidate).unwrap_or_default();
        let get = |key: &str| -> Option<String> {
            config
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        let is_active = active
            .as_ref()
            .is_some_and(|a| a.canonicalize().ok() == candidate.canonicalize().ok());
        let prompt = get("prompt").unwrap_or_else(|| {
            candidate
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
        });
        found.push((
            candidate.to_string_lossy().into_owned(),
            get("tcl_version").unwrap_or_else(|| "?".to_string()),
            prompt,
            if is_active {
                "yes".to_string()
            } else {
                String::new()
            },
        ));
    }

    if json {
        let arr: Vec<Value> = found
            .iter()
            .map(|(path, tcl, prompt, active)| {
                json!({"path": path, "tcl_version": tcl, "prompt": prompt, "active": active})
            })
            .collect();
        println!("{}", ui::json_output(&Value::Array(arr)));
    } else if found.is_empty() {
        println!("  No virtual environments found.");
    } else {
        println!(
            "{:<30} {:<12} {:<10} {:<6}",
            "PATH", "TCL", "PROMPT", "ACTIVE"
        );
        for (path, tcl, prompt, active) in &found {
            println!("{path:<30} {tcl:<12} {prompt:<10} {active:<6}");
        }
    }
    Ok(0)
}

fn run_update(path: &Path, tcl: &str, json: bool) -> anyhow::Result<u8> {
    let colour = ui::use_colour_for_json(json);
    let venv_path = absolutise(path);
    let tclsh = match update_venv(&venv_path, tcl) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if json {
        println!(
            "{}",
            ui::json_output(&json!({
                "path": venv_path.to_string_lossy(),
                "tcl": tcl,
                "tclsh": tclsh.to_string_lossy(),
            }))
        );
    } else {
        println!(
            "{}",
            ui::ok(
                &format!(
                    "updated {} to tclsh {tcl} ({})",
                    venv_path.display(),
                    tclsh.display()
                ),
                colour
            )
        );
    }
    Ok(0)
}

fn run_in_venv(path: &Path, command: &[String]) -> anyhow::Result<u8> {
    let venv_path = absolutise(path);
    if !venv_path.join("tclvenv.cfg").is_file() {
        eprintln!("error: not a tclpkg venv: {}", venv_path.display());
        return Ok(1);
    }
    if command.is_empty() {
        eprintln!("error: no command specified after --");
        return Ok(1);
    }

    let lib_dir = venv_path.join("lib");
    let existing = std::env::var("TCLLIBPATH").unwrap_or_default();
    let tcllibpath = if existing.is_empty() {
        lib_dir.to_string_lossy().into_owned()
    } else {
        format!("{} {existing}", lib_dir.display())
    };
    let bin_dir = venv_path.join("bin");
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = bin_dir.into_os_string();
    new_path.push(if cfg!(windows) { ";" } else { ":" });
    new_path.push(&path_env);

    let status = Command::new(&command[0])
        .args(&command[1..])
        .env("TCL_VENV", &venv_path)
        .env("TCLLIBPATH", tcllibpath)
        .env("PATH", new_path)
        .status();
    match status {
        Ok(s) => Ok(u8::try_from(s.code().unwrap_or(1)).unwrap_or(1)),
        Err(e) => {
            eprintln!("error: {e}");
            Ok(1)
        }
    }
}

fn absolutise(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    }
}
