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

//! `tcl pkg` verb group — Tcl package management.
//!
//! Thin wrappers over the `tcl_pkg` modules: parse args, drive the library,
//! format output. Handler-level errors
//! print `error: <msg>` to stderr and return exit code 1.

// Handlers return `anyhow::Result<u8>` for a uniform dispatch signature even
// when a given verb cannot fail; the wrap is the interface contract.
#![allow(clippy::unnecessary_wraps)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use tcl_pkg::cas::ContentAddressableStore;
use tcl_pkg::installer;
use tcl_pkg::lockfile::{LockFile, LockedPackage, SourceSpec, read_lockfile, write_lockfile};
use tcl_pkg::manifest::load_manifest;
use tcl_pkg::registry::RegistryClient;
use tcl_pkg::resolver::{ExcludeSpec, PackageRef, ReplaceSpec, ResolveInput, resolve};
use tcl_pkg::ui;
use tcl_pkg::version::Version;

/// Outcome of resolving + fetching one `(name, version)`, memoised so a package
/// is fetched from the network at most once and shared between transitive
/// resolution and materialisation.
type FetchOutcome = Result<Option<installer::MaterialiseResult>, FetchError>;

/// Why a fetch did not yield an installable package.
#[derive(Clone)]
enum FetchError {
    /// The source URL was refused by registry allow/deny/require-https policy.
    /// A hard error: the install aborts.
    PolicyRejected(String),
    /// The fetch itself failed (network, bad archive, …). A soft error: keep a
    /// placeholder lock entry recording the attempted source and warn.
    Fetch { source: SourceSpec, message: String },
}

/// Materialise a stored package into a throwaway directory and return its own
/// (non-dev) `require` directives as `(ref, source_url)`, so the resolver can
/// walk transitive dependencies and later fetch them. Best-effort: a
/// missing/broken manifest yields no requires (the package is treated as a
/// leaf).
fn read_package_requires(
    cas: &ContentAddressableStore,
    integrity: &str,
) -> Vec<(PackageRef, Option<String>)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "tclpkg-resolve-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let reqs = if cas.materialise(integrity, &tmp, false).is_ok() {
        let manifest_path = tmp.join("tclpkg.tcl");
        if manifest_path.is_file() {
            load_manifest(&manifest_path).map_or_else(
                |_| Vec::new(),
                |ast| {
                    ast.requires
                        .iter()
                        .map(|r| {
                            (
                                PackageRef::new(r.name.clone(), r.minimum.clone()),
                                r.source_url.clone(),
                            )
                        })
                        .collect()
                },
            )
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let _ = std::fs::remove_dir_all(&tmp);
    reqs
}

/// Parse a lockfile `requires` entry (`name@version`) back into a [`PackageRef`].
fn parse_locked_ref(entry: &str) -> Option<PackageRef> {
    let (name, ver) = entry.rsplit_once('@')?;
    Version::parse(ver)
        .ok()
        .map(|v| PackageRef::new(name.to_string(), v))
}

/// The package name of a lockfile `requires` entry (`name@version` → `name`).
fn locked_req_name(entry: &str) -> &str {
    entry.rsplit_once('@').map_or(entry, |(name, _)| name)
}

use crate::cli::{PkgCommand, PkgCommon, PolicyAction};

/// Dispatch a `tcl pkg` sub-action.
pub fn run(action: &PkgCommand) -> anyhow::Result<u8> {
    match action {
        PkgCommand::Init {
            name,
            init_version,
            init_license,
            tcl,
            force,
            json,
        } => run_init(
            name.as_deref(),
            init_version.as_deref(),
            init_license.as_deref(),
            tcl.as_deref(),
            *force,
            *json,
        ),
        PkgCommand::Install {
            common,
            no_dev,
            frozen,
        } => run_install(common, *no_dev, *frozen),
        PkgCommand::List { common } => run_list(common),
        PkgCommand::Tree { common } => run_tree(common),
        PkgCommand::Verify { common } => run_verify(common),
        PkgCommand::Info { package, common } => run_info(package, common),
        PkgCommand::Add {
            package,
            min_version,
            source,
            dev,
            common,
        } => run_add(
            package,
            min_version.as_deref(),
            source.as_deref(),
            *dev,
            common,
        ),
        PkgCommand::Remove { package, common } => run_remove(package, common),
        PkgCommand::Update { packages, common } => run_update(packages, common),
        PkgCommand::Sync { common } => run_sync(common),
        PkgCommand::Outdated { common } => run_outdated(common),
        PkgCommand::Why { package, common } => run_why(package, common),
        PkgCommand::Vendor { dir, common } => run_vendor(dir, common),
        PkgCommand::Run { extra, common } => run_run(extra, common),
        PkgCommand::Freeze { common } => run_freeze(common),
        PkgCommand::Search {
            query,
            json,
            offline,
        } => run_search(query, *json, *offline),
        PkgCommand::Policy { action } => run_policy(action),
        PkgCommand::Hooks { json } => run_hooks(*json),
        PkgCommand::Audit { lines, json } => run_audit(*lines, *json),
        PkgCommand::Trust { package, remove } => run_trust(package, *remove),
        PkgCommand::Build { common } => run_build(common),
    }
}

fn find_project_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?.canonicalize().ok()?;
    for _ in 0..20 {
        if current.join("tclpkg.tcl").is_file() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => break,
        }
    }
    None
}

