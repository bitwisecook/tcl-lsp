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

//! `tcl-sandbox` — portable, deprivileged execution for the `tcl pkg` package
//! manager.
//!
//! The package manager runs three classes of external code: its own trusted
//! tools (`git`, `tclsh`), operator-defined hooks, and — most dangerously —
//! package-provided build scripts. Every one of them is funnelled through this
//! crate so that none inherits the caller's ambient authority (credentials in
//! the environment, the user's home directory, unrestricted network).
//!
//! # Design
//!
//! Execution is modelled as a [`Profile`] (what a command *requests*) evaluated
//! against a [`SandboxPolicy`] (the operator's *floor* — the minimum confinement
//! that must hold, plus hard denials). The effective grant is
//! `requested ∩ allowed` widened by the policy floor; if the host cannot enforce
//! a floor the policy marked mandatory, execution **fails closed**.
//!
//! Two confinement tiers exist:
//!
//! * **Baseline** (every platform, always applied): the environment is cleared
//!   to an explicit allow-list (so tokens, SSH keys and cloud credentials never
//!   leak), the working directory is pinned to a caller-supplied root, output is
//!   captured, and a wall-clock timeout kills runaway children.
//! * **OS-native** (per platform, layered on top): Landlock + seccomp on Linux,
//!   Seatbelt on macOS, `pledge`/`unveil` on OpenBSD, Capsicum on FreeBSD, and
//!   restricted tokens + Job Objects on Windows. These are introduced behind the
//!   [`Confinement`] trait; [`detect_confinement`] returns the strongest tier the
//!   running host can provide and execution reports the [`IsolationLevel`]
//!   actually achieved.
//!
//! The crate contains no `unsafe`: OS-native tiers are driven through wrapper
//! crates that encapsulate the raw syscalls, honouring the workspace
//! `unsafe_code = "forbid"` lint.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub mod capability;

pub use capability::Capability;

/// Environment variable name fragments that must never be passed to a sandboxed
/// child, even if a caller or policy accidentally allow-lists them. Matched
/// case-insensitively as substrings of the variable name. This is the
/// defence-in-depth net behind the explicit allow-list; the lessons of
/// event-stream (AES key read from `npm_package_description`) and the
/// Shai-Hulud worm (credential theft from the process environment) make a
/// belt-and-braces denylist worthwhile.
const SENSITIVE_FRAGMENTS: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "APIKEY",
    "API_KEY",
    "_KEY",
    "PRIVATE",
    "AWS_",
    "GH_",
    "GITHUB_",
    "GITLAB_",
    "NPM_",
    "PYPI_",
    "CARGO_REGISTRY_TOKEN",
    "SSH_",
    "GPG_",
    "NETRC",
    "GCP_",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "AZURE_",
    "DOCKER_",
    "KUBE",
    "VAULT_",
];

/// Whether an environment variable name looks sensitive and should be withheld.
#[must_use]
pub fn is_sensitive_env(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    SENSITIVE_FRAGMENTS.iter().any(|frag| upper.contains(frag))
}

/// The confinement actually achieved for a given execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationLevel {
    /// OS-native confinement (filesystem + network) was applied.
    Os,
    /// Only the portable baseline (env scrub, cwd pin, timeout) was applied.
    Baseline,
    /// Nothing could be enforced (should never happen — baseline is always on).
    None,
}

impl IsolationLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            IsolationLevel::Os => "os",
            IsolationLevel::Baseline => "baseline",
            IsolationLevel::None => "none",
        }
    }
}

