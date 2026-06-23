//! Layered, operator-lockable package-manager policy.
//!
//! Policy is merged from three TOML layers, lowest precedence first:
//!
//! 1. **System** (`/etc/tcl-lsp/pkg-policy.toml`, `%PROGRAMDATA%\tcl-lsp\…` on
//!    Windows) — the operator/corporate layer. Honoured only when the file is
//!    owned by root/Administrators and not world-writable, so a developer cannot
//!    weaken it by editing a file they own.
//! 2. **User** (`~/.config/tcl-lsp/pkg.toml`).
//! 3. **Project** (`tclpkg.toml` next to the manifest).
//!
//! The system layer may **lock** keys so the user and project layers cannot
//! override them — the primitive that lets corporate IT deploy a floor
//! developers cannot loosen:
//!
//! * `lock = ["sandbox.fail-closed", "registry"]` — a dotted leaf is frozen at
//!   its system value; a whole section name freezes the entire subtree (no
//!   sibling additions either).
//! * `lock-all = true` — every *leaf* the system layer sets is frozen, while
//!   still allowing lower layers to add keys the system did not set.
//!
//! Attempts to override a locked key are ignored and surfaced as warnings (see
//! [`LoadedPolicy::warnings`]).
//!
//! The manifest (`tclpkg.tcl`) stays pure data; nothing here executes code. The
//! policy only *describes* what may run and how confined it must be — the
//! [`crate::exec`] chokepoint and [`crate::hooks`] dispatcher enforce it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tcl_sandbox::SandboxPolicy;
use toml::{Table, Value};

/// The fully-merged, typed policy.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PolicyConfig {
    pub sandbox: SandboxSection,
    pub registry: RegistrySection,
    pub cooldown: CooldownSection,
    pub verification: VerificationSection,
    pub build: BuildSection,
    #[serde(default)]
    pub hooks: Vec<HookDef>,
    /// Dotted keys / section names the system layer froze.
    #[serde(default)]
    pub lock: Vec<String>,
    /// Freeze every leaf the system layer set.
    #[serde(default)]
    pub lock_all: bool,
}

/// Sandbox floor applied to every sandboxed execution.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SandboxSection {
    /// Abort when a mandatory floor cannot be met on this host.
    pub fail_closed: bool,
    /// Network denial must be enforced for default-deny executions.
    pub require_network_deny: bool,
    /// Force the network off for everything, even callers that request it.
    pub deny_network: bool,
    /// Clamp every command's timeout to at most this many seconds.
    pub max_timeout_secs: Option<u64>,
    /// Extra environment names every sandboxed child may pass through.
    pub env_allow: Vec<String>,
    /// Environment names that must never be passed through.
    pub env_deny: Vec<String>,
}

/// Registry source controls.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RegistrySection {
    /// If non-empty, only source URLs with one of these prefixes are allowed.
    pub allow: Vec<String>,
    /// Source URLs with any of these prefixes are rejected.
    pub deny: Vec<String>,
    /// Reject non-`https://` source URLs.
    pub require_https: bool,
}

/// Minimum-release-age ("cooldown") control — defends against the replication
/// window of worms like Shai-Hulud and freshly-published typosquats.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CooldownSection {
    pub min_release_age_days: u64,
}

/// Integrity / provenance requirements.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct VerificationSection {
    /// Require every locked package to carry an integrity hash and verify it.
    pub require_integrity: bool,
    /// Require provenance/attestation verification (delegated to a hook).
    pub require_provenance: bool,
}

/// Build-phase script controls. Off by default — packages are data unless an
/// operator opts in, mirroring npm v12 / pnpm / Bun / Deno.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BuildSection {
    /// Allow declared build scripts to run at all.
    pub allow_build_scripts: bool,
    /// Packages whose build script is trusted to run (when allowed).
    pub trusted: Vec<String>,
}