fn manifest_path(common: &PkgCommon) -> PathBuf {
    if let Some(override_path) = &common.manifest {
        return override_path.clone();
    }
    if let Some(root) = find_project_root() {
        return root.join("tclpkg.tcl");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("tclpkg.tcl")
}

fn lockfile_path(common: &PkgCommon) -> PathBuf {
    let mpath = manifest_path(common);
    mpath
        .parent()
        .map_or_else(|| PathBuf::from("tclpkg.lock"), |p| p.join("tclpkg.lock"))
}

/// Where `install` materialises packages: the active venv's `lib/` if one is
/// set, else `<project>/lib` next to the manifest.
fn install_lib_dir(mpath: &Path) -> PathBuf {
    if let Some(venv) = std::env::var_os("TCL_VENV") {
        return PathBuf::from(venv).join("lib");
    }
    mpath
        .parent()
        .map_or_else(|| PathBuf::from("lib"), |p| p.join("lib"))
}

fn run_init(
    name: Option<&str>,
    init_version: Option<&str>,
    init_license: Option<&str>,
    tcl: Option<&str>,
    force: bool,
    json: bool,
) -> anyhow::Result<u8> {
    let path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("tclpkg.tcl");
    if path.exists() && !force {
        eprintln!(
            "error: {} already exists (use --force to overwrite)",
            path.display()
        );
        return Ok(1);
    }
    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let name = name.map_or(dir_name, ToString::to_string);
    let version = init_version.unwrap_or("0.1.0");
    let license = init_license.unwrap_or("MIT");
    let tcl = tcl.unwrap_or(">=8.6");

    let body = format!(
        "package     {name}\nversion     {version}\nlicense     {license}\ntcl         {tcl}\n"
    );
    if let Err(e) = std::fs::write(&path, body) {
        eprintln!("error: {e}");
        return Ok(1);
    }

    let colour = ui::use_colour_for_json(json);
    if json {
        println!(
            "{}",
            ui::json_output(
                &json!({"path": path.to_string_lossy(), "name": name, "version": version})
            )
        );
    } else {
        println!("{}", ui::ok(&format!("wrote {}", path.display()), colour));
    }
    Ok(0)
}

#[allow(clippy::too_many_lines)] // the full install flow in one function
fn run_install(common: &PkgCommon, no_dev: bool, frozen: bool) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour_for_json(common.json);

    let manifest = match load_manifest(&mpath) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

    let project_dir = mpath
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let loaded = tcl_pkg::policy::load(Some(&project_dir));
    for w in &loaded.warnings {
        eprintln!("{}", ui::warn(w, colour));
    }
    let install_ctx = tcl_pkg::hooks::HookContext::new(&project_dir)
        .var("MANIFEST", mpath.to_string_lossy())
        .var("PROJECT", project_dir.to_string_lossy());
    if let Err(e) = tcl_pkg::hooks::run_stage(
        tcl_pkg::hooks::Stage::PreInstall,
        &loaded.config,
        &install_ctx,
    ) {
        eprintln!("error: {e}");
        return Ok(1);
    }

    let direct: Vec<PackageRef> = manifest
        .requires
        .iter()
        .map(|r| PackageRef::new(r.name.clone(), r.minimum.clone()))
        .collect();
    let dev_direct: Vec<PackageRef> = manifest
        .dev_requires
        .iter()
        .map(|r| PackageRef::new(r.name.clone(), r.minimum.clone()))
        .collect();
    let replaces: Vec<ReplaceSpec> = manifest
        .replaces
        .iter()
        .map(|r| ReplaceSpec {
            name: r.name.clone(),
            version: r.version.clone(),
        })
        .collect();
    let excludes: Vec<ExcludeSpec> = manifest
        .excludes
        .iter()
        .map(|e| ExcludeSpec {
            name: e.name.clone(),
            version: e.version.clone(),
        })
        .collect();

    // Source lookup tables, needed both by the resolver's provider (to fetch a
    // dependency's own manifest and read its transitive `require`s) and by
    // materialisation. `manifest_sources` starts from the root manifest's
    // `-source` overrides and is extended, during resolution, with any
    // `-source` an intermediate package declares for its own dependencies — so
    // a self-contained graph resolves without a registry.
    let mut seed_sources: HashMap<String, String> = HashMap::new();
    for r in manifest.requires.iter().chain(manifest.dev_requires.iter()) {
        if let Some(url) = &r.source_url {
            seed_sources.insert(r.name.clone(), url.clone());
        }
    }
    let manifest_sources: RefCell<HashMap<String, String>> = RefCell::new(seed_sources);
    let mut replace_sources: HashMap<String, String> = HashMap::new();
    for r in &manifest.replaces {
        replace_sources.insert(r.name.clone(), r.source_url.clone());
    }
    // --frozen never touches the network or the tree.
    let offline = frozen || common.offline;
    let cache = tcl_pkg::cache_dir();
    let cas = ContentAddressableStore::new(&cache);
    let registry = RefCell::new(RegistryClient::new(&cache, common.offline));
    let lib_dir = install_lib_dir(&mpath);
    let lockpath = mpath
        .parent()
        .map_or_else(|| PathBuf::from("tclpkg.lock"), |p| p.join("tclpkg.lock"));

    // Fetch cache shared between transitive resolution and materialisation, so a
    // package is fetched from the network at most once per (name, version). The
    // registry allow/deny + require-https floor is applied here, before any
    // fetch — including fetches performed while walking transitive deps — so a
    // rejected source can never be contacted.
    let fetched: RefCell<HashMap<(String, String), FetchOutcome>> = RefCell::new(HashMap::new());
    let fetch_one = |name: &str, version: &str| -> FetchOutcome {
        let key = (name.to_string(), version.to_string());
        if let Some(hit) = fetched.borrow().get(&key) {
            return hit.clone();
        }
        let source = {
            let mut reg = registry.borrow_mut();
            installer::resolve_source(
                name,
                version,
                &replace_sources,
                &manifest_sources.borrow(),
                Some(&mut reg),
            )
        };
        let outcome = match source {
            None => Ok(None),
            Some(source) => {
                if !source.url.is_empty() && !loaded.config.source_allowed(&source.url) {
                    Err(FetchError::PolicyRejected(source.url.clone()))
                } else {
                    match installer::fetch_and_store(&source, name, version, &cas, 60) {
                        Ok(result) => Ok(Some(result)),
                        Err(e) => Err(FetchError::Fetch {
                            source,
                            message: e.to_string(),
                        }),
                    }
                }
            }
        };
        fetched.borrow_mut().insert(key, outcome.clone());
        outcome
    };

    // Online provider: fetch each package and read the transitive `require`s
    // from its own manifest. A policy-rejected source aborts resolution; a fetch
    // failure degrades to a leaf (the materialisation loop then warns).
    let online_provider =
        |name: &str, version: &Version| -> Result<Vec<PackageRef>, tcl_pkg::TclPkgError> {
            match fetch_one(name, &version.to_string()) {
                Ok(Some(result)) => {
                    let mut refs = Vec::new();
                    for (child, source_url) in read_package_requires(&cas, &result.integrity) {
                        // Propagate an intermediate package's `-source` override
                        // so its dependency can be fetched (no transitive
                        // `replace`, matching Go — this is only source discovery).
                        if let Some(url) = source_url {
                            manifest_sources
                                .borrow_mut()
                                .entry(child.name.clone())
                                .or_insert(url);
                        }
                        refs.push(child);
                    }
                    Ok(refs)
                }
                Ok(None) | Err(FetchError::Fetch { .. }) => Ok(Vec::new()),
                Err(FetchError::PolicyRejected(url)) => Err(tcl_pkg::TclPkgError::new(format!(
                    "{name} {version}: source '{url}' rejected by registry policy"
                ))),
            }
        };

    // Offline / frozen provider: reproduce the locked graph from an existing
    // lockfile (whose entries already record their `require`s) so transitive
    // packages and direct/indirect classification survive without the network.
    let existing_lock = if offline {
        read_lockfile(&lockpath).ok()
    } else {
        None
    };
    let offline_provider =
        |name: &str, _version: &Version| -> Result<Vec<PackageRef>, tcl_pkg::TclPkgError> {
            let reqs = existing_lock
                .as_ref()
                .and_then(|lf| lf.lookup(name))
                .map(|p| {
                    p.requires
                        .iter()
                        .filter_map(|s| parse_locked_ref(s))
                        .collect()
                })
                .unwrap_or_default();
            Ok(reqs)
        };

    let provider_ref: &tcl_pkg::resolver::PackageManifestProvider = if offline {
        &offline_provider
    } else {
        &online_provider
    };

    let input = ResolveInput {
        direct,
        dev_direct,
        replaces,
        excludes,
        provider: Some(provider_ref),
        include_dev: !no_dev,
    };
    let resolved = match resolve(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    drop(input);

    let mut lf = LockFile::new(manifest.name.clone(), manifest.tcl_constraint.clone());
    lf.stamp();
    for rp in &resolved {
        let name = rp.reference.name.clone();
        let version = rp.reference.version.to_string();
        let mut entry = LockedPackage {
            name: name.clone(),
            version: version.clone(),
            source: SourceSpec::new("tarball", ""),
            integrity: String::new(),
            size: 0,
            requires: rp.requires.iter().map(ToString::to_string).collect(),
            provides: Vec::new(),
            license: String::new(),
            dev: rp.dev,
        };
        if !offline {
            // Reuses the cached fetch performed during resolution.
            match fetch_one(&name, &version) {
                Ok(Some(result)) => {
                    if let Err(e) = installer::materialise(
                        &cas,
                        &result.integrity,
                        &lib_dir,
                        &name,
                        &version,
                        true,
                    ) {
                        println!("{}", ui::warn(&format!("{name} {version}: {e}"), colour));
                    }
                    entry.source = result.source;
                    entry.integrity = result.integrity;
                    entry.size = result.size;
                    entry.provides = result.provides;
                    entry.license = result.license;

                    // Operator post-fetch hook (scanner / provenance / deny):
                    // a non-zero exit aborts the install.
                    let fetch_ctx = tcl_pkg::hooks::HookContext::new(&project_dir)
                        .var("NAME", name.clone())
                        .var("VERSION", version.clone())
                        .var("SOURCE_URL", entry.source.url.clone())
                        .var("INTEGRITY", entry.integrity.clone())
                        // Packages materialise at `lib/<name>-<version>`
                        // (see installer::materialise); point scanners there,
                        // not at the non-existent `lib/<name>`.
                        .var(
                            "PKG_DIR",
                            lib_dir.join(format!("{name}-{version}")).to_string_lossy(),
                        );
                    if let Err(e) = tcl_pkg::hooks::run_stage(
                        tcl_pkg::hooks::Stage::PostFetch,
                        &loaded.config,
                        &fetch_ctx,
                    ) {
                        eprintln!("error: {e}");
                        return Ok(1);
                    }
                }
                Ok(None) => {}
                Err(FetchError::PolicyRejected(url)) => {
                    eprintln!(
                        "error: {name} {version}: source '{url}' rejected by registry policy"
                    );
                    return Ok(1);
                }
                Err(FetchError::Fetch { source, message }) => {
                    entry.source = source;
                    println!(
                        "{}",
                        ui::warn(&format!("{name} {version}: {message}"), colour)
                    );
                }
            }
        }
        lf.packages.push(entry);
    }

    // Enforce integrity / post-install policy before the lockfile is written.
    if loaded.config.verification.require_integrity && !offline {
        let missing: Vec<&str> = lf
            .packages
            .iter()
            .filter(|p| p.integrity.is_empty())
            .map(|p| p.name.as_str())
            .collect();
        if !missing.is_empty() {
            eprintln!(
                "error: policy requires integrity hashes but these have none: {}",
                missing.join(", ")
            );
            return Ok(1);
        }
    }
    if let Err(e) = tcl_pkg::hooks::run_stage(
        tcl_pkg::hooks::Stage::PostInstall,
        &loaded.config,
        &install_ctx,
    ) {
        eprintln!("error: {e}");
        return Ok(1);
    }

    if frozen {
        if !lockpath.is_file() {
            eprintln!("error: --frozen requires an existing tclpkg.lock");
            return Ok(1);
        }
        let existing = match read_lockfile(&lockpath) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(1);
            }
        };
        let mut old: Vec<(String, String)> = existing
            .packages
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();
        let mut new: Vec<(String, String)> = lf
            .packages
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();
        old.sort();
        new.sort();
        if old != new {
            eprintln!(
                "error: lockfile would change but --frozen is set; update the manifest and re-run without --frozen"
            );
            return Ok(1);
        }
        if common.json {
            println!(
                "{}",
                ui::json_output(&json!({"packages": lf.packages.len(), "frozen": true}))
            );
        } else {
            println!(
                "{}",
                ui::ok(
                    &format!("lockfile is up to date ({} packages)", lf.packages.len()),
                    colour
                )
            );
        }
        return Ok(0);
    }

    if let Err(e) = write_lockfile(&lf, &lockpath) {
        eprintln!("error: {e}");
        return Ok(1);
    }

    if common.json {
        println!(
            "{}",
            ui::json_output(
                &json!({"packages": lf.packages.len(), "lockfile": lockpath.to_string_lossy()})
            )
        );
    } else {
        for pkg in &lf.packages {
            let dev_tag = if pkg.dev { " (dev)" } else { "" };
            println!(
                "{}",
                ui::ok(
                    &format!("{:<20} {}{}", pkg.name, pkg.version, dev_tag),
                    colour
                )
            );
        }
        println!(
            "{}",
            ui::ok(&format!("wrote {}", lockpath.display()), colour)
        );
    }
    Ok(0)
}

