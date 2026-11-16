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

//! The Cargo workspace fixture the self-test builds: its files, its scratch directory and the rustc probe.

use super::{Fixture, NEXT_FIXTURE, Target};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{env, fs};

pub(super) fn create_fixture(mut next_root: impl FnMut() -> PathBuf) -> Result<Fixture> {
    loop {
        let root = next_root();
        match fs::create_dir(&root) {
            // Cargo reports canonical paths, and the temp directory reaches us
            // through a symlink on macOS (/var -> /private/var), so canonicalise
            // the root here or every strip_prefix against it misses.
            Ok(()) => {
                let root = fs::canonicalize(&root).unwrap_or(root);
                return Ok(Fixture { root });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("creating smoke fixture {}", root.display()));
            }
        }
    }
}

impl Fixture {
    pub(super) fn new() -> Result<Self> {
        create_fixture(|| {
            let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            env::temp_dir().join(format!(
                "tcl-lsp-smoke-targets-{}-{serial}",
                std::process::id()
            ))
        })
    }

    pub(super) fn write(&self, relative: &str, contents: &str) -> Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
    }

    #[cfg(unix)]
    fn write_executable(&self, relative: &str, contents: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        self.write(relative, contents)?;
        let path = self.root.join(relative);
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)
            .with_context(|| format!("making {} executable", path.display()))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
pub(super) fn rustc_probe_config(fixture: &Fixture) -> Result<String> {
    let probe = fixture.root.join("rustc-probe.sh");
    let marker = fixture.root.join("rustc-sysroot-queried");
    let wrapper = fixture.root.join("rustc-wrapper.sh");
    let wrapper_marker = fixture.root.join("rustc-wrapper-queried");
    let workspace_wrapper = fixture.root.join("rustc-workspace-wrapper.sh");
    let workspace_wrapper_marker = fixture.root.join("rustc-workspace-wrapper-queried");
    fixture.write_executable(
        "rustc-probe.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --print ] && [ \"$2\" = sysroot ]; then : > \"{}\"; fi\nexec rustc \"$@\"\n",
            marker.display()
        ),
    )?;
    fixture.write_executable(
        "rustc-wrapper.sh",
        &format!(
            "#!/bin/sh\n: > \"{}\"\nexec \"$@\"\n",
            wrapper_marker.display()
        ),
    )?;
    fixture.write_executable(
        "rustc-workspace-wrapper.sh",
        &format!(
            "#!/bin/sh\n: > \"{}\"\nexec \"$@\"\n",
            workspace_wrapper_marker.display()
        ),
    )?;
    Ok(format!(
        "rustc = {}\nrustc-wrapper = {}\nrustc-workspace-wrapper = {}\n",
        serde_json::to_string(&probe)?,
        serde_json::to_string(&wrapper)?,
        serde_json::to_string(&workspace_wrapper)?,
    ))
}

#[cfg(not(unix))]
pub(super) fn rustc_probe_config(_fixture: &Fixture) -> Result<String> {
    Ok(String::new())
}