/// What a command requests to run with. Built by the caller (the `tcl-pkg`
/// `exec` chokepoint) and then clamped against the [`SandboxPolicy`].
// A request descriptor whose independent on/off knobs (network, capture,
// full-env, scrub) are clearer as named flags than as a packed enum.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Profile {
    /// A short label for audit logs, e.g. `"git-fetch"` or `"hook:scan"`.
    pub label: String,
    /// The program to run. Should be an absolute path unless `PATH` is passed
    /// through, since the environment is cleared.
    pub program: PathBuf,
    /// Arguments (not passed through a shell).
    pub args: Vec<String>,
    /// The working directory the child is confined to. Required.
    pub cwd: PathBuf,
    /// Environment variables to copy from the parent *if present*. Subject to
    /// the sensitive-name denylist and the policy `deny_env` list.
    pub env_passthrough: Vec<String>,
    /// Environment variables to set explicitly (trusted caller-supplied values,
    /// e.g. `HOME` pointed at a throwaway dir). Not filtered.
    pub env_set: Vec<(String, String)>,
    /// Whether the child may use the network. Default-deny for hooks and build
    /// scripts.
    pub allow_network: bool,
    /// Filesystem paths the child may read (for the OS-native tier).
    pub fs_read: Vec<PathBuf>,
    /// Filesystem paths the child may write (for the OS-native tier).
    pub fs_write: Vec<PathBuf>,
    /// Per-command wall-clock timeout.
    pub timeout: Option<Duration>,
    /// Whether to capture stdout/stderr (`true`) or inherit the parent's
    /// terminal (`false`, e.g. `tcl pkg run`).
    pub capture: bool,
    /// Bytes to feed to the child's stdin. When `None`, stdin is inherited if
    /// `capture` is false (interactive) and otherwise closed.
    pub stdin_data: Option<Vec<u8>>,
    /// Pass the *entire* parent environment through, rather than only the
    /// allow-listed names. Used for `tcl pkg run`, which executes the project's
    /// own trusted entry point and should behave like invoking `tclsh` directly
    /// while still gaining audit, timeout and (future) OS confinement.
    pub env_passthrough_all: bool,
    /// Whether the sensitive-name denylist is applied to passed-through
    /// variables. Defaults to `true`; only `tcl pkg run` (trusted user code)
    /// turns it off so an app's own credentials are not stripped.
    pub scrub_env: bool,
}

impl Profile {
    /// A minimally-capable profile: no network, output captured, confined to
    /// `cwd`.
    #[must_use]
    pub fn new(
        label: impl Into<String>,
        program: impl Into<PathBuf>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            label: label.into(),
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.into(),
            env_passthrough: Vec::new(),
            env_set: Vec::new(),
            allow_network: false,
            fs_read: Vec::new(),
            fs_write: Vec::new(),
            timeout: Some(Duration::from_mins(2)),
            capture: true,
            stdin_data: None,
            env_passthrough_all: false,
            scrub_env: true,
        }
    }

    #[must_use]
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    #[must_use]
    pub fn args<I, S>(mut self, it: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(it.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn pass_env(mut self, name: impl Into<String>) -> Self {
        self.env_passthrough.push(name.into());
        self
    }

    #[must_use]
    pub fn set_env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_set.push((name.into(), value.into()));
        self
    }

    #[must_use]
    pub fn network(mut self, allow: bool) -> Self {
        self.allow_network = allow;
        self
    }

    #[must_use]
    pub fn timeout(mut self, t: Option<Duration>) -> Self {
        self.timeout = t;
        self
    }

    #[must_use]
    pub fn capture(mut self, c: bool) -> Self {
        self.capture = c;
        self
    }

    #[must_use]
    pub fn stdin_bytes(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.stdin_data = Some(data.into());
        self
    }

    /// Configure the profile to run trusted user code (`tcl pkg run`): inherit
    /// the full environment without scrubbing, stream stdio, and allow the
    /// network. Policy (`deny_network`, `deny_env`, `max_timeout`) still applies.
    #[must_use]
    pub fn user_code(mut self) -> Self {
        self.env_passthrough_all = true;
        self.scrub_env = false;
        self.capture = false;
        self.allow_network = true;
        self.timeout = None;
        self
    }
}

/// The operator-controlled floor. Produced by the layered policy loader in
/// `tcl-pkg` and applied to every [`Profile`]. A profile can only ever be made
/// *more* restrictive by the policy, never less.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Force the network off regardless of what a profile requests.
    pub deny_network: bool,
    /// The network *must* be deniable for default-deny profiles. If the host
    /// cannot enforce it and [`SandboxPolicy::fail_closed`] is set, execution
    /// aborts rather than running with a weaker isolation than promised.
    pub require_network_deny: bool,
    /// Abort when the achieved isolation cannot meet a mandatory floor.
    pub fail_closed: bool,
    /// Environment names every profile may pass through (e.g. `PATH`, locale).
    pub base_env_passthrough: Vec<String>,
    /// Environment names that must never be passed, even if requested.
    pub deny_env: Vec<String>,
    /// An upper bound clamped onto every profile's timeout.
    pub max_timeout: Option<Duration>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        // Sensible, portable defaults. `PATH` (and the Windows essentials) are
        // passed so bare interpreter names resolve; nothing sensitive is.
        let mut base = vec![
            "PATH".to_string(),
            "LANG".to_string(),
            "LC_ALL".to_string(),
            "LC_CTYPE".to_string(),
            "TZ".to_string(),
        ];
        if cfg!(target_os = "windows") {
            for v in [
                "SystemRoot",
                "WINDIR",
                "PATHEXT",
                "ComSpec",
                "NUMBER_OF_PROCESSORS",
            ] {
                base.push(v.to_string());
            }
        }
        Self {
            deny_network: false,
            require_network_deny: false,
            fail_closed: false,
            base_env_passthrough: base,
            deny_env: Vec::new(),
            max_timeout: None,
        }
    }
}