fn read_lock_or_report(common: &PkgCommon) -> Result<LockFile, u8> {
    match read_lockfile(lockfile_path(common)) {
        Ok(l) => Ok(l),
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn run_list(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    if common.json {
        let pkgs: Vec<Value> = lf.packages.iter().map(locked_to_json).collect();
        println!("{}", ui::json_output(&json!({"packages": pkgs})));
    } else {
        println!(
            "{:<20} {:<12} {:<8} {:<8}",
            "NAME", "VERSION", "KIND", "DEV"
        );
        let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        for pkg in pkgs {
            // A package is transitive when some *other* locked package requires
            // it; otherwise it was requested directly.
            let is_transitive = lf.packages.iter().any(|other| {
                other.name != pkg.name
                    && other
                        .requires
                        .iter()
                        .any(|r| locked_req_name(r) == pkg.name)
            });
            let kind = if is_transitive { "trans" } else { "direct" };
            let dev = if pkg.dev { "dev" } else { "" };
            println!(
                "{:<20} {:<12} {:<8} {:<8}",
                pkg.name, pkg.version, kind, dev
            );
        }
    }
    Ok(0)
}

fn run_tree(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    if common.json {
        let pkgs: Vec<Value> = lf.packages.iter().map(locked_to_json).collect();
        println!(
            "{}",
            ui::json_output(&json!({"name": lf.name, "packages": pkgs}))
        );
    } else {
        println!("{}", lf.name);
        let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        let total = pkgs.len();
        for (i, pkg) in pkgs.iter().enumerate() {
            let connector = if i == total - 1 {
                "└── "
            } else {
                "├── "
            };
            let dev_tag = if pkg.dev { " [dev]" } else { "" };
            println!("{connector}{} {}{dev_tag}", pkg.name, pkg.version);
        }
    }
    Ok(0)
}

fn run_verify(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let colour = ui::use_colour_for_json(common.json);
    let lib_dir = install_lib_dir(&manifest_path(common));
    let cas = ContentAddressableStore::new(&tcl_pkg::cache_dir());
    let mut failures = 0;
    let mut mismatches = 0;
    for pkg in &lf.packages {
        if pkg.integrity.is_empty() {
            println!(
                "{}",
                ui::warn(
                    &format!("{:<20} {:<12} no integrity hash", pkg.name, pkg.version),
                    colour
                )
            );
            failures += 1;
            continue;
        }

        // Actually recompute and compare the hash, rather than merely checking
        // the recorded string is non-empty. Prefer the
        // materialised worktree the runtime actually loads
        // (`lib/<name>-<version>`); fall back to the CAS tree. A missing copy
        // cannot be verified — flag it rather than passing it.
        let installed = lib_dir.join(format!("{}-{}", pkg.name, pkg.version));
        let target = if installed.is_dir() {
            Some(("lib", installed))
        } else {
            match cas.tree_path(&pkg.integrity) {
                Ok(tree) => Some(("cache", tree)),
                Err(_) => None,
            }
        };
        let Some((source, root)) = target else {
            println!(
                "{}",
                ui::warn(
                    &format!(
                        "{:<20} {:<12} not materialised (run 'tcl pkg install')",
                        pkg.name, pkg.version
                    ),
                    colour
                )
            );
            failures += 1;
            continue;
        };

        if tcl_pkg::cas::verify_integrity(&root, &pkg.integrity) {
            let trunc: String = pkg.integrity.chars().take(30).collect();
            println!(
                "{}",
                ui::ok(
                    &format!("{:<20} {:<12} {trunc}… ({source})", pkg.name, pkg.version),
                    colour
                )
            );
        } else {
            println!(
                "{}",
                ui::warn(
                    &format!(
                        "{:<20} {:<12} INTEGRITY MISMATCH ({source} content tampered)",
                        pkg.name, pkg.version
                    ),
                    colour
                )
            );
            mismatches += 1;
        }
    }
    if mismatches > 0 {
        eprintln!(
            "\n{mismatches} package(s) failed integrity verification — the materialised \
             content does not match the lockfile hash."
        );
        return Ok(1);
    }
    if failures > 0 {
        eprintln!(
            "\n{failures} package(s) could not be verified (no hash or not materialised) — \
             run 'tcl pkg install' to populate."
        );
        return Ok(1);
    }
    Ok(0)
}

fn run_info(package: &str, common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let Some(entry) = lf.lookup(package) else {
        eprintln!("error: package '{package}' not found in lockfile");
        return Ok(1);
    };
    if common.json {
        println!("{}", ui::json_output(&locked_to_json(entry)));
    } else {
        println!("Name:      {}", entry.name);
        println!("Version:   {}", entry.version);
        println!("Source:    {} {}", entry.source.kind, entry.source.url);
        let integ = if entry.integrity.is_empty() {
            "(not computed)"
        } else {
            &entry.integrity
        };
        println!("Integrity: {integ}");
        let lic = if entry.license.is_empty() {
            "(unknown)"
        } else {
            &entry.license
        };
        println!("Licence:   {lic}");
        println!("Dev:       {}", if entry.dev { "yes" } else { "no" });
        if !entry.requires.is_empty() {
            println!("Requires:  {}", entry.requires.join(", "));
        }
        if !entry.provides.is_empty() {
            println!("Provides:  {}", entry.provides.join(", "));
        }
    }
    Ok(0)
}

fn run_add(
    package: &str,
    min_version: Option<&str>,
    source: Option<&str>,
    dev: bool,
    common: &PkgCommon,
) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour_for_json(common.json);
    if let Err(e) = load_manifest(&mpath) {
        eprintln!("error: {e}");
        return Ok(1);
    }
    let min_ver = min_version.unwrap_or("0.0.1");
    let directive = if dev { "dev-require" } else { "require" };
    let mut new_line = format!("{directive} {package} {min_ver}");
    if let Some(url) = source {
        new_line.push_str(" -source ");
        new_line.push_str(url);
    }
    let text = match std::fs::read_to_string(&mpath) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    let new_text = format!("{}\n{new_line}\n", text.trim_end_matches('\n'));
    if let Err(e) = std::fs::write(&mpath, new_text) {
        eprintln!("error: {e}");
        return Ok(1);
    }
    if common.json {
        println!(
            "{}",
            ui::json_output(&json!({"added": package, "version": min_ver, "dev": dev}))
        );
    } else {
        println!(
            "{}",
            ui::ok(
                &format!("added {package} {min_ver} to {}", mpath.display()),
                colour
            )
        );
        println!(
            "{}",
            ui::dim("  run 'tcl pkg install' to resolve and lock", colour)
        );
    }
    Ok(0)
}

fn run_remove(package: &str, common: &PkgCommon) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour_for_json(common.json);
    if !mpath.is_file() {
        eprintln!("error: manifest not found: {}", mpath.display());
        return Ok(1);
    }
    let text = match std::fs::read_to_string(&mpath) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    let mut count = 0;
    let mut kept: Vec<String> = Vec::new();
    for line in text.split_inclusive('\n') {
        if line_is_requirement_for(line, package) {
            count += 1;
        } else {
            kept.push(line.to_string());
        }
    }
    if count == 0 {
        eprintln!("error: package '{package}' not found in manifest");
        return Ok(1);
    }
    if let Err(e) = std::fs::write(&mpath, kept.concat()) {
        eprintln!("error: {e}");
        return Ok(1);
    }
    if common.json {
        println!(
            "{}",
            ui::json_output(&json!({"removed": package, "directives_removed": count}))
        );
    } else {
        println!(
            "{}",
            ui::ok(
                &format!("removed {package} from {}", mpath.display()),
                colour
            )
        );
        println!(
            "{}",
            ui::dim("  run 'tcl pkg install' to update the lockfile", colour)
        );
    }
    Ok(0)
}