pub(super) fn reset_rustc_probe(fixture: &Fixture) -> Result<()> {
    for name in [
        "rustc-sysroot-queried",
        "rustc-wrapper-queried",
        "rustc-workspace-wrapper-queried",
    ] {
        let marker = fixture.root.join(name);
        if marker.exists() {
            fs::remove_file(&marker).with_context(|| format!("removing {}", marker.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn verify_rustc_probe(fixture: &Fixture) -> Result<()> {
    for (name, context) in [
        ("rustc-sysroot-queried", "configured Cargo rustc"),
        ("rustc-wrapper-queried", "configured Cargo rustc wrapper"),
        (
            "rustc-workspace-wrapper-queried",
            "configured Cargo workspace rustc wrapper",
        ),
    ] {
        if !fixture.root.join(name).is_file() {
            bail!("{context} was not used for the sysroot query");
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn verify_rustc_probe(_fixture: &Fixture) -> Result<()> {
    Ok(())
}

pub(super) fn fixture_target<'a>(
    targets: &'a [Target],
    package: &str,
    name: &str,
) -> Result<&'a Target> {
    targets
        .iter()
        .find(|target| target.package == package && target.name == name)
        .with_context(|| format!("fixture target {package}:{name} is missing"))
}

pub(super) const CARGO_FIXTURE_FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        r#"[workspace]
members = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v"]
resolver = "2"
"#,
    ),
    (
        "a/Cargo.toml",
        r#"[package]
name = "a"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib"]

[features]
build_only = []
normal = []

[[bin]]
name = "build_smoke"
path = "src/build.rs"
required-features = ["build_only"]

[[bin]]
name = "normal_smoke"
path = "src/normal.rs"
required-features = ["normal"]
"#,
    ),
    (
        "a/src/lib.rs",
        "pub fn value() -> bool { true }\npub fn normal_is_enabled() -> bool { cfg!(feature = \"normal\") }\n",
    ),
    ("a/src/build.rs", "fn main() {}\n"),
    ("a/src/normal.rs", "fn main() {}\n"),
    (
        "b/Cargo.toml",
        r#"[package]
name = "b"
version = "0.1.0"
edition = "2024"

[build-dependencies]
a = { path = "../a", features = ["build_only"] }
e = { path = "../e", features = ["host_only"] }
"#,
    ),
    ("b/src/lib.rs", "pub fn value() -> bool { true }\n"),
    ("b/build.rs", "fn main() { assert!(a::value()); }\n"),
    (
        "c/Cargo.toml",
        r#"[package]
name = "c"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a", features = ["normal"] }
"#,
    ),
    ("c/src/lib.rs", "pub fn value() -> bool { a::value() }\n"),
    (
        "c/build.rs",
        "fn main() { std::thread::sleep(std::time::Duration::from_millis(250)); let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "d/Cargo.toml",
        r#"[package]
name = "d"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
u = { path = "../u", features = ["proc_dependency"] }
v = { path = "../v" }

[features]
default = ["root"]
host_only = ["v/host_dependency"]
root = []

[[test]]
name = "proc_macro_host_smoke"
path = "tests/host.rs"
required-features = ["host_only"]

[[test]]
name = "proc_macro_root_smoke"
path = "tests/root.rs"
required-features = ["root"]
"#,
    ),
    (
        "d/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"proc-macro-native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "d/src/lib.rs",
        "extern crate proc_macro;\n#[proc_macro]\npub fn marker(_input: proc_macro::TokenStream) -> proc_macro::TokenStream { proc_macro::TokenStream::new() }\n",
    ),
    ("d/tests/host.rs", "#[test]\nfn host_only() {}\n"),
    ("d/tests/root.rs", "#[test]\nfn root() {}\n"),
    (
        "e/Cargo.toml",
        r#"[package]
name = "e"
version = "0.1.0"
edition = "2024"
links = "fixture-e"

[dependencies]
c = { path = "../c" }
f = { path = "../f" }

[build-dependencies]
r = { path = "../r" }

[features]
host_only = []
"#,
    ),
    (
        "e/src/lib.rs",
        concat!(
            "#[test]\nfn ",
            "smoke_dependency() { assert!(f::normal_is_enabled()); assert_eq!(std::env::var(\"CARGO_PKG_NAME\").as_deref(), Ok(\"e\")); assert_eq!(std::env::var(\"CARGO_PKG_VERSION\").as_deref(), Ok(\"0.1.0\")); assert_eq!(std::env::var(\"CARGO_PKG_DESCRIPTION\").as_deref(), Ok(\"\")); assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"inherited\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_DIR\").unwrap()).ends_with(\"e\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_PATH\").unwrap()).ends_with(std::path::Path::new(\"e/Cargo.toml\"))); assert_eq!(std::env::var(\"TCL_LSP_BUILD_ENV\").as_deref(), Ok(\"owned\")); let out_dir = std::env::var(\"OUT_DIR\").unwrap(); assert!(out_dir.replace(char::from(92), \"/\").contains(\"/build/e\")); let variable = if cfg!(windows) { \"PATH\" } else if cfg!(target_os = \"macos\") { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let paths = std::env::split_paths(&std::env::var_os(variable).unwrap()).collect::<Vec<_>>(); assert!(paths.iter().any(|path| path == &std::path::Path::new(&out_dir).join(\"native\"))); assert!(paths.iter().any(|path| path.ends_with(\"shared-native\"))); assert!(!paths.iter().any(|path| path.ends_with(\"outside-native\"))); assert!(paths.iter().any(|path| { let path = path.to_string_lossy().replace(char::from(92), \"/\"); path.contains(\"/build/f\") && path.ends_with(\"/out/native\") })); assert!(paths.iter().any(|path| { let path = path.to_string_lossy().replace(char::from(92), \"/\"); path.contains(\"/build/r\") && path.ends_with(\"/out/build-only-native\") })); assert!(paths.iter().any(|path| path == std::path::Path::new(&std::env::var(\"TCL_LSP_EXPECTED_SYSROOT_LIB\").unwrap()))); if let Some(record) = std::env::var_os(\"TCL_LSP_RECORD_CARGO_PATH\") { let text = paths.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>().join(\"\\n\"); std::fs::write(record, text).unwrap(); } if !cfg!(windows) && !cfg!(target_os = \"macos\") { assert_eq!(std::env::var(\"TCL_LSP_SMOKE_RUNNER\").as_deref(), Ok(\"used\")); } }\n\n",
            "#[test]\nfn long_smoke_dependency() { panic!(\"deep test must not run\"); }\n\n",
            "#[test]\nfn deep() { panic!(\"deep test must not run\"); }\n",
        ),
    ),
    (
        "e/build.rs",
        "fn main() { let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let native = out.join(\"native\"); let later = out.join(\"aaa-native\"); let shared = out.parent().unwrap().parent().unwrap().parent().unwrap().join(\"shared-native\"); let escaped = out.ancestors().find(|path| path.file_name().is_some_and(|name| name == \"target\")).unwrap().join(\"../outside-native\"); std::fs::create_dir_all(&native).unwrap(); std::fs::create_dir_all(&later).unwrap(); std::fs::create_dir_all(&shared).unwrap(); std::fs::create_dir_all(&escaped).unwrap(); let context = if std::env::var_os(\"CARGO_FEATURE_HOST_ONLY\").is_some() { \"host\" } else { \"owned\" }; println!(\"cargo::rustc-env=TCL_LSP_BUILD_ENV={context}\"); println!(\"cargo::rustc-env=CARGO_PKG_NAME=overridden\"); println!(\"cargo::rustc-env=CARGO_PKG_VERSION=9.9.9\"); println!(\"cargo::rustc-env=CARGO_MANIFEST_DIR=overridden\"); println!(\"cargo::rustc-env=CARGO_MANIFEST_PATH=overridden\"); println!(\"cargo::rustc-link-search=native={}\", native.display()); println!(\"cargo::rustc-link-search=native={}\", later.display()); println!(\"cargo::rustc-link-search=native={}\", shared.display()); println!(\"cargo::rustc-link-search=native={}\", escaped.display()); }\n",
    ),
    (
        "f/Cargo.toml",
        r#"[package]
name = "f"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a" }
"#,
    ),
    (
        "f/src/lib.rs",
        "pub fn normal_is_enabled() -> bool { a::normal_is_enabled() }\n",
    ),
    (
        "f/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "g/Cargo.toml",
        r#"[package]
name = "g"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
"#,
    ),
    ("g/src/lib.rs", "pub fn value() -> bool { true }\n"),
    (
        "h/Cargo.toml",
        r#"[package]
name = "h"
version = "0.1.0"
edition = "2024"

[lib]
name = "library_smoke"

[[bin]]
name = "library-helper"
path = "src/main.rs"
"#,
    ),
    (
        "h/src/lib.rs",
        r#"#[test]
fn ordinary_library_test() {
    assert!(std::env::current_dir().unwrap().ends_with("h"));
    assert!(std::env::var_os("CARGO_BIN_EXE_library-helper").is_none());
}
"#,
    ),
    ("h/src/main.rs", "fn main() {}\n"),
    ("h/tests/deep.rs", "#[test]\nfn ordinary() {}\n"),
    (
        "i/Cargo.toml",
        r#"[package]
name = "i"
version = "0.1.0"
edition = "2024"

[[example]]
name = "demo_smoke"
path = "examples/demo.rs"
test = true
"#,
    ),
    ("i/src/lib.rs", ""),
    (
        "i/examples/demo.rs",
        r#"#[test]
fn ordinary_example_test() {
    assert!(std::env::current_dir().unwrap().ends_with("i"));
}
fn main() {}
"#,
    ),
    (
        "j/Cargo.toml",
        r#"[package]
name = "j"
version = "0.1.0"
edition = "2024"

[[example]]
name = "demo_smoke"
path = "examples/demo.rs"
test = false

[[bench]]
name = "bench_smoke"
path = "benches/bench.rs"
test = false
"#,
    ),
    ("j/src/lib.rs", ""),
    (
        "j/examples/demo.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false example must not compile as a test\");\nfn main() {}\n",
    ),
    (
        "j/benches/bench.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false benchmark must not compile as a test\");\nfn main() {}\n",
    ),
    (
        "k/Cargo.toml",
        r#"[package]
name = "k"
version = "0.1.0"
edition = "2024"

[[bench]]
name = "bench_smoke"
path = "benches/bench.rs"
"#,
    ),
    ("k/src/lib.rs", ""),
    (
        "k/benches/bench.rs",
        "#[test]\nfn ordinary_bench_test() {}\n",
    ),
    (
        "l/Cargo.toml",
        r#"[package]
name = "l"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "helper-tool"
path = "src/main.rs"
"#,
    ),
    ("l/src/main.rs", "fn main() {}\n"),
    (
        "l/tests/smoke.rs",
        "#[test]\nfn smoke_binary_path() { assert!(std::path::Path::new(&std::env::var(\"CARGO_BIN_EXE_helper-tool\").unwrap()).is_file()); assert_eq!(std::env::var(\"CARGO_BIN_EXE_unrelated\").as_deref(), Ok(\"inherited\")); assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"inherited\")); }\n",
    ),
    (
        "m/Cargo.toml",
        r#"[package]
name = "m"
version = "0.1.0"
edition = "2024"

[[bench]]
name = "unique_smoke"
path = "benches/bench.rs"
"#,
    ),
    ("m/src/lib.rs", ""),
    (
        "m/benches/bench.rs",
        "#[test]\nfn ordinary_bench_test() {}\n",
    ),
    (
        "n/Cargo.toml",
        r#"[package]
name = "n"
version = "0.1.0"
edition = "2024"

[lib]
test = false
"#,
    ),
    (
        "n/src/lib.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false library must not compile as a test\");\n",
    ),
    (
        "o/Cargo.toml",
        r#"[package]
name = "o"
version = "0.1.0"
edition = "2024"

[lib]
name = "common"
path = "../shared.rs"
"#,
    ),
    (
        "p/Cargo.toml",
        r#"[package]
name = "p"
version = "0.1.0"
edition = "2024"

[lib]
name = "common"
path = "../shared.rs"
"#,
    ),
    ("shared.rs", "#[test]\nfn smoke_shared_source() {}\n"),
    (
        "q/Cargo.toml",
        r#"[package]
name = "q"
version = "0.1.0"
edition = "2024"
"#,
    ),
    (
        "q/src/lib.rs",
        "#[test]\nfn smoke_dynamic_override() { assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"emitted\")); let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let variable = if cfg!(windows) { \"PATH\" } else if cfg!(target_os = \"macos\") { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let paths = std::env::split_paths(&std::env::var_os(variable).unwrap()).collect::<Vec<_>>(); assert_eq!(paths.first(), Some(&out.join(\"custom\"))); assert!(!paths.contains(&out.join(\"linked\"))); if let Some(record) = std::env::var_os(\"TCL_LSP_RECORD_OVERRIDE_PATH\") { let text = paths.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>().join(\"\\n\"); std::fs::write(record, text).unwrap(); } }\n",
    ),
    (
        "q/build.rs",
        "fn main() { let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let custom = out.join(\"custom\"); let linked = out.join(\"linked\"); std::fs::create_dir_all(&custom).unwrap(); std::fs::create_dir_all(&linked).unwrap(); let target = std::env::var(\"CARGO_CFG_TARGET_OS\").unwrap(); let variable = if target == \"windows\" { \"PATH\" } else if target == \"macos\" { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let mut paths = vec![custom]; if let Some(existing) = std::env::var_os(variable) { paths.extend(std::env::split_paths(&existing)); } let override_value = std::env::join_paths(paths).unwrap(); println!(\"cargo::rustc-link-search=native={}\", linked.display()); println!(\"cargo::rustc-env={variable}={}\", override_value.to_string_lossy()); println!(\"cargo::rustc-env=CARGO_MANIFEST_LINKS=emitted\"); }\n",
    ),
    (
        "r/Cargo.toml",
        r#"[package]
name = "r"
version = "0.1.0"
edition = "2024"
"#,
    ),
    ("r/src/lib.rs", "pub fn value() -> bool { true }\n"),
    (
        "r/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"build-only-native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "s/Cargo.toml",
        r#"[package]
name = "s"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true
"#,
    ),
    (
        "s/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"unrelated-proc-macro-native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    ("s/src/lib.rs", "extern crate proc_macro;\n"),
    (
        "t/Cargo.toml",
        r#"[package]
name = "t"
version = "0.1.0"
edition = "2024"

[dependencies]
d = { path = "../d", features = ["host_only"] }
"#,
    ),
    (
        "t/src/lib.rs",
        r#"d::marker!();

#[test]
fn smoke_proc_macro_link_path() {
    let variable = if cfg!(windows) {
        "PATH"
    } else if cfg!(target_os = "macos") {
        "DYLD_FALLBACK_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    };
    let paths = std::env::split_paths(&std::env::var_os(variable).unwrap()).collect::<Vec<_>>();
    assert!(paths.iter().any(|path| {
        let path = path.to_string_lossy().replace(char::from(92), "/");
        path.contains("/build/d") && path.ends_with("/out/proc-macro-native")
    }));
    if let Some(record) = std::env::var_os("TCL_LSP_RECORD_PROC_MACRO_PATH") {
        let text = paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(record, text).unwrap();
    }
}
"#,
    ),
    (
        "u/Cargo.toml",
        r#"[package]
name = "u"
version = "0.1.0"
edition = "2024"

[features]
proc_dependency = []

[[bin]]
name = "proc_dependency_smoke"
path = "src/main.rs"
required-features = ["proc_dependency"]
"#,
    ),
    (
        "u/src/main.rs",
        "#[cfg(not(feature = \"proc_dependency\"))]\ncompile_error!(\"procedural-macro dependency feature is missing\");\nfn main() {}\n",
    ),
    (
        "v/Cargo.toml",
        r#"[package]
name = "v"
version = "0.1.0"
edition = "2024"

[features]
host_dependency = []

[[bin]]
name = "host_dependency_smoke"
path = "src/main.rs"
required-features = ["host_dependency"]
"#,
    ),
    (
        "v/src/main.rs",
        "compile_error!(\"host-unit-only dependency feature reached a workspace-root target\");\nfn main() {}\n",
    ),
];
