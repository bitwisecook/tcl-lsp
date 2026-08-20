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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::keys::{known_hosts, ssh_key};
use russh::{ChannelMsg, Disconnect};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use tokio::io::AsyncReadExt as _;

use super::auth::{Credentials, xdg_config_dir};

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

/// Where accepted SSH host keys are persisted, in OpenSSH `known_hosts`
/// format — under the tool's own config dir (`$XDG_CONFIG_HOME/f5` or
/// `~/.config/f5`), independent of the user's real `~/.ssh/known_hosts`.
fn known_hosts_path() -> PathBuf {
    xdg_config_dir().join("known_hosts")
}

/// The result of checking a presented host key against the persisted store.
#[derive(Debug, PartialEq)]
enum HostKeyCheck {
    /// Matches the key previously recorded for this host: safe to proceed.
    Known,
    /// No entry recorded for this host yet (`StrictHostKeyChecking=accept-new`
    /// semantics): the caller should persist the key and proceed.
    New,
    /// A *different* key is on record for this host — a man-in-the-middle, or
    /// the device was re-imaged / re-keyed. Must not proceed silently.
    Changed { line: usize },
    /// The `known_hosts` store could not be read or parsed for some other
    /// reason (I/O error, corrupt entry). Fail closed rather than guess.
    Unreadable(String),
}

/// Check `key` for `host:port` against the `known_hosts`-format store at
/// `path`, without side effects.
fn verify_host_key(host: &str, port: u16, key: &ssh_key::PublicKey, path: &Path) -> HostKeyCheck {
    match known_hosts::check_known_hosts_path(host, port, key, path) {
        Ok(true) => HostKeyCheck::Known,
        Ok(false) => HostKeyCheck::New,
        Err(russh::keys::Error::KeyChanged { line }) => HostKeyCheck::Changed { line },
        Err(e) => HostKeyCheck::Unreadable(e.to_string()),
    }
}

/// Decide whether to proceed with `key` for `host:port`, persisting it to
/// `path` on first sight. Fails closed (returns `false`) whenever trust can't
/// actually be established: a recorded mismatch, an unreadable store, *or* a
/// new key that could not be written — trust-on-first-use only holds if the
/// key is actually persisted, since a key that silently fails to save would
/// leave this host looking "unknown" (not "changed") on every later
/// connection, defeating the check entirely.
fn accept_host_key(host: &str, port: u16, key: &ssh_key::PublicKey, path: &Path) -> bool {
    match verify_host_key(host, port, key, path) {
        HostKeyCheck::Known => true,
        HostKeyCheck::New => match known_hosts::learn_known_hosts_path(host, port, key, path) {
            Ok(()) => true,
            Err(e) => {
                eprintln!(
                    "error: could not record host key for {host} in {}: {e} — refusing to \
                     connect, since the key could not be persisted for future verification",
                    path.display()
                );
                false
            }
        },
        HostKeyCheck::Changed { line } => {
            eprintln!(
                "error: host key for {host} does not match the key recorded at {}:{line} — \
                 refusing to connect. This may indicate a man-in-the-middle attack; if the \
                 device's key legitimately changed, remove the stale entry from that file.",
                path.display()
            );
            false
        }
        HostKeyCheck::Unreadable(e) => {
            eprintln!(
                "error: could not verify the host key for {host} against {}: {e}",
                path.display()
            );
            false
        }
    }
}

/// Server-key handler: verifies the presented host key against the persisted
/// `known_hosts` store (`StrictHostKeyChecking=accept-new` semantics — an
/// unknown host is trusted on first use and its key recorded; a *known* host
/// presenting a *different* key is rejected rather than silently accepted).
struct AcceptServerKey {
    host: String,
    port: u16,
    known_hosts_path: PathBuf,
}

impl client::Handler for AcceptServerKey {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> {
        std::future::ready(Ok(accept_host_key(
            &self.host,
            self.port,
            server_public_key,
            &self.known_hosts_path,
        )))
    }
}