/// The result of a sandboxed execution.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Process exit code (`None` if killed by signal / timeout).
    pub code: Option<i32>,
    /// Whether the process succeeded (exit code 0).
    pub success: bool,
    /// Captured stdout (empty when `capture` was false).
    pub stdout: Vec<u8>,
    /// Captured stderr (empty when `capture` was false).
    pub stderr: Vec<u8>,
    /// Whether the wall-clock timeout fired and the child was killed.
    pub timed_out: bool,
    /// The confinement tier actually applied.
    pub isolation: IsolationLevel,
    /// Whether network denial was actually enforced by the OS layer.
    pub network_enforced: bool,
}

/// Errors raised before or during sandboxed execution.
#[derive(Debug)]
pub enum SandboxError {
    /// A mandatory policy floor could not be met on this host.
    FailClosed(String),
    /// Spawning or waiting on the child failed.
    Io(std::io::Error),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::FailClosed(m) => write!(f, "sandbox policy not satisfiable: {m}"),
            SandboxError::Io(e) => write!(f, "sandbox exec failed: {e}"),
        }
    }
}

impl std::error::Error for SandboxError {}

impl From<std::io::Error> for SandboxError {
    fn from(e: std::io::Error) -> Self {
        SandboxError::Io(e)
    }
}

/// A platform confinement tier. The baseline is always available; stronger
/// tiers are added per OS behind this trait.
pub trait Confinement {
    /// The isolation level this tier represents.
    fn level(&self) -> IsolationLevel;
    /// Whether this tier can actually prevent network egress.
    fn enforces_network_deny(&self) -> bool;
    /// Apply any OS-native restrictions to `cmd` before it is spawned. The
    /// baseline tier does nothing here (env/cwd are applied generically).
    fn configure(&self, _cmd: &mut Command, _profile: &Profile) -> std::io::Result<()> {
        Ok(())
    }
}

/// The portable baseline: env scrubbing, cwd pinning and timeouts (applied by
/// [`run`]); no OS-native filesystem or network restriction.
#[derive(Debug, Default, Clone, Copy)]
pub struct Baseline;

impl Confinement for Baseline {
    fn level(&self) -> IsolationLevel {
        IsolationLevel::Baseline
    }
    fn enforces_network_deny(&self) -> bool {
        false
    }
}

/// Return the strongest confinement tier available on this host.
///
/// Today this is always [`Baseline`]; OS-native tiers (Landlock/seccomp,
/// Seatbelt, `pledge`/`unveil`, Capsicum, Job Objects) are layered in here as
/// they are implemented, gated by `#[cfg(target_os = ...)]` and runtime probes.
#[must_use]
pub fn detect_confinement() -> Box<dyn Confinement> {
    Box::new(Baseline)
}

/// Compute the effective environment for a profile under a policy, reading
/// parent values through `lookup`. Factored out from [`effective_env`] so the
/// scrubbing logic is testable without mutating the process environment (which
/// is `unsafe` under edition 2024 and banned by `unsafe_code = "forbid"`).
fn effective_env_with<F>(
    profile: &Profile,
    policy: &SandboxPolicy,
    lookup: F,
) -> Vec<(String, String)>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out: Vec<(String, String)> = Vec::new();
    let denied = |name: &str| -> bool {
        is_sensitive_env(name) || policy.deny_env.iter().any(|d| d.eq_ignore_ascii_case(name))
    };
    // Passthroughs: policy base ∪ profile request, copied from the parent only
    // when present and not denied.
    for name in policy
        .base_env_passthrough
        .iter()
        .chain(profile.env_passthrough.iter())
    {
        if denied(name) {
            continue;
        }
        if out.iter().any(|(k, _)| k.eq_ignore_ascii_case(name)) {
            continue;
        }
        if let Some(val) = lookup(name) {
            out.push((name.clone(), val));
        }
    }
    // Explicit sets win (trusted caller-supplied values, e.g. a throwaway HOME).
    for (k, v) in &profile.env_set {
        out.retain(|(ek, _)| !ek.eq_ignore_ascii_case(k));
        out.push((k.clone(), v.clone()));
    }
    out
}