/// An operator hook bound to a lifecycle stage.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct HookDef {
    pub name: String,
    /// Lifecycle stage, e.g. `post-fetch` (see [`crate::hooks::Stage`]).
    pub stage: String,
    /// Argument vector; the first element is the program (run without a shell).
    pub command: Vec<String>,
    /// Whether the hook may use the network (default deny).
    pub network: bool,
    /// Per-hook timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Filesystem read grants (for the OS-native confinement tier).
    pub fs_read: Vec<String>,
    /// Filesystem write grants.
    pub fs_write: Vec<String>,
    /// Environment names the hook may pass through.
    pub env: Vec<String>,
}

impl PolicyConfig {
    /// Derive the sandbox floor from the policy.
    #[must_use]
    pub fn sandbox_policy(&self) -> SandboxPolicy {
        let mut base = SandboxPolicy {
            deny_network: self.sandbox.deny_network,
            require_network_deny: self.sandbox.require_network_deny,
            fail_closed: self.sandbox.fail_closed,
            deny_env: self.sandbox.env_deny.clone(),
            max_timeout: self.sandbox.max_timeout_secs.map(Duration::from_secs),
            ..SandboxPolicy::default()
        };
        base.base_env_passthrough
            .extend(self.sandbox.env_allow.iter().cloned());
        base
    }

    /// Whether a source URL is permitted by the registry allow/deny policy.
    #[must_use]
    pub fn source_allowed(&self, url: &str) -> bool {
        // Local path / git sources are not URLs; only gate plain-http ones.
        if self.registry.require_https && url.starts_with("http://") {
            return false;
        }
        if self.registry.deny.iter().any(|p| url.starts_with(p)) {
            return false;
        }
        if !self.registry.allow.is_empty() {
            return self.registry.allow.iter().any(|p| url.starts_with(p));
        }
        true
    }
}

/// Which file a policy layer was loaded from, and whether it was honoured.
#[derive(Debug, Clone)]
pub struct PolicySource {
    pub layer: &'static str,
    pub path: PathBuf,
    pub loaded: bool,
    pub note: Option<String>,
}

/// The result of loading and merging policy.
#[derive(Debug, Clone)]
pub struct LoadedPolicy {
    pub config: PolicyConfig,
    pub sources: Vec<PolicySource>,
    pub locked: Vec<String>,
    pub warnings: Vec<String>,
}

/// Load and merge the system, user and project policy layers.
///
/// `project_dir`, when given, is the directory containing the manifest; its
/// `tclpkg.toml` is the project layer. Missing files are skipped; malformed
/// files are skipped with a warning rather than aborting (a broken user file
/// must never break a locked-down system floor).
#[must_use]
pub fn load(project_dir: Option<&Path>) -> LoadedPolicy {
    let system_path = crate::system_config_dir().join("pkg-policy.toml");
    let user_path = crate::config_dir().join("pkg.toml");
    let project_path = project_dir.map(|d| d.join("tclpkg.toml"));

    let mut sources = Vec::new();
    let mut warnings = Vec::new();

    let system = load_system_layer(&system_path, &mut sources, &mut warnings);
    let explicit_locks = string_array(&system, "lock");
    let lock_all = system
        .get("lock-all")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut merged = system.clone();
    for (layer, path) in [("user", Some(user_path)), ("project", project_path)] {
        let Some(path) = path else { continue };
        merge_lower_layer(
            layer,
            &path,
            &system,
            &explicit_locks,
            lock_all,
            &mut merged,
            &mut sources,
            &mut warnings,
        );
    }

    let locked = collect_locked_leaves(&system, "", &explicit_locks, lock_all);
    let config = merged.try_into::<PolicyConfig>().unwrap_or_else(|e| {
        warnings.push(format!("policy schema error: {e}"));
        PolicyConfig::default()
    });

    LoadedPolicy {
        config,
        sources,
        locked,
        warnings,
    }
}