/// Whether `line` is a `require`/`dev-require` directive for `package`
/// (matches `^\s*(?:require|dev-require)\s+<pkg>\b.*$`).
fn line_is_requirement_for(line: &str, package: &str) -> bool {
    let trimmed = line.trim_start();
    for prefix in ["require", "dev-require"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let mut chars = rest.chars();
            if let Some(c) = chars.next()
                && c.is_whitespace()
            {
                let after = rest.trim_start();
                if let Some(tail) = after.strip_prefix(package) {
                    // Require a word boundary after the package name.
                    if tail.chars().next().is_none_or(|c| !is_word_char(c)) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn run_update(packages: &[String], common: &PkgCommon) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour_for_json(common.json);
    let manifest = match load_manifest(&mpath) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    let mut all_names: Vec<String> = Vec::new();
    for r in manifest.requires.iter().chain(manifest.dev_requires.iter()) {
        all_names.push(r.name.clone());
    }
    let targets: Vec<String> = if packages.is_empty() {
        all_names.clone()
    } else {
        packages.to_vec()
    };
    let mut updated: Vec<String> = Vec::new();
    for name in &targets {
        if all_names.iter().any(|n| n == name) {
            updated.push(name.clone());
        } else {
            eprintln!("  skip: {name} not in manifest");
        }
    }
    if common.json {
        println!("{}", ui::json_output(&json!({"updated": updated})));
    } else if updated.is_empty() {
        println!("  nothing to update");
    } else {
        for name in &updated {
            println!(
                "{}",
                ui::ok(
                    &format!("{name} (already at minimum — registry query not yet wired)"),
                    colour
                )
            );
        }
        println!(
            "{}",
            ui::dim("  run 'tcl pkg install' to re-resolve", colour)
        );
    }
    Ok(0)
}

/// Ensure one locked package's content is in the CAS (fetching from its
/// recorded source when missing, unless offline) and materialise it into
/// `lib/<name>-<version>`. Returns `true` on success. Shared by [`run_sync`].
fn sync_one_package(
    pkg: &LockedPackage,
    cas: &ContentAddressableStore,
    lib_dir: &Path,
    common: &PkgCommon,
    colour: bool,
) -> bool {
    if pkg.integrity.is_empty() {
        if !common.json {
            println!(
                "{}",
                ui::warn(
                    &format!("{:<20} {:<12} no integrity hash", pkg.name, pkg.version),
                    colour
                )
            );
        }
        return false;
    }
    if !cas.has(&pkg.integrity) {
        if common.offline {
            if !common.json {
                println!(
                    "{}",
                    ui::warn(
                        &format!(
                            "{:<20} {:<12} not cached and --offline set",
                            pkg.name, pkg.version
                        ),
                        colour
                    )
                );
            }
            return false;
        }
        match installer::fetch_and_store(&pkg.source, &pkg.name, &pkg.version, cas, 60) {
            Ok(result) if result.integrity == pkg.integrity => {}
            Ok(result) => {
                eprintln!(
                    "error: {} {}: integrity mismatch (lock {}, fetched {})",
                    pkg.name, pkg.version, pkg.integrity, result.integrity
                );
                return false;
            }
            Err(e) => {
                eprintln!("error: {} {}: {e}", pkg.name, pkg.version);
                return false;
            }
        }
    }
    match installer::materialise(cas, &pkg.integrity, lib_dir, &pkg.name, &pkg.version, true) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("error: {} {}: {e}", pkg.name, pkg.version);
            false
        }
    }
}

fn run_sync(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let colour = ui::use_colour_for_json(common.json);
    let lockpath = lockfile_path(common);
    let lib_dir = install_lib_dir(&manifest_path(common));
    let cas = ContentAddressableStore::new(&tcl_pkg::cache_dir());

    // A lock-driven install must actually materialise each locked package into
    // `lib/<name>-<version>`, not merely print the lockfile.
    // For each package: ensure its content is in the CAS (fetch from the
    // recorded source when missing, unless offline), enforce that the fetched
    // content matches the locked integrity, then materialise it.
    let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
    pkgs.sort_by(|a, b| a.name.cmp(&b.name));
    let mut synced: Vec<String> = Vec::new();
    let mut failures = 0;
    for pkg in &pkgs {
        if sync_one_package(pkg, &cas, &lib_dir, common, colour) {
            synced.push(pkg.name.clone());
        } else {
            failures += 1;
        }
    }

    if common.json {
        println!(
            "{}",
            ui::json_output(&json!({
                "synced": synced.len(),
                "failed": failures,
                "lockfile": lockpath.to_string_lossy(),
            }))
        );
    } else {
        for name in &synced {
            println!("{}", ui::ok(name, colour));
        }
        println!(
            "{}",
            ui::ok(
                &format!(
                    "synced {} package(s) into {}",
                    synced.len(),
                    lib_dir.display()
                ),
                colour
            )
        );
    }
    if failures > 0 {
        eprintln!("\n{failures} package(s) could not be synced.");
        return Ok(1);
    }
    Ok(0)
}

fn run_outdated(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    if common.json {
        println!("{}", ui::json_output(&json!({"outdated": []})));
    } else {
        println!("{:<20} {:<12} {:<12}", "NAME", "CURRENT", "LATEST");
        let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        for pkg in &pkgs {
            println!("{:<20} {:<12} {:<12}", pkg.name, pkg.version, pkg.version);
        }
        println!(
            "{}",
            ui::dim(
                "\n  (registry version lookup not yet wired)",
                ui::use_colour(None)
            )
        );
    }
    Ok(0)
}

/// Whether any of `requires` (a list of `name@version` requirement strings)
/// names `package`.  Matches the requirement's package *name* — the part
/// before `@` — not a substring, so `why http` does not falsely match a
/// dependent that requires `shttp@1.0` or `http2@…` (issue 197).
fn requires_package(requires: &[String], package: &str) -> bool {
    requires.iter().any(|r| locked_req_name(r) == package)
}

fn run_why(package: &str, common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let Some(entry) = lf.lookup(package) else {
        eprintln!("error: package '{package}' not found in lockfile");
        return Ok(1);
    };
    let dependents: Vec<&LockedPackage> = lf
        .packages
        .iter()
        .filter(|other| requires_package(&other.requires, package))
        .collect();
    if common.json {
        let names: Vec<String> = dependents.iter().map(|d| d.name.clone()).collect();
        println!(
            "{}",
            ui::json_output(&json!({
                "package": package,
                "version": entry.version,
                "required_by": names,
                "dev": entry.dev,
            }))
        );
    } else {
        println!("{package} {}", entry.version);
        if !dependents.is_empty() {
            for dep in &dependents {
                println!("└── required by {} {}", dep.name, dep.version);
            }
        } else if entry.dev {
            println!("└── direct dev-require dependency");
        } else {
            println!("└── direct dependency");
        }
    }
    Ok(0)
}

fn run_vendor(dir: &Path, common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let colour = ui::use_colour_for_json(common.json);
    let cas = ContentAddressableStore::new(&tcl_pkg::cache_dir());
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("error: {e}");
        return Ok(1);
    }
    let mut vendored: Vec<String> = Vec::new();
    for pkg in &lf.packages {
        let dest = dir.join(format!("{}-{}", pkg.name, pkg.version));
        if !pkg.integrity.is_empty() && cas.has(&pkg.integrity) {
            if cas.materialise(&pkg.integrity, &dest, false).is_ok() {
                vendored.push(pkg.name.clone());
            }
        } else if pkg.integrity.is_empty() {
            println!(
                "{}",
                ui::warn(
                    &format!("{} has no integrity hash — skipping", pkg.name),
                    colour
                )
            );
        }
    }
    if common.json {
        println!(
            "{}",
            ui::json_output(&json!({"vendored": vendored, "dir": dir.to_string_lossy()}))
        );
    } else if vendored.is_empty() {
        println!(
            "{}",
            ui::dim("  no packages with integrity hashes to vendor", colour)
        );
        println!(
            "{}",
            ui::dim(
                "  run 'tcl pkg install' first to populate the cache",
                colour
            )
        );
    } else {
        for name in &vendored {
            println!("{}", ui::ok(&format!("vendored {name}"), colour));
        }
        println!(
            "{}",
            ui::ok(
                &format!("wrote {} package(s) to {}/", vendored.len(), dir.display()),
                colour
            )
        );
    }
    Ok(0)
}