/// Compute the effective environment for a profile under a policy, reading from
/// the real process environment.
fn effective_env(profile: &Profile, policy: &SandboxPolicy) -> Vec<(String, String)> {
    if profile.env_passthrough_all {
        return full_env(profile, policy);
    }
    effective_env_with(profile, policy, |name| {
        std::env::var_os(name).map(|v| v.to_string_lossy().into_owned())
    })
}

/// Inherit the entire parent environment, dropping only policy-denied names (and
/// sensitive names when `scrub_env` is set), then applying explicit sets.
fn full_env(profile: &Profile, policy: &SandboxPolicy) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in std::env::vars() {
        if policy.deny_env.iter().any(|d| d.eq_ignore_ascii_case(&k)) {
            continue;
        }
        if profile.scrub_env && is_sensitive_env(&k) {
            continue;
        }
        out.push((k, v));
    }
    for (k, v) in &profile.env_set {
        out.retain(|(ek, _)| !ek.eq_ignore_ascii_case(k));
        out.push((k.clone(), v.clone()));
    }
    out
}

/// Run a command under the sandbox, applying `policy` as the floor.
///
/// # Errors
///
/// Returns [`SandboxError::FailClosed`] when a mandatory floor (e.g. network
/// denial) cannot be guaranteed on this host, and [`SandboxError::Io`] when the
/// child cannot be spawned or waited on.
pub fn run(profile: &Profile, policy: &SandboxPolicy) -> Result<Outcome, SandboxError> {
    let confinement = detect_confinement();

    let want_network = profile.allow_network && !policy.deny_network;
    // Network denial is only *actually* in force when the child is not to have
    // the network AND the active confinement tier can provably prevent egress.
    // The portable baseline cannot, so on a baseline-only host this is always
    // `false` for a default-deny child — which the guards below act on.
    let network_enforced = !want_network && confinement.enforces_network_deny();

    // Hard operator denial (`deny_network`) is a floor, not a hint: when the
    // operator forces the network off, a real restriction MUST result. If the
    // active confinement cannot prevent egress we refuse rather than run the
    // child with full network while reporting the network as denied. This holds
    // regardless of `fail_closed` — `deny_network` is itself the mandate.
    if policy.deny_network && !network_enforced {
        return Err(SandboxError::FailClosed(format!(
            "'{}' requires the network to be denied (policy deny-network), but this host \
             only provides '{}' isolation, which cannot enforce network denial",
            profile.label,
            confinement.level().as_str()
        )));
    }

    // Fail-closed: if we intend to deny the network but cannot actually enforce
    // it, refuse rather than silently running with weaker isolation.
    if !want_network && policy.require_network_deny && !network_enforced && policy.fail_closed {
        return Err(SandboxError::FailClosed(format!(
            "'{}' requires enforced network denial, but this host only provides '{}' isolation",
            profile.label,
            confinement.level().as_str()
        )));
    }

    let env = effective_env(profile, policy);
    let timeout = match (profile.timeout, policy.max_timeout) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    let mut cmd = Command::new(&profile.program);
    cmd.args(&profile.args);
    cmd.env_clear();
    for (k, v) in &env {
        cmd.env(k, v);
    }
    cmd.current_dir(&profile.cwd);
    if profile.stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else if profile.capture {
        cmd.stdin(Stdio::null());
    } else {
        cmd.stdin(Stdio::inherit());
    }
    if profile.capture {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    }
    confinement.configure(&mut cmd, profile)?;

    let mut child = cmd.spawn()?;

    // Feed stdin on a thread so a large payload cannot deadlock against the
    // child filling its stdout while we are still writing.
    let stdin_handle = match (child.stdin.take(), profile.stdin_data.clone()) {
        (Some(mut sink), Some(data)) => Some(std::thread::spawn(move || {
            use std::io::Write;
            let _ = sink.write_all(&data);
            // Dropping `sink` closes the pipe, signalling EOF.
        })),
        _ => None,
    };

    // Drain pipes on threads so a chatty child cannot deadlock against a full
    // pipe buffer while we wait on the timeout.
    let stdout_handle = child.stdout.take().map(drain_thread);
    let stderr_handle = child.stderr.take().map(drain_thread);

    let (status, timed_out) = wait_with_timeout(&mut child, timeout)?;

    if let Some(h) = stdin_handle {
        let _ = h.join();
    }
    let stdout = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    Ok(Outcome {
        code: status.and_then(|s| s.code()),
        success: status.is_some_and(|s| s.success()) && !timed_out,
        stdout,
        stderr,
        timed_out,
        isolation: confinement.level(),
        network_enforced,
    })
}