/// Connect to the device and authenticate by password.
async fn connect(credentials: &Credentials) -> Result<client::Handle<AcceptServerKey>, String> {
    let config = Arc::new(client::Config::default());
    let addr = (credentials.host.as_str(), credentials.ssh_port);
    let handler = AcceptServerKey {
        host: credentials.host.clone(),
        port: credentials.ssh_port,
        known_hosts_path: known_hosts_path(),
    };
    let mut handle = client::connect(config, addr, handler)
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    // Two distinct ed25519 test-vector public keys (arbitrary, not secrets —
    // the same pair `russh` uses in its own `known_hosts` test suite).
    const KEY_A: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ";
    const KEY_B: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIA6rWI3G1sz07DnfFlrouTcysQlj2P+jpNSOEWD9OJ3X";

    fn key(base64: &str) -> ssh_key::PublicKey {
        russh::keys::parse_public_key_base64(base64).expect("parse test public key")
    }

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A throwaway directory under the system temp dir, removed on drop.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut path = std::env::temp_dir();
            path.push(format!("f5-cli-ssh-known-hosts-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn known_hosts(&self) -> PathBuf {
            self.path.join("known_hosts")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn verify_host_key_reports_new_for_unknown_host() {
        let dir = TempDir::new();
        assert_eq!(
            verify_host_key("bigip.example.com", 22, &key(KEY_A), &dir.known_hosts()),
            HostKeyCheck::New
        );
    }

    #[test]
    fn verify_host_key_reports_new_when_store_does_not_exist() {
        let dir = TempDir::new();
        // The directory (and file) were never created — a fresh install.
        let missing = dir.path.join("does-not-exist").join("known_hosts");
        assert_eq!(
            verify_host_key("bigip.example.com", 22, &key(KEY_A), &missing),
            HostKeyCheck::New
        );
    }

    #[test]
    fn verify_host_key_reports_known_after_learning() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip.example.com", 22, &key(KEY_A), &path)
            .expect("learn host key");
        assert_eq!(
            verify_host_key("bigip.example.com", 22, &key(KEY_A), &path),
            HostKeyCheck::Known
        );
    }

    #[test]
    fn verify_host_key_reports_changed_for_a_different_key() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip.example.com", 22, &key(KEY_A), &path)
            .expect("learn host key");
        assert!(matches!(
            verify_host_key("bigip.example.com", 22, &key(KEY_B), &path),
            HostKeyCheck::Changed { .. }
        ));
    }

    #[test]
    fn verify_host_key_is_new_for_a_different_host_in_the_same_store() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip-a.example.com", 22, &key(KEY_A), &path)
            .expect("learn host key");
        // A second, unrelated host isn't affected by the first host's entry —
        // it gets its own trust-on-first-use, not a spurious `Changed`.
        assert_eq!(
            verify_host_key("bigip-b.example.com", 22, &key(KEY_B), &path),
            HostKeyCheck::New
        );
    }

    #[test]
    fn verify_host_key_distinguishes_non_standard_ports() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip.example.com", 2222, &key(KEY_A), &path)
            .expect("learn host key");
        // Same host, default port 22: no entry recorded for *that* port/host
        // pair, so it's still new rather than matching the :2222 entry.
        assert_eq!(
            verify_host_key("bigip.example.com", 22, &key(KEY_A), &path),
            HostKeyCheck::New
        );
        assert_eq!(
            verify_host_key("bigip.example.com", 2222, &key(KEY_A), &path),
            HostKeyCheck::Known
        );
    }

    #[test]
    fn accept_host_key_accepts_and_persists_a_new_host() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        assert!(accept_host_key("bigip.example.com", 22, &key(KEY_A), &path));
        assert_eq!(
            verify_host_key("bigip.example.com", 22, &key(KEY_A), &path),
            HostKeyCheck::Known
        );
    }

    #[test]
    fn accept_host_key_accepts_a_matching_known_host() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip.example.com", 22, &key(KEY_A), &path)
            .expect("learn host key");
        assert!(accept_host_key("bigip.example.com", 22, &key(KEY_A), &path));
    }

    #[test]
    fn accept_host_key_rejects_a_changed_key() {
        let dir = TempDir::new();
        let path = dir.known_hosts();
        known_hosts::learn_known_hosts_path("bigip.example.com", 22, &key(KEY_A), &path)
            .expect("learn host key");
        assert!(!accept_host_key(
            "bigip.example.com",
            22,
            &key(KEY_B),
            &path
        ));
    }

    #[test]
    fn accept_host_key_fails_closed_when_the_store_cannot_be_written() {
        let dir = TempDir::new();
        // Make the known_hosts *parent* a regular file, so `create_dir_all`
        // inside `learn_known_hosts_path` cannot succeed regardless of
        // privilege level (unlike a plain permission bit, this also fails
        // when tests run as root).
        let blocked_parent = dir.path.join("blocked");
        std::fs::write(&blocked_parent, b"not a directory").expect("create blocking file");
        let path = blocked_parent.join("known_hosts");

        // Without the fail-closed fix this would return `true` despite never
        // having persisted the key, silently defeating verification on every
        // later connection to this host.
        assert!(!accept_host_key(
            "bigip.example.com",
            22,
            &key(KEY_A),
            &path
        ));
    }
}