/// Read and trust-check the system layer, recording its [`PolicySource`].
fn load_system_layer(
    path: &Path,
    sources: &mut Vec<PolicySource>,
    warnings: &mut Vec<String>,
) -> Table {
    let mut record = |loaded: bool, note: Option<String>| {
        sources.push(PolicySource {
            layer: "system",
            path: path.to_path_buf(),
            loaded,
            note,
        });
    };
    match read_layer(path) {
        LayerRead::Missing => {
            record(false, None);
            Table::new()
        }
        LayerRead::Invalid(e) => {
            warnings.push(format!("{}: {e}", path.display()));
            record(false, Some(e));
            Table::new()
        }
        LayerRead::Parsed(t) if system_file_trusted(path) => {
            record(true, None);
            t
        }
        LayerRead::Parsed(_) => {
            let note = "ignored: system policy must be owned by root and not world-writable";
            warnings.push(format!("{}: {note}", path.display()));
            record(false, Some(note.to_string()));
            Table::new()
        }
    }
}

/// Read a user/project layer and merge it into `merged`, honouring locks.
#[allow(clippy::too_many_arguments)]
fn merge_lower_layer(
    layer: &'static str,
    path: &Path,
    system: &Table,
    explicit_locks: &[String],
    lock_all: bool,
    merged: &mut Table,
    sources: &mut Vec<PolicySource>,
    warnings: &mut Vec<String>,
) {
    match read_layer(path) {
        LayerRead::Missing => sources.push(PolicySource {
            layer,
            path: path.to_path_buf(),
            loaded: false,
            note: None,
        }),
        LayerRead::Invalid(e) => {
            warnings.push(format!("{}: {e}", path.display()));
            sources.push(PolicySource {
                layer,
                path: path.to_path_buf(),
                loaded: false,
                note: Some(e),
            });
        }
        LayerRead::Parsed(t) => {
            merge_locked(
                merged,
                &t,
                system,
                "",
                explicit_locks,
                lock_all,
                layer,
                warnings,
            );
            sources.push(PolicySource {
                layer,
                path: path.to_path_buf(),
                loaded: true,
                note: None,
            });
        }
    }
}

/// Extract a TOML array of strings at `key`.
fn string_array(table: &Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

enum LayerRead {
    Missing,
    Parsed(Table),
    Invalid(String),
}

fn read_layer(path: &Path) -> LayerRead {
    if !path.is_file() {
        return LayerRead::Missing;
    }
    match std::fs::read_to_string(path) {
        Ok(text) => match text.parse::<Table>() {
            Ok(t) => LayerRead::Parsed(t),
            Err(e) => LayerRead::Invalid(e.to_string()),
        },
        Err(e) => LayerRead::Invalid(e.to_string()),
    }
}

/// On Unix, the system policy must be owned by uid 0 and not group/world
/// writable. On Windows we trust `%PROGRAMDATA%` ACLs and accept the file.
#[cfg(unix)]
fn system_file_trusted(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let mode = meta.mode();
    let world_or_group_writable = mode & 0o022 != 0;
    meta.uid() == 0 && !world_or_group_writable
}

#[cfg(not(unix))]
fn system_file_trusted(_path: &Path) -> bool {
    true
}

/// Look up a dotted path in a table.
fn table_get<'a>(table: &'a Table, path: &str) -> Option<&'a Value> {
    let mut current: Option<&Value> = None;
    for (i, part) in path.split('.').enumerate() {
        let next = if i == 0 {
            table.get(part)
        } else {
            current.and_then(Value::as_table).and_then(|t| t.get(part))
        };
        current = next;
        current?;
    }
    current
}

/// Whether `path` is frozen by the system layer.
fn is_locked(path: &str, system: &Table, explicit: &[String], lock_all: bool) -> bool {
    // An explicit lock entry matches the path itself or any ancestor.
    for entry in explicit {
        if entry == path || path.starts_with(&format!("{entry}.")) {
            return true;
        }
    }
    if lock_all {
        // Under lock-all only leaves the system actually set are frozen; tables
        // stay open so lower layers may add sibling keys.
        return matches!(table_get(system, path), Some(v) if !v.is_table());
    }
    false
}