fn drain_thread<R>(mut r: R) -> std::thread::JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        buf
    })
}

/// Wait on `child`, killing it if `timeout` elapses. Returns the exit status
/// (`None` when killed) and whether the timeout fired.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Option<Duration>,
) -> std::io::Result<(Option<std::process::ExitStatus>, bool)> {
    use wait_timeout::ChildExt;
    let Some(t) = timeout else {
        let status = child.wait()?;
        return Ok((Some(status), false));
    };
    if let Some(status) = child.wait_timeout(t)? {
        Ok((Some(status), false))
    } else {
        let _ = child.kill();
        let _ = child.wait();
        Ok((None, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_env_detection() {
        assert!(is_sensitive_env("GITHUB_TOKEN"));
        assert!(is_sensitive_env("aws_secret_access_key"));
        assert!(is_sensitive_env("MY_API_KEY"));
        assert!(is_sensitive_env("SSH_AUTH_SOCK"));
        assert!(is_sensitive_env("npm_token"));
        assert!(!is_sensitive_env("PATH"));
        assert!(!is_sensitive_env("LANG"));
        assert!(!is_sensitive_env("HOME"));
    }

    /// Resolve a system tool to an absolute path via `PATH`, the way a shell
    /// would.
    ///
    /// A [`Profile`]'s program must be absolute, because the sandbox clears the
    /// child's environment — but *which* absolute path a tool lives at is not
    /// portable. `true` is `/bin/true` on most Linux distros and
    /// `/usr/bin/true` on macOS (which has no `/bin/true` at all), so a
    /// hardcoded path passes on one and fails with a bare `NotFound` on the
    /// other. `true`, `env` and `sleep` are on `PATH` on every distro we build
    /// on and on macOS, so resolve them rather than guessing a location.
    fn tool(name: &str) -> PathBuf {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join(name))
            .find(|p| is_executable_file(p))
            .unwrap_or_else(|| {
                panic!("`{name}` is not on PATH; the sandbox tests need it (coreutils)")
            })
    }

    #[cfg(unix)]
    fn is_executable_file(p: &std::path::Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    fn is_executable_file(p: &std::path::Path) -> bool {
        p.is_file()
    }

    /// A fixed fake environment, so the scrubbing logic is exercised without
    /// mutating the real process environment.
    fn fake_env(name: &str) -> Option<String> {
        match name {
            "TCL_SB_TEST_SAFE" => Some("ok".into()),
            "TCL_SB_TEST_TOKEN" => Some("leak".into()),
            "TCL_SB_TEST_BLOCKME" => Some("v".into()),
            _ => None,
        }
    }

    #[test]
    fn effective_env_scrubs_secrets_and_dedupes() {
        let policy = SandboxPolicy {
            base_env_passthrough: vec!["TCL_SB_TEST_SAFE".into()],
            ..SandboxPolicy::default()
        };
        let profile = Profile::new("t", tool("true"), "/")
            .pass_env("TCL_SB_TEST_SAFE")
            .pass_env("TCL_SB_TEST_TOKEN")
            .set_env("HOME", "/tmp/throwaway");
        let env = effective_env_with(&profile, &policy, fake_env);
        assert!(
            env.iter()
                .any(|(k, v)| k == "TCL_SB_TEST_SAFE" && v == "ok")
        );
        // Sensitive name (contains TOKEN) is dropped even though requested.
        assert!(!env.iter().any(|(k, _)| k == "TCL_SB_TEST_TOKEN"));
        assert!(
            env.iter()
                .any(|(k, v)| k == "HOME" && v == "/tmp/throwaway")
        );
        // Deduped despite appearing in both base and request.
        assert_eq!(
            env.iter().filter(|(k, _)| k == "TCL_SB_TEST_SAFE").count(),
            1
        );
    }

    #[test]
    fn deny_env_overrides_passthrough() {
        let policy = SandboxPolicy {
            deny_env: vec!["TCL_SB_TEST_BLOCKME".into()],
            base_env_passthrough: vec![],
            ..SandboxPolicy::default()
        };
        let profile = Profile::new("t", tool("true"), "/").pass_env("TCL_SB_TEST_BLOCKME");
        let env = effective_env_with(&profile, &policy, fake_env);
        assert!(!env.iter().any(|(k, _)| k == "TCL_SB_TEST_BLOCKME"));
    }

    #[test]
    fn fail_closed_when_network_deny_unenforceable() {
        let policy = SandboxPolicy {
            require_network_deny: true,
            fail_closed: true,
            ..SandboxPolicy::default()
        };
        // Baseline cannot enforce network denial, so a default-deny profile must
        // be refused.
        let profile = Profile::new("build", tool("true"), "/").network(false);
        let err = run(&profile, &policy).unwrap_err();
        assert!(matches!(err, SandboxError::FailClosed(_)));
    }

    #[test]
    fn deny_network_fails_closed_when_unenforceable() {
        // `deny-network = true` is a hard floor. The portable baseline cannot
        // enforce egress denial, so the run must refuse rather than silently
        // proceed with full network — even without `fail_closed` set.
        let policy = SandboxPolicy {
            deny_network: true,
            ..SandboxPolicy::default()
        };
        // Even a profile that *requests* the network must be refused: the
        // operator floor forces it off, and off cannot be guaranteed here.
        let profile = Profile::new("hook", tool("true"), "/").network(true);
        let err = run(&profile, &policy).unwrap_err();
        match err {
            SandboxError::FailClosed(m) => assert!(
                m.contains("deny-network"),
                "expected a deny-network fail-closed message, got: {m}"
            ),
            other @ SandboxError::Io(_) => panic!("expected FailClosed, got {other:?}"),
        }
    }

    #[test]
    fn deny_network_refuses_default_deny_profile_too() {
        // A default-deny profile under `deny-network` is likewise refused on a
        // host that cannot enforce denial: no silent proceed, no error swallow.
        let policy = SandboxPolicy {
            deny_network: true,
            ..SandboxPolicy::default()
        };
        let profile = Profile::new("build", tool("true"), "/").network(false);
        assert!(matches!(
            run(&profile, &policy).unwrap_err(),
            SandboxError::FailClosed(_)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn network_enforced_reports_false_on_baseline() {
        // The Outcome flag must reflect reality: the baseline never enforces
        // network denial, so a run that is *allowed* the network reports the
        // denial as not enforced (rather than the old misleading `true`).
        let profile = Profile::new("t", tool("true"), "/").network(true);
        let outcome = run(&profile, &SandboxPolicy::default()).unwrap();
        assert!(!outcome.network_enforced);
    }

    #[cfg(unix)]
    #[test]
    fn runs_and_captures_with_scrubbed_env() {
        // `cargo test` runs the test binary with CARGO_* vars in its environment.
        // They are not in the sandbox allow-list, so a scrubbed child must not
        // see them. Only assert when the parent actually has one.
        let leaked = std::env::var_os("CARGO_MANIFEST_DIR").is_some();
        let profile = Profile::new("t", tool("env"), "/")
            .set_env("HOME", "/tmp/throwaway-home")
            .timeout(Some(Duration::from_secs(10)));
        let outcome = run(&profile, &SandboxPolicy::default()).unwrap();
        assert!(outcome.success);
        let text = String::from_utf8_lossy(&outcome.stdout);
        assert!(text.contains("HOME=/tmp/throwaway-home"));
        if leaked {
            assert!(
                !text.contains("CARGO_MANIFEST_DIR="),
                "sandbox leaked parent env var into child: {text}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_runaway_child() {
        let profile = Profile::new("t", tool("sleep"), "/")
            .arg("30")
            .timeout(Some(Duration::from_millis(300)));
        let outcome = run(&profile, &SandboxPolicy::default()).unwrap();
        assert!(outcome.timed_out);
        assert!(!outcome.success);
    }
}