fn run_run(extra: &[String], common: &PkgCommon) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let manifest = match load_manifest(&mpath) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if manifest.entry.is_empty() {
        eprintln!("error: no 'entry' directive in manifest");
        return Ok(1);
    }
    let parent = mpath.parent().unwrap_or_else(|| Path::new("."));
    let entry_path = parent.join(&manifest.entry);
    if !entry_path.is_file() {
        eprintln!("error: entry file not found: {}", entry_path.display());
        return Ok(1);
    }

    let tclsh = if let Some(venv) = std::env::var_os("TCL_VENV") {
        PathBuf::from(venv).join("bin").join("tclsh")
    } else if let Some(p) = tcl_pkg::venv::find_tclsh() {
        p
    } else {
        eprintln!("error: tclsh not found on PATH");
        return Ok(1);
    };

    // Operator hooks + audit + policy floor apply even to running the project's
    // own entry point ("anything tcl pkg runs is sandboxed").
    let loaded = tcl_pkg::policy::load(Some(parent));
    let ctx = tcl_pkg::hooks::HookContext::new(parent)
        .var("ENTRY", entry_path.to_string_lossy())
        .var("MANIFEST", mpath.to_string_lossy());
    if let Err(e) = tcl_pkg::hooks::run_stage(tcl_pkg::hooks::Stage::PreRun, &loaded.config, &ctx) {
        eprintln!("error: {e}");
        return Ok(1);
    }

    let mut profile = tcl_sandbox::Profile::new("pkg-run", &tclsh, parent)
        .user_code()
        .arg(entry_path.to_string_lossy())
        .args(extra.iter().cloned());
    let lib_dir = parent.join("lib");
    if lib_dir.is_dir() {
        let existing = std::env::var("TCLLIBPATH").unwrap_or_default();
        let value = if existing.is_empty() {
            lib_dir.to_string_lossy().into_owned()
        } else {
            format!("{} {existing}", lib_dir.display())
        };
        profile = profile.set_env("TCLLIBPATH", value);
    }

    let code = match tcl_pkg::exec::execute(&profile, &loaded.config.sandbox_policy()) {
        Ok(out) => u8::try_from(out.code.unwrap_or(1)).unwrap_or(1),
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

    if let Err(e) = tcl_pkg::hooks::run_stage(tcl_pkg::hooks::Stage::PostRun, &loaded.config, &ctx)
    {
        eprintln!("error: {e}");
        return Ok(1);
    }
    Ok(code)
}