/// Deep-merge `incoming` into `acc`, refusing to override paths locked by the
/// system layer and recording each refusal as a warning.
#[allow(clippy::too_many_arguments)]
fn merge_locked(
    acc: &mut Table,
    incoming: &Table,
    system: &Table,
    prefix: &str,
    explicit: &[String],
    lock_all: bool,
    layer: &str,
    warnings: &mut Vec<String>,
) {
    for (k, v) in incoming {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        if is_locked(&path, system, explicit, lock_all) {
            warnings.push(format!("{layer}: ignored override of locked key '{path}'"));
            continue;
        }
        match (acc.get_mut(k), v) {
            (Some(Value::Table(at)), Value::Table(it)) => {
                merge_locked(at, it, system, &path, explicit, lock_all, layer, warnings);
            }
            _ => {
                acc.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Enumerate the concrete locked leaf paths, for display by `tcl pkg policy`.
fn collect_locked_leaves(
    system: &Table,
    prefix: &str,
    explicit: &[String],
    lock_all: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in system {
        if k == "lock" || k == "lock-all" {
            continue;
        }
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        if let Value::Table(t) = v {
            // An explicitly locked section freezes the whole subtree.
            if explicit.iter().any(|e| e == &path) {
                out.push(format!("{path} (section)"));
            } else {
                out.extend(collect_locked_leaves(t, &path, explicit, lock_all));
            }
        } else if is_locked(&path, system, explicit, lock_all) {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(system: &str, user: &str) -> (PolicyConfig, Vec<String>) {
        let system: Table = system.parse().unwrap();
        let user: Table = user.parse().unwrap();
        let explicit: Vec<String> = system
            .get("lock")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let lock_all = system
            .get("lock-all")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut merged = system.clone();
        let mut warnings = Vec::new();
        merge_locked(
            &mut merged,
            &user,
            &system,
            "",
            &explicit,
            lock_all,
            "user",
            &mut warnings,
        );
        (merged.try_into().unwrap(), warnings)
    }

    #[test]
    fn unlocked_user_overrides_system() {
        let (cfg, warns) = merge(
            "[sandbox]\nfail-closed = false\n",
            "[sandbox]\nfail-closed = true\n",
        );
        assert!(cfg.sandbox.fail_closed);
        assert!(warns.is_empty());
    }

    #[test]
    fn locked_leaf_cannot_be_overridden() {
        let (cfg, warns) = merge(
            "lock = [\"sandbox.fail-closed\"]\n[sandbox]\nfail-closed = true\n",
            "[sandbox]\nfail-closed = false\n",
        );
        assert!(cfg.sandbox.fail_closed, "locked system value must win");
        assert!(warns.iter().any(|w| w.contains("sandbox.fail-closed")));
    }

    #[test]
    fn locked_section_freezes_subtree() {
        let (cfg, warns) = merge(
            "lock = [\"registry\"]\n[registry]\nrequire-https = true\nallow = [\"https://a\"]\n",
            "[registry]\nrequire-https = false\ndeny = [\"https://b\"]\n",
        );
        assert!(cfg.registry.require_https);
        assert!(
            cfg.registry.deny.is_empty(),
            "no sibling additions into a locked section"
        );
        assert!(warns.iter().any(|w| w.contains("registry")));
    }

    #[test]
    fn lock_all_freezes_set_leaves_but_allows_new_keys() {
        let (cfg, _warns) = merge(
            "lock-all = true\n[sandbox]\ndeny-network = true\n",
            "[sandbox]\ndeny-network = false\nfail-closed = true\n",
        );
        // The system-set leaf is frozen ...
        assert!(cfg.sandbox.deny_network);
        // ... but a key the system never set may still be added by the user.
        assert!(cfg.sandbox.fail_closed);
    }

    #[test]
    fn source_allowed_respects_https_and_lists() {
        let cfg = PolicyConfig {
            registry: RegistrySection {
                require_https: true,
                allow: vec!["https://good/".into()],
                deny: vec!["https://good/evil".into()],
            },
            ..PolicyConfig::default()
        };
        assert!(cfg.source_allowed("https://good/pkg"));
        assert!(
            !cfg.source_allowed("http://good/pkg"),
            "http rejected under require-https"
        );
        assert!(!cfg.source_allowed("https://good/evil"), "deny prefix wins");
        assert!(
            !cfg.source_allowed("https://other/pkg"),
            "not in allow list"
        );
    }
}
