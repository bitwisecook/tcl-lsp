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

//! Credential resolution for the `f5` remote verbs.
//!
//! Resolution order (highest priority first):
//!
//! 1. Explicit CLI arguments (`--host`, `--user`, `--password`)
//! 2. Environment variables (`F5_HOST`, `F5_USER`, `F5_PASSWORD`)
//! 3. XDG config file (`$XDG_CONFIG_HOME/f5/hosts.toml` or
//!    `~/.config/f5/hosts.toml`) keyed by host alias
//! 4. Interactive prompt
//!
//! The error strings (`no host/user/password configured (...)`) use fixed
//! wording.

use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;

/// A fully resolved set of device credentials.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub host: String,
    pub user: String,
    pub password: String,
    pub port: u16,
    /// The SSH port used by the `ssh` / `auto` fetch transports.
    pub ssh_port: u16,
}

/// The XDG config directory holding `hosts.toml` (`$XDG_CONFIG_HOME/f5` or
/// `~/.config/f5`). Also used by the SSH transport to persist its
/// `known_hosts` store (see [`super::ssh`]).
pub(super) fn xdg_config_dir() -> PathBuf {
    if let Some(base) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(base).join("f5");
    }
    home_dir().join(".config").join("f5")
}

/// The XDG cache directory used for fetch artefacts (`$XDG_CACHE_HOME/f5` or
/// `~/.cache/f5`).
#[must_use]
pub fn xdg_cache_dir() -> PathBuf {
    if let Some(base) = env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(base).join("f5");
    }
    home_dir().join(".cache").join("f5")
}

fn home_dir() -> PathBuf {
    env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn hosts_config_path() -> PathBuf {
    xdg_config_dir().join("hosts.toml")
}

/// A single alias entry from `hosts.toml` (`[hosts.<alias>]` table).
type AliasEntry = BTreeMap<String, String>;

/// Minimal `hosts.toml` reader: parses `[hosts.<alias>]` tables and their
/// string / integer scalar keys. Full TOML is overkill (and the `toml` crate is
/// not a workspace dependency); only the host alias map is consulted, and only
/// on the live (untested) path. Unreadable / malformed files warn and yield an
/// empty map (graceful degradation).
fn load_hosts_config() -> BTreeMap<String, AliasEntry> {
    let path = hosts_config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let mut hosts: BTreeMap<String, AliasEntry> = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // Expect `hosts.<alias>`; ignore anything else.
            let parts: Vec<&str> = rest.split('.').collect();
            if parts.len() == 2 && parts[0].trim() == "hosts" {
                let alias = unquote(parts[1].trim());
                current = Some(alias.clone());
                hosts.entry(alias).or_default();
            } else {
                current = None;
            }
            continue;
        }
        if let (Some(alias), Some((key, value))) = (current.as_ref(), line.split_once('=')) {
            let key = key.trim().to_owned();
            let value = unquote(value.trim());
            hosts.entry(alias.clone()).or_default().insert(key, value);
        }
    }
    hosts
}

/// Strip matching single / double quotes from a bare TOML scalar.
fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        return s[1..s.len() - 1].to_owned();
    }
    s.to_owned()
}

/// Options threaded into [`resolve_credentials`].
pub struct ResolveOptions<'a> {
    pub host: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub port: Option<u16>,
    pub ssh_port: Option<u16>,
    pub interactive: bool,
}

/// Resolve a complete [`Credentials`] from layered sources.
///
/// # Errors
/// Returns the exact error strings (`"no host configured (set --host or
/// F5_HOST)"`, etc.) when a required field cannot be resolved non-interactively,
/// or an I/O message when an interactive prompt fails.
pub fn resolve_credentials(opts: &ResolveOptions) -> Result<Credentials, String> {
    let config = load_hosts_config();

    // Look up alias entry if *host* matches a configured alias.
    let mut alias_entry: AliasEntry = opts
        .host
        .and_then(|h| config.get(h))
        .cloned()
        .unwrap_or_default();

    let pick = |entry: &AliasEntry, field: &str, env_name: &str| -> Option<String> {
        if let Some(v) = env::var(env_name).ok().filter(|v| !v.is_empty()) {
            return Some(v);
        }
        if let Some(v) = entry.get(field) {
            return Some(v.clone());
        }
        None
    };

    let mut resolved_host = opts
        .host
        .map(str::to_owned)
        .or_else(|| pick(&alias_entry, "host", "F5_HOST"));
    if resolved_host.is_none() {
        if !opts.interactive {
            return Err("no host configured (set --host or F5_HOST)".to_owned());
        }
        let h = prompt_line("F5 host: ")?;
        if h.is_empty() {
            return Err("host is required".to_owned());
        }
        resolved_host = Some(h);
    }
    let mut resolved_host = resolved_host.expect("resolved_host is set above");
    // If host is an alias, replace with the real address from the config.
    if let Some(entry) = config.get(&resolved_host)
        && let Some(real) = entry.get("host")
    {
        alias_entry = entry.clone();
        resolved_host = real.clone();
    }

    let resolved_user = if let Some(u) = opts
        .user
        .map(str::to_owned)
        .or_else(|| pick(&alias_entry, "user", "F5_USER"))
    {
        u
    } else {
        if !opts.interactive {
            return Err("no user configured (set --user or F5_USER)".to_owned());
        }
        let u = prompt_line(&format!("F5 user [{resolved_host}]: "))?;
        if u.is_empty() {
            return Err("user is required".to_owned());
        }
        u
    };

    let resolved_password = if let Some(p) = opts
        .password
        .map(str::to_owned)
        .or_else(|| pick(&alias_entry, "password", "F5_PASSWORD"))
    {
        p
    } else {
        if !opts.interactive {
            return Err("no password configured (set --password or F5_PASSWORD)".to_owned());
        }
        let p =
            rpassword::prompt_password(format!("Password for {resolved_user}@{resolved_host}: "))
                .map_err(|e| e.to_string())?;
        if p.is_empty() {
            return Err("password is required".to_owned());
        }
        p
    };

    let resolved_port = if let Some(port) = opts.port {
        port
    } else {
        pick(&alias_entry, "port", "F5_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(443)
    };

    let resolved_ssh_port = if let Some(port) = opts.ssh_port {
        port
    } else {
        pick(&alias_entry, "ssh_port", "F5_SSH_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(22)
    };

    Ok(Credentials {
        host: resolved_host,
        user: resolved_user,
        password: resolved_password,
        port: resolved_port,
        ssh_port: resolved_ssh_port,
    })
}

/// Print a prompt to stdout and read one whitespace-stripped line from stdin.
fn prompt_line(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(line.trim().to_owned())
}
