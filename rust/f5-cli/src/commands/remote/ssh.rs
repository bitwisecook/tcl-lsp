//! SSH/scp transport for `f5 fetch`.
//!
//! Wraps the system `ssh` and `scp` binaries via [`std::process::Command`] to
//! avoid an in-process SSH client (which would pull in `russh` and the `unsafe`
//! / C dependencies the workspace forbids). Falls back with a clear error if
//! either binary is not on `PATH`.
//!
//! Password authentication uses `sshpass` when available; otherwise the caller
//! is expected to have key-based auth configured (and the `password` field on
//! [`Credentials`] is ignored).
//!
//! The transport talks to a *live* device, so — like the REST transport — it is
//! not exercised by the offline tests; only the `PATH`-resolution error path and
//! the pure argv builders are unit-tested.

use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::auth::Credentials;

/// Poll interval while waiting on a spawned `ssh` / `scp` child.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Resolve `binary` on `PATH`, returning its absolute path or a not-found error.
fn which_or_err(binary: &str) -> Result<String, String> {
    which(binary).ok_or_else(|| format!("'{binary}' not found on PATH"))
}

/// Search the `PATH` environment for an executable file named `binary`.
/// Returns the first match.
fn which(binary: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(binary);
        if is_executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// Whether `path` is a regular file with an execute bit set (owner/group/other).
fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The shared `ssh` connection options: accept new host keys, allow an
/// interactive password (`BatchMode` off), on the configured SSH port.
fn ssh_base_args(credentials: &Credentials) -> Vec<String> {
    vec![
        "-o".to_owned(),
        "StrictHostKeyChecking=accept-new".to_owned(),
        "-o".to_owned(),
        "BatchMode=no".to_owned(),
        "-p".to_owned(),
        credentials.ssh_port.to_string(),
    ]
}

/// Build the full argv for `ssh <target> <remote_command>`, wrapping in
/// `sshpass -e` when a password helper is available. `program` is argv[0].
fn ssh_argv(
    ssh: &str,
    sshpass: Option<&str>,
    credentials: &Credentials,
    remote_command: &str,
) -> Vec<String> {
    let target = format!("{}@{}", credentials.user, credentials.host);
    let mut argv = Vec::new();
    if let Some(sshpass) = sshpass {
        argv.push(sshpass.to_owned());
        argv.push("-e".to_owned());
    }
    argv.push(ssh.to_owned());
    argv.extend(ssh_base_args(credentials));
    argv.push(target);
    argv.push(remote_command.to_owned());
    argv
}

/// Build the full argv for `scp <target>:<remote_path> <local_path>`, wrapping
/// in `sshpass -e` when available. `-O` forces the legacy scp protocol that
/// BIG-IP commonly needs. `program` is argv[0].
fn scp_argv(
    scp: &str,
    sshpass: Option<&str>,
    credentials: &Credentials,
    remote_path: &str,
    local_path: &Path,
) -> Vec<String> {
    let target = format!("{}@{}:{}", credentials.user, credentials.host, remote_path);
    let mut argv = Vec::new();
    if let Some(sshpass) = sshpass {
        argv.push(sshpass.to_owned());
        argv.push("-e".to_owned());
    }
    argv.push(scp.to_owned());
    argv.push("-O".to_owned());
    argv.push("-o".to_owned());
    argv.push("StrictHostKeyChecking=accept-new".to_owned());
    argv.push("-P".to_owned());
    argv.push(credentials.ssh_port.to_string());
    argv.push(target);
    argv.push(local_path.to_string_lossy().into_owned());
    argv
}

/// A captured subprocess result: exit status plus decoded stdout / stderr.
struct Captured {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Spawn `argv` (with `SSHPASS` set when `password` is `Some`), enforcing
/// `timeout` seconds, and capture stdout + stderr. Reader threads drain both
/// pipes so a chatty child can never dead-lock on a full pipe buffer.
///
/// A non-positive `timeout` waits indefinitely (matching an unset timeout).
fn run_capture(
    argv: &[String],
    password: Option<&str>,
    timeout: f64,
    label: &str,
) -> Result<Captured, String> {
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(password) = password {
        cmd.env("SSHPASS", password);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{label}: failed to spawn {}: {e}", argv[0]))?;

    let mut out_pipe = child.stdout.take().expect("stdout piped");
    let mut err_pipe = child.stderr.take().expect("stderr piped");
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });

    let deadline = (timeout > 0.0).then(|| Instant::now() + Duration::from_secs_f64(timeout));
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| format!("{label}: {e}"))? {
            break status;
        }
        if deadline.is_some_and(|dl| Instant::now() >= dl) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err(format!("{label}: timed out after {timeout}s"));
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(Captured {
        status,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Run `remote_command` on the device over SSH, returning its stdout.
fn run_ssh(
    credentials: &Credentials,
    remote_command: &str,
    timeout: f64,
) -> Result<String, String> {
    let ssh = which_or_err("ssh")?;
    let sshpass = which("sshpass");
    let target = format!("{}@{}", credentials.user, credentials.host);
    let argv = ssh_argv(&ssh, sshpass.as_deref(), credentials, remote_command);
    let password = sshpass.as_ref().map(|_| credentials.password.as_str());
    let label = format!("ssh '{target}'");
    let captured = run_capture(&argv, password, timeout, &label)?;
    if !captured.status.success() {
        return Err(exit_message("ssh", &target, &captured));
    }
    Ok(captured.stdout)
}

/// Copy `remote_path` from the device to `local_path` via scp.
fn run_scp(
    credentials: &Credentials,
    remote_path: &str,
    local_path: &Path,
    timeout: f64,
) -> Result<(), String> {
    let scp = which_or_err("scp")?;
    let sshpass = which("sshpass");
    let target = format!("{}@{}:{}", credentials.user, credentials.host, remote_path);
    let argv = scp_argv(
        &scp,
        sshpass.as_deref(),
        credentials,
        remote_path,
        local_path,
    );
    let password = sshpass.as_ref().map(|_| credentials.password.as_str());
    let label = format!("scp '{target}'");
    let captured = run_capture(&argv, password, timeout, &label)?;
    if !captured.status.success() {
        return Err(exit_message("scp", &target, &captured));
    }
    Ok(())
}

/// Format the `<tool> '<target>': exit <code>: <msg>` error for a non-zero
/// exit, preferring stderr over stdout for the trailing message.
fn exit_message(tool: &str, target: &str, captured: &Captured) -> String {
    let code = captured
        .status
        .code()
        .map_or_else(|| "signal".to_owned(), |c| c.to_string());
    let detail = {
        let err = captured.stderr.trim();
        if err.is_empty() {
            captured.stdout.trim()
        } else {
            err
        }
    };
    format!("{tool} '{target}': exit {code}: {detail}")
}

/// Pull the running config over SSH/scp, returning `(scf_text, ucs_bytes)`.
///
/// `fmt` selects `"scf"`, `"ucs"`, or `"both"`. A `"ucs"`-only fetch
/// reconstructs the SCF text from the archive via [`tcl_bigip_io::ucs_to_scf`]
/// so downstream verbs always have a usable text artefact.
///
/// # Errors
/// Returns a transport error string on a missing `ssh` / `scp`, a non-zero
/// remote exit, a timeout, or a UCS→SCF conversion failure.
pub fn fetch(
    credentials: &Credentials,
    fmt: &str,
    timeout: f64,
    name: &str,
) -> Result<(String, Option<Vec<u8>>), String> {
    let tmp = tempdir()?;

    let mut scf_text = String::new();
    let mut ucs_bytes: Option<Vec<u8>> = None;

    if matches!(fmt, "scf" | "both") {
        run_ssh(
            credentials,
            &format!("tmsh save sys config file {name} no-passphrase"),
            timeout,
        )?;
        let local_scf = tmp.path().join(format!("{name}.scf"));
        run_scp(
            credentials,
            &format!("/var/local/scf/{name}.scf"),
            &local_scf,
            timeout,
        )?;
        let bytes = std::fs::read(&local_scf).map_err(|e| e.to_string())?;
        scf_text = String::from_utf8_lossy(&bytes).into_owned();
    }

    if matches!(fmt, "ucs" | "both") {
        run_ssh(credentials, &format!("tmsh save sys ucs {name}"), timeout)?;
        let local_ucs = tmp.path().join(format!("{name}.ucs"));
        run_scp(
            credentials,
            &format!("/var/local/ucs/{name}.ucs"),
            &local_ucs,
            timeout,
        )?;
        let bytes = std::fs::read(&local_ucs).map_err(|e| e.to_string())?;
        if scf_text.is_empty() {
            scf_text = tcl_bigip_io::ucs_to_scf(&bytes, false).map_err(|e| e.to_string())?;
        }
        ucs_bytes = Some(bytes);
    }

    Ok((scf_text, ucs_bytes))
}

/// A self-cleaning temporary directory (RAII), created under the system temp
/// dir with a `f5fetch-` prefix. Avoids a `tempfile` crate dependency for the
/// one place the SSH transport needs scratch space.
struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Create a fresh scratch directory, retrying on the rare name collision.
fn tempdir() -> Result<TempDir, String> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    for attempt in 0..1024u32 {
        let candidate = base.join(format!("f5fetch-{pid}-{nanos}-{attempt}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(TempDir { path: candidate }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("could not create a temporary directory for the SSH fetch".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds() -> Credentials {
        Credentials {
            host: "bigip.example.com".to_owned(),
            user: "admin".to_owned(),
            password: "secret".to_owned(),
            port: 443,
            ssh_port: 2222,
        }
    }

    #[test]
    fn ssh_argv_without_sshpass() {
        let argv = ssh_argv(
            "/usr/bin/ssh",
            None,
            &creds(),
            "tmsh save sys ucs f5_fetch_1",
        );
        assert_eq!(
            argv,
            vec![
                "/usr/bin/ssh",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "BatchMode=no",
                "-p",
                "2222",
                "admin@bigip.example.com",
                "tmsh save sys ucs f5_fetch_1",
            ]
        );
    }

    #[test]
    fn ssh_argv_with_sshpass_prefixes_helper() {
        let argv = ssh_argv("/usr/bin/ssh", Some("/usr/bin/sshpass"), &creds(), "uptime");
        assert_eq!(&argv[..3], &["/usr/bin/sshpass", "-e", "/usr/bin/ssh"]);
        assert_eq!(argv.last().unwrap(), "uptime");
    }

    // The `ssh` / `scp` not-on-PATH error path is covered end-to-end by the
    // `remote_parity` integration test (`ssh_transport_missing_binary`), which
    // runs the built binary with a controlled `PATH` in a child process — the
    // workspace forbids the `unsafe` global-env mutation an in-process test of
    // `which` would need.

    #[test]
    fn scp_argv_uses_legacy_protocol_and_uppercase_port() {
        let argv = scp_argv(
            "/usr/bin/scp",
            None,
            &creds(),
            "/var/local/scf/f5_fetch_1.scf",
            Path::new("/tmp/out.scf"),
        );
        assert_eq!(
            argv,
            vec![
                "/usr/bin/scp",
                "-O",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-P",
                "2222",
                "admin@bigip.example.com:/var/local/scf/f5_fetch_1.scf",
                "/tmp/out.scf",
            ]
        );
    }

    #[test]
    fn exit_message_prefers_stderr() {
        // A non-zero exit with stderr set prefers it over stdout.
        let out = Command::new("sh")
            .args(["-c", "echo out; echo err 1>&2; exit 3"])
            .output()
            .expect("spawn sh");
        let captured = Captured {
            status: out.status,
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        };
        assert_eq!(
            exit_message("ssh", "admin@host", &captured),
            "ssh 'admin@host': exit 3: err"
        );
    }
}
