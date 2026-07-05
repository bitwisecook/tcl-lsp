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

//! SSH/scp transport for `f5 fetch`, built on the pure-Rust `russh` client.
//!
//! An in-process SSH client (no system `ssh` / `scp` / `sshpass` binaries):
//! [`russh`] runs the `tmsh save …` commands over an exec channel and
//! [`russh_sftp`] downloads the resulting artefact over the SFTP subsystem.
//! Authentication is by password — [`Credentials`] always resolves one.
//!
//! `russh` is async; the synchronous [`fetch`] entry point drives it on a
//! private current-thread Tokio runtime, wrapping the whole exchange in a
//! single [`tokio::time::timeout`].
//!
//! The transport talks to a *live* device, so — like the REST transport — it is
//! not exercised by the offline tests; only the pure command/path builders are
//! unit-tested.

use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::keys::ssh_key;
use russh::{ChannelMsg, Disconnect};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use tokio::io::AsyncReadExt as _;

use super::auth::Credentials;

/// The `tmsh` command that writes an SCF for `name`, and the path it lands at.
fn scf_save(name: &str) -> (String, String) {
    (
        format!("tmsh save sys config file {name} no-passphrase"),
        format!("/var/local/scf/{name}.scf"),
    )
}

/// The `tmsh` command that writes a UCS archive for `name`, and its path.
fn ucs_save(name: &str) -> (String, String) {
    (
        format!("tmsh save sys ucs {name}"),
        format!("/var/local/ucs/{name}.ucs"),
    )
}

/// Server-key handler: accept the presented host key unconditionally
/// (`StrictHostKeyChecking=accept-new` semantics — a fetch to a fresh device
/// should not fail on an unknown key).
struct AcceptServerKey;

impl client::Handler for AcceptServerKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Connect to the device and authenticate by password.
async fn connect(credentials: &Credentials) -> Result<client::Handle<AcceptServerKey>, String> {
    let config = Arc::new(client::Config::default());
    let addr = (credentials.host.as_str(), credentials.ssh_port);
    let mut handle = client::connect(config, addr, AcceptServerKey)
        .await
        .map_err(|e| format!("ssh connect to {}: {e}", credentials.host))?;
    let auth = handle
        .authenticate_password(credentials.user.as_str(), credentials.password.as_str())
        .await
        .map_err(|e| format!("ssh auth to {}: {e}", credentials.host))?;
    if !auth.success() {
        return Err(format!(
            "ssh auth to {}: password authentication failed for {}",
            credentials.host, credentials.user
        ));
    }
    Ok(handle)
}

/// Run `command` over an exec channel, returning its stdout. A non-zero remote
/// exit is surfaced with the collected stderr.
async fn run_command(
    handle: &client::Handle<AcceptServerKey>,
    command: &str,
) -> Result<String, String> {
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("ssh channel: {e}"))?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| format!("ssh exec: {e}"))?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit: Option<u32> = None;
    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
            ChannelMsg::ExtendedData { ref data, .. } => stderr.extend_from_slice(data),
            ChannelMsg::ExitStatus { exit_status } => exit = Some(exit_status),
            _ => {}
        }
    }

    if exit.is_some_and(|code| code != 0) {
        let detail = String::from_utf8_lossy(&stderr);
        let detail = detail.trim();
        let detail = if detail.is_empty() {
            String::from_utf8_lossy(&stdout).trim().to_owned()
        } else {
            detail.to_owned()
        };
        return Err(format!(
            "remote command `{command}` exited {}: {detail}",
            exit.unwrap_or_default()
        ));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

/// Download `remote_path` from the device over the SFTP subsystem.
async fn sftp_download(
    handle: &client::Handle<AcceptServerKey>,
    remote_path: &str,
) -> Result<Vec<u8>, String> {
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("ssh channel: {e}"))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("sftp subsystem: {e}"))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("sftp session: {e}"))?;
    let mut file = sftp
        .open_with_flags(remote_path, OpenFlags::READ)
        .await
        .map_err(|e| format!("sftp open {remote_path}: {e}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("sftp read {remote_path}: {e}"))?;
    Ok(bytes)
}

/// The full async fetch: connect, save the requested artefact(s), download
/// them, and (for a UCS-only fetch) reconstruct the SCF text locally.
async fn fetch_async(
    credentials: &Credentials,
    fmt: &str,
    name: &str,
) -> Result<(String, Option<Vec<u8>>), String> {
    let handle = connect(credentials).await?;

    let mut scf_text = String::new();
    let mut ucs_bytes: Option<Vec<u8>> = None;

    if matches!(fmt, "scf" | "both") {
        let (command, path) = scf_save(name);
        run_command(&handle, &command).await?;
        let bytes = sftp_download(&handle, &path).await?;
        scf_text = String::from_utf8_lossy(&bytes).into_owned();
    }

    if matches!(fmt, "ucs" | "both") {
        let (command, path) = ucs_save(name);
        run_command(&handle, &command).await?;
        let bytes = sftp_download(&handle, &path).await?;
        if scf_text.is_empty() {
            scf_text = tcl_bigip_io::ucs_to_scf(&bytes, false).map_err(|e| e.to_string())?;
        }
        ucs_bytes = Some(bytes);
    }

    let _ = handle.disconnect(Disconnect::ByApplication, "", "en").await;
    Ok((scf_text, ucs_bytes))
}

/// Pull the running config over SSH, returning `(scf_text, ucs_bytes)`.
///
/// `fmt` selects `"scf"`, `"ucs"`, or `"both"`. A `"ucs"`-only fetch
/// reconstructs the SCF text from the archive via [`tcl_bigip_io::ucs_to_scf`]
/// so downstream verbs always have a usable text artefact. A positive `timeout`
/// bounds the whole exchange; a non-positive value waits indefinitely.
///
/// # Errors
/// Returns a transport error string on a connection / auth failure, a non-zero
/// remote exit, a download failure, a timeout, or a UCS→SCF conversion failure.
pub fn fetch(
    credentials: &Credentials,
    fmt: &str,
    timeout: f64,
    name: &str,
) -> Result<(String, Option<Vec<u8>>), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("ssh runtime: {e}"))?;
    runtime.block_on(async {
        let work = fetch_async(credentials, fmt, name);
        if timeout > 0.0 {
            match tokio::time::timeout(Duration::from_secs_f64(timeout), work).await {
                Ok(result) => result,
                Err(_) => Err(format!("ssh fetch timed out after {timeout}s")),
            }
        } else {
            work.await
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scf_save_builds_tmsh_command_and_path() {
        let (command, path) = scf_save("f5_fetch_1");
        assert_eq!(
            command,
            "tmsh save sys config file f5_fetch_1 no-passphrase"
        );
        assert_eq!(path, "/var/local/scf/f5_fetch_1.scf");
    }

    #[test]
    fn ucs_save_builds_tmsh_command_and_path() {
        let (command, path) = ucs_save("f5_fetch_1");
        assert_eq!(command, "tmsh save sys ucs f5_fetch_1");
        assert_eq!(path, "/var/local/ucs/f5_fetch_1.ucs");
    }
}