fn run_freeze(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
    pkgs.sort_by(|a, b| a.name.cmp(&b.name));
    if common.json {
        let mut map = Map::new();
        for pkg in &pkgs {
            map.insert(pkg.name.clone(), Value::String(pkg.version.clone()));
        }
        println!("{}", ui::json_output(&Value::Object(map)));
    } else {
        for pkg in &pkgs {
            let source_suffix = if pkg.source.url.is_empty() {
                String::new()
            } else {
                format!(" -source {}", pkg.source.url)
            };
            let directive = if pkg.dev { "dev-require" } else { "require" };
            println!("{directive} {} {}{source_suffix}", pkg.name, pkg.version);
        }
    }
    Ok(0)
}

fn run_search(query: &str, json: bool, offline: bool) -> anyhow::Result<u8> {
    let mut client = RegistryClient::new(&tcl_pkg::cache_dir(), offline);
    let results = match client.search(query) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if json {
        let arr: Vec<Value> = results
            .iter()
            .map(|e| json!({"name": e.name, "description": e.description}))
            .collect();
        println!("{}", ui::json_output(&Value::Array(arr)));
    } else if results.is_empty() {
        println!("No matches found.");
    } else {
        for entry in &results {
            println!("  {:<20}  {}", entry.name, entry.description);
        }
    }
    Ok(0)
}

