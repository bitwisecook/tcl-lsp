//! `tcl pkg` verb group — Tcl package management.
//!
//! Port of `tooling/tcl/verbs/pkg.py`. Thin wrappers over the `tcl_pkg`
//! modules: parse args, drive the library, format output. Handler-level errors
//! print `error: <msg>` to stderr and return exit code 1, matching the Python
//! verb's `except Exception` paths.

// Handlers return `anyhow::Result<u8>` for a uniform dispatch signature even
// when a given verb cannot fail; the wrap is the interface contract.
#![allow(clippy::unnecessary_wraps)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value, json};
use tcl_pkg::cas::ContentAddressableStore;
use tcl_pkg::installer;
use tcl_pkg::lockfile::{LockFile, LockedPackage, SourceSpec, read_lockfile, write_lockfile};
use tcl_pkg::manifest::load_manifest;
use tcl_pkg::registry::RegistryClient;
use tcl_pkg::resolver::{ExcludeSpec, PackageRef, ReplaceSpec, ResolveInput, resolve};
use tcl_pkg::ui;

use crate::cli::{PkgCommand, PkgCommon};

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

    let colour = ui::use_colour(Some(!json));
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

#[allow(clippy::too_many_lines)] // direct port of the Python `_run_install`
fn run_install(common: &PkgCommon, no_dev: bool, frozen: bool) -> anyhow::Result<u8> {
    let mpath = manifest_path(common);
    let colour = ui::use_colour(Some(!common.json));

    let manifest = match load_manifest(&mpath) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

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

    let input = ResolveInput {
        direct,
        dev_direct,
        replaces,
        excludes,
        provider: None,
        include_dev: !no_dev,
    };
    let resolved = match resolve(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };

    // Source lookup tables for materialisation.
    let mut manifest_sources: HashMap<String, String> = HashMap::new();
    for r in manifest.requires.iter().chain(manifest.dev_requires.iter()) {
        if let Some(url) = &r.source_url {
            manifest_sources.insert(r.name.clone(), url.clone());
        }
    }
    let mut replace_sources: HashMap<String, String> = HashMap::new();
    for r in &manifest.replaces {
        replace_sources.insert(r.name.clone(), r.source_url.clone());
    }
    // --frozen never touches the network or the tree.
    let offline = frozen || common.offline;
    let cache = tcl_pkg::cache_dir();
    let cas = ContentAddressableStore::new(&cache);
    let mut registry = RegistryClient::new(&cache, common.offline);
    let lib_dir = install_lib_dir(&mpath);

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
            let source = installer::resolve_source(
                &name,
                &version,
                &replace_sources,
                &manifest_sources,
                Some(&mut registry),
            );
            if let Some(source) = source {
                match installer::fetch_and_store(&source, &name, &version, &cas, 60) {
                    Ok(result) => {
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
                    }
                    Err(e) => {
                        entry.source = source;
                        println!("{}", ui::warn(&format!("{name} {version}: {e}"), colour));
                    }
                }
            }
        }
        lf.packages.push(entry);
    }

    let lockpath = mpath
        .parent()
        .map_or_else(|| PathBuf::from("tclpkg.lock"), |p| p.join("tclpkg.lock"));

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
            let kind = if pkg.requires.is_empty() {
                "direct"
            } else {
                "trans"
            };
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
    let colour = ui::use_colour(Some(!common.json));
    let mut failures = 0;
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
        } else {
            let trunc: String = pkg.integrity.chars().take(30).collect();
            println!(
                "{}",
                ui::ok(
                    &format!("{:<20} {:<12} {trunc}…", pkg.name, pkg.version),
                    colour
                )
            );
        }
    }
    if failures > 0 {
        eprintln!(
            "\n{failures} package(s) have no integrity hash — run 'tcl pkg install' to populate."
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
    let colour = ui::use_colour(Some(!common.json));
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
    let colour = ui::use_colour(Some(!common.json));
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
/// (mirrors the Python regex `^\s*(?:require|dev-require)\s+<pkg>\b.*$`).
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
    let colour = ui::use_colour(Some(!common.json));
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

fn run_sync(common: &PkgCommon) -> anyhow::Result<u8> {
    let lf = match read_lock_or_report(common) {
        Ok(l) => l,
        Err(code) => return Ok(code),
    };
    let colour = ui::use_colour(Some(!common.json));
    let lockpath = lockfile_path(common);
    if common.json {
        println!(
            "{}",
            ui::json_output(
                &json!({"packages": lf.packages.len(), "lockfile": lockpath.to_string_lossy()})
            )
        );
    } else {
        let mut pkgs: Vec<&LockedPackage> = lf.packages.iter().collect();
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        for pkg in &pkgs {
            println!(
                "{}",
                ui::ok(&format!("{:<20} {}", pkg.name, pkg.version), colour)
            );
        }
        println!(
            "{}",
            ui::ok(
                &format!(
                    "synced from {} ({} packages)",
                    lockpath.display(),
                    lf.packages.len()
                ),
                colour
            )
        );
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
        .filter(|other| other.requires.iter().any(|r| r.contains(package)))
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
    let colour = ui::use_colour(Some(!common.json));
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

    let mut cmd = Command::new(&tclsh);
    cmd.arg(&entry_path).args(extra);
    let lib_dir = parent.join("lib");
    if lib_dir.is_dir() {
        let existing = std::env::var("TCLLIBPATH").unwrap_or_default();
        let value = if existing.is_empty() {
            lib_dir.to_string_lossy().into_owned()
        } else {
            format!("{} {existing}", lib_dir.display())
        };
        cmd.env("TCLLIBPATH", value);
    }
    match cmd.status() {
        Ok(s) => Ok(u8::try_from(s.code().unwrap_or(1)).unwrap_or(1)),
        Err(e) => {
            eprintln!("error: {e}");
            Ok(1)
        }
    }
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

/// Build the canonical JSON object for a locked package (`LockedPackage.to_dict`
/// in Python). Keys are sorted by the canonical-JSON emitter.
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