fn run_build(common: &PkgCommon) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour_for_json(common.json);
    let manifest = match load_manifest(&mpath) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    if manifest.build.script.is_empty() {
        eprintln!("error: no 'build' directive in manifest");
        return Ok(1);
    }
    let project_dir = mpath
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let loaded = tcl_pkg::policy::load(Some(&project_dir));

    // Packages are data by default: a build script runs only when the operator
    // both enables build scripts and trusts this package.
    if !loaded.config.build_script_allowed(&manifest.name) {
        eprintln!(
            "error: build script for '{}' is not permitted by policy",
            manifest.name
        );
        eprintln!(
            "  hint: set [build] allow-build-scripts = true and run 'tcl pkg trust {}'",
            manifest.name
        );
        return Ok(1);
    }

    let script = project_dir.join(&manifest.build.script);
    if !script.is_file() {
        eprintln!("error: build script not found: {}", script.display());
        return Ok(1);
    }
    let tclsh = if let Some(venv) = std::env::var_os("TCL_VENV") {
        PathBuf::from(venv).join("bin").join("tclsh")
    } else if let Some(p) = tcl_pkg::venv::find_tclsh() {
        p
    } else {
        eprintln!("error: tclsh not found on PATH");
        return Ok(1);
    };

    let ctx = tcl_pkg::hooks::HookContext::new(&project_dir)
        .var("NAME", manifest.name.clone())
        .var("BUILD_SCRIPT", script.to_string_lossy());
    if let Err(e) = tcl_pkg::hooks::run_stage(tcl_pkg::hooks::Stage::PreBuild, &loaded.config, &ctx)
    {
        eprintln!("error: {e}");
        return Ok(1);
    }

    // Deprivileged: network denied unless declared (and still policy-clamped),
    // environment scrubbed of secrets, confined to the project directory, output
    // streamed. A throwaway HOME keeps the script away from the user's dotfiles.
    let build_home = project_dir.join(".tclpkg-build-home");
    let _ = std::fs::create_dir_all(&build_home);
    let mut profile = tcl_sandbox::Profile::new("pkg-build", &tclsh, &project_dir)
        .arg(script.to_string_lossy())
        .network(manifest.build.network)
        .capture(false)
        .set_env("HOME", build_home.to_string_lossy())
        .set_env("TCLPKG_BUILD", "1");
    for name in ["PATH", "TCL_LIBRARY", "TCLLIBPATH", "TMPDIR"] {
        profile = profile.pass_env(name);
    }
    profile.fs_write = vec![project_dir.clone()];
    profile.fs_read = vec![project_dir.clone()];

    println!(
        "{}",
        ui::dim(
            &format!(
                "running build script {} (deprivileged)",
                manifest.build.script
            ),
            colour
        )
    );
    let code = match tcl_pkg::exec::execute(&profile, &loaded.config.sandbox_policy()) {
        Ok(out) => {
            if out.isolation == tcl_sandbox::IsolationLevel::Baseline {
                eprintln!(
                    "{}",
                    ui::warn(
                        "build ran with baseline isolation only (no OS-native filesystem/network confinement on this host)",
                        colour
                    )
                );
            }
            u8::try_from(out.code.unwrap_or(1)).unwrap_or(1)
        }
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

    if let Err(e) =
        tcl_pkg::hooks::run_stage(tcl_pkg::hooks::Stage::PostBuild, &loaded.config, &ctx)
    {
        eprintln!("error: {e}");
        return Ok(1);
    }
    if code == 0 {
        println!("{}", ui::ok("build complete", colour));
    }
    Ok(code)
}

fn run_policy(action: &PolicyAction) -> anyhow::Result<u8> {
    let project = find_project_root();
    let loaded = tcl_pkg::policy::load(project.as_deref());
    match action {
        PolicyAction::Show { json } => policy_show(&loaded, *json),
        PolicyAction::Verify { json } => policy_verify(&loaded, *json),
    }
}

fn policy_show(loaded: &tcl_pkg::policy::LoadedPolicy, json: bool) -> anyhow::Result<u8> {
    let cfg = &loaded.config;
    if json {
        let sources: Vec<Value> = loaded
            .sources
            .iter()
            .map(|s| {
                json!({
                    "layer": s.layer,
                    "path": s.path.to_string_lossy(),
                    "loaded": s.loaded,
                    "note": s.note,
                })
            })
            .collect();
        let config = serde_json::to_value(cfg).unwrap_or(Value::Null);
        println!(
            "{}",
            ui::json_output(&json!({
                "sources": sources,
                "locked": loaded.locked,
                "warnings": loaded.warnings,
                "config": config,
            }))
        );
        return Ok(0);
    }
    let colour = ui::use_colour(None);
    println!("Policy layers (low → high precedence):");
    for s in &loaded.sources {
        let status = if s.loaded { "loaded" } else { "—" };
        let note = s
            .note
            .as_deref()
            .map_or(String::new(), |n| format!("  ({n})"));
        println!("  {:<8} {} [{status}]{note}", s.layer, s.path.display());
    }
    println!("\nSandbox floor:");
    println!("  fail-closed         {}", cfg.sandbox.fail_closed);
    println!("  deny-network        {}", cfg.sandbox.deny_network);
    println!(
        "  require-network-deny {}",
        cfg.sandbox.require_network_deny
    );
    if let Some(t) = cfg.sandbox.max_timeout_secs {
        println!("  max-timeout-secs    {t}");
    }
    println!("\nRegistry:");
    println!("  require-https {}", cfg.registry.require_https);
    println!("  allow         {:?}", cfg.registry.allow);
    println!("  deny          {:?}", cfg.registry.deny);
    println!("\nVerification:");
    println!(
        "  require-integrity  {}",
        cfg.verification.require_integrity
    );
    println!(
        "  require-provenance {}",
        cfg.verification.require_provenance
    );
    println!("  cooldown-days      {}", cfg.cooldown.min_release_age_days);
    println!("\nBuild scripts:");
    println!("  allow-build-scripts {}", cfg.build.allow_build_scripts);
    println!("  trusted             {:?}", cfg.build.trusted);
    println!("\nHooks: {} configured", cfg.hooks.len());
    if loaded.locked.is_empty() {
        println!("\nAdmin-locked keys: none");
    } else {
        println!("\nAdmin-locked keys:");
        for k in &loaded.locked {
            println!("  {k}");
        }
    }
    if !loaded.warnings.is_empty() {
        println!();
        for w in &loaded.warnings {
            println!("{}", ui::warn(w, colour));
        }
    }
    Ok(0)
}

fn policy_verify(loaded: &tcl_pkg::policy::LoadedPolicy, json: bool) -> anyhow::Result<u8> {
    let ok = loaded.warnings.is_empty();
    if json {
        println!(
            "{}",
            ui::json_output(&json!({"ok": ok, "warnings": loaded.warnings}))
        );
    } else {
        let colour = ui::use_colour(None);
        if ok {
            println!("{}", ui::ok("policy OK", colour));
        } else {
            for w in &loaded.warnings {
                println!("{}", ui::warn(w, colour));
            }
        }
    }
    Ok(u8::from(!ok))
}

fn run_hooks(json: bool) -> anyhow::Result<u8> {
    let project = find_project_root();
    let loaded = tcl_pkg::policy::load(project.as_deref());
    let rows = tcl_pkg::hooks::describe(&loaded.config);
    if json {
        let arr: Vec<Value> = rows
            .iter()
            .map(|(stage, name, cmd)| json!({"stage": stage, "name": name, "command": cmd}))
            .collect();
        println!("{}", ui::json_output(&json!({"hooks": arr})));
    } else if rows.is_empty() {
        println!("No operator hooks configured.");
    } else {
        println!("{:<14} {:<16} COMMAND", "STAGE", "NAME");
        for (stage, name, cmd) in &rows {
            println!("{stage:<14} {name:<16} {cmd}");
        }
    }
    Ok(0)
}

fn run_audit(lines: usize, json: bool) -> anyhow::Result<u8> {
    let path = tcl_pkg::exec::audit_log_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let all: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = all.len().saturating_sub(lines);
    let tail = &all[start..];
    if json {
        let arr: Vec<Value> = tail
            .iter()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .collect();
        println!("{}", ui::json_output(&Value::Array(arr)));
    } else if tail.is_empty() {
        println!("No audit records ({}).", path.display());
    } else {
        for l in tail {
            println!("{l}");
        }
    }
    Ok(0)
}

fn run_trust(package: &str, remove: bool) -> anyhow::Result<u8> {
    let colour = ui::use_colour(None);
    match tcl_pkg::policy::set_trusted(package, !remove) {
        Ok(path) => {
            let verb = if remove { "untrusted" } else { "trusted" };
            println!(
                "{}",
                ui::ok(&format!("{verb} {package} (in {})", path.display()), colour)
            );
            Ok(0)
        }
        Err(e) => {
            eprintln!("error: {e}");
            Ok(1)
        }
    }
}

/// Build the canonical JSON object for a locked package. Keys are sorted by the
/// canonical-JSON emitter.
fn locked_to_json(pkg: &LockedPackage) -> Value {
    let mut provides = pkg.provides.clone();
    provides.sort();
    let mut requires = pkg.requires.clone();
    requires.sort();
    json!({
        "dev": pkg.dev,
        "integrity": pkg.integrity,
        "license": pkg.license,
        "name": pkg.name,
        "provides": provides,
        "requires": requires,
        "size": pkg.size,
        "source": {
            "type": pkg.source.kind,
            "url": pkg.source.url,
            "subdir": pkg.source.subdir,
            "rev": pkg.source.rev,
        },
        "version": pkg.version,
    })
}

#[cfg(test)]
mod why_tests {
    use super::requires_package;

    #[test]
    fn requires_package_matches_name_not_substring() {
        let reqs = vec![
            "shttp@1.0".to_string(),
            "http2@3.0".to_string(),
            "http@2.0".to_string(),
        ];
        // Exact name match only.
        assert!(requires_package(&reqs, "http"));
        assert!(requires_package(&reqs, "shttp"));
        assert!(requires_package(&reqs, "http2"));
        // Substrings that are not the full name must NOT match (issue 197).
        assert!(!requires_package(&reqs, "ttp"));
        assert!(!requires_package(&reqs, "htt"));
        // A dependent whose only requirement is `shttp` is not a dependent of
        // `http`.
        assert!(!requires_package(&["shttp@1.0".to_string()], "http"));
    }
}
