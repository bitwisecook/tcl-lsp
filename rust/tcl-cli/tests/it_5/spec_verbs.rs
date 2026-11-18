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

//! Integration tests for the `spec` verb group.
//!
//! These drive the built `tcl` binary against fixtures built in a temporary
//! directory at test time — a two-release fake package, one release as a
//! directory and one as a `.zip`. **No test here touches the network**: the
//! `--github` mode's units (tag mapping, glob filtering, URL construction) are
//! tested in `src/commands/spec.rs`, and everything downstream of "sources in
//! hand" is exercised through `--snapshot`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

mod export;
mod import_release_history;
mod import_scan_and_probe;
mod test_against_package;
mod upgrade;

/// `Command::output` through a reused `tcl` process (`tcl_cli_support::worker`):
/// a fresh process spends seconds loading packs before it reads its argv, and
/// this file asserts on many invocations.
trait Reused {
    fn reused_output(&mut self) -> std::io::Result<std::process::Output>;
}

impl Reused for Command {
    fn reused_output(&mut self) -> std::io::Result<std::process::Output> {
        tcl_cli_support::worker::output(self, env!("CARGO_TARGET_TMPDIR"))
    }
}

/// A temporary directory removed when the guard drops.
struct Tree(PathBuf);

impl Tree {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-spec-verbs-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// One release as an unpacked directory.
    fn release_dir(&self, version: &str, text: &str) -> PathBuf {
        let dir = self.0.join(version);
        std::fs::create_dir_all(&dir).expect("release dir");
        std::fs::write(dir.join("demo.tcl"), text).expect("write source");
        // A non-Tcl file that must be ignored, and a dot-directory that must
        // not be walked.
        std::fs::write(dir.join("README.md"), "# demo\n").expect("write readme");
        dir
    }

    /// One release as a `.zip`, with the sources under a top-level directory
    /// the way a real release archive ships them.
    fn release_zip(&self, version: &str, text: &str) -> PathBuf {
        let path = self.0.join(format!("demo-{version}.zip"));
        let file = std::fs::File::create(&path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file(format!("demo-{version}/demo.tcl"), options)
            .expect("zip entry");
        zip.write_all(text.as_bytes()).expect("zip write");
        zip.finish().expect("finish zip");
        path
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Run `tcl spec import …`, returning `(stdout, stderr, exit code)`.
fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The body of one `command NAME { … }` block of a rendered pack.
fn command_block<'a>(pack: &'a str, name: &str) -> &'a str {
    let start = pack
        .find(&format!("command {name} {{"))
        .unwrap_or_else(|| panic!("no `command {name}` in:\n{pack}"));
    let rest = &pack[start..];
    let end = rest.find("\n}\n").unwrap_or(rest.len());
    &rest[..end]
}

/// Where a pack sits relative to the project `tcl spec test` is run in.
#[derive(Clone, Copy)]
enum Layout {
    /// In the project directory, which has no manifest.
    Flat,
    /// In a `specs` directory below the project's manifest.
    Nested,
    /// In a dependency vendored into the project, which has a manifest and a
    /// policy of its own.
    Vendored,
}

/// A Tcl package on disk, a pack that describes it and the project policy that
/// opts the package in, for `tcl spec test`.
struct Described {
    tree: Tree,
    /// The operator's project, which the verb is run in.
    project: PathBuf,
    pack: PathBuf,
    library: PathBuf,
}

impl Described {
    /// `package` is the Tcl source of the `demo` package, `pack` the pack that
    /// describes it, and `trusted` the packages the project's policy opts in.
    fn new(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Flat)
    }

    /// The same, with the pack in a `specs` directory below a project that has a
    /// manifest and holds the policy: the project is not the pack's directory.
    fn nested(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Nested)
    }

    /// The same, with the pack in a dependency vendored into the project. `trusted`
    /// is the dependency's own policy, and the project's says nothing until
    /// [`Self::trust_in_the_project`].
    fn vendored(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Vendored)
    }

    /// Write `tclpkg.toml` in `dir`, opting the named packages in.
    fn write_policy(dir: &Path, trusted: &[&str]) {
        let names: Vec<String> = trusted.iter().map(|name| format!("{name:?}")).collect();
        std::fs::write(
            dir.join("tclpkg.toml"),
            format!(
                "[build]\nallow-build-scripts = true\ntrusted = [{}]\n",
                names.join(", ")
            ),
        )
        .expect("write the policy");
    }

    /// The operator's own policy opts the named packages in.
    fn trust_in_the_project(&self, trusted: &[&str]) {
        Self::write_policy(&self.project, trusted);
    }

    /// The directory of the dependency [`Self::vendored`] vendors into the project.
    fn dependency(&self) -> PathBuf {
        self.project.join("vendor").join("demo")
    }

    fn build(tag: &str, package: &str, pack: &str, trusted: &[&str], layout: Layout) -> Self {
        let tree = Tree::new(tag);
        let library = tree.path().join("lib");
        let package_dir = library.join("demo");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("pkgIndex.tcl"),
            "package ifneeded demo 1.0 [list source [file join $dir demo.tcl]]\n",
        )
        .expect("write the index");
        std::fs::write(package_dir.join("demo.tcl"), package).expect("write the package");
        let project_dir = tree.path().join("project");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        let (pack_dir, policy_dir) = match layout {
            Layout::Flat => (project_dir.clone(), project_dir.clone()),
            Layout::Nested => {
                std::fs::write(project_dir.join("tclpkg.tcl"), "package demo 1.0\n")
                    .expect("write the manifest");
                (project_dir.join("specs"), project_dir.clone())
            }
            Layout::Vendored => {
                std::fs::write(project_dir.join("tclpkg.tcl"), "package app 1.0\n")
                    .expect("write the project's manifest");
                let dependency = project_dir.join("vendor").join("demo");
                std::fs::create_dir_all(&dependency).expect("dependency dir");
                std::fs::write(dependency.join("tclpkg.tcl"), "package demo 1.0\n")
                    .expect("write the dependency's manifest");
                (dependency.join("specs"), dependency)
            }
        };
        std::fs::create_dir_all(&pack_dir).expect("pack dir");
        let pack_path = pack_dir.join("demo.tclspec");
        std::fs::write(&pack_path, pack).expect("write the pack");
        if !trusted.is_empty() {
            Self::write_policy(&policy_dir, trusted);
        }
        Self {
            tree,
            project: project_dir,
            pack: pack_path,
            library,
        }
    }

    /// `tcl spec test` on the pack, with the package's directory named and every
    /// per-user directory pointed into the tree, and no shell named.
    fn command(&self) -> Command {
        self.command_in(&self.project)
    }

    /// The same, run from `dir`.
    fn command_in(&self, dir: &Path) -> Command {
        let mut command = self.environment_in(dir);
        command.args(["spec", "test", &self.pack.to_string_lossy()]);
        command
    }

    /// `tcl` run in the project, with the package's directory named and every
    /// per-user directory pointed into the tree, and no arguments.
    fn environment(&self) -> Command {
        self.environment_in(&self.project)
    }

    /// The same, run from `dir`.
    fn environment_in(&self, dir: &Path) -> Command {
        let home = self.tree.path().join("home");
        std::fs::create_dir_all(&home).expect("home");
        let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
        command
            .current_dir(dir)
            .env("TCLLIBPATH", &self.library)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_STATE_HOME", home.join("state"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env_remove("TCL_VENV");
        command
    }

    /// The same, with the shell named.
    fn run(&self, tclsh: &Path) -> (String, String, i32) {
        self.run_in(&self.project, tclsh)
    }

    /// The same, from `dir`.
    fn run_in(&self, dir: &Path, tclsh: &Path) -> (String, String, i32) {
        let mut command = self.command_in(dir);
        command.arg("--tclsh").arg(tclsh);
        finished(&mut command)
    }
}

/// Run `command` to its end: its standard output and error, and its status.
fn finished(command: &mut Command) -> (String, String, i32) {
    let output = command.output().expect("failed to spawn tcl binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The Tcl 8.6 reference `tclsh`, located as every oracle test locates it: a
/// missing one fails the test when `TCL_LSP_REQUIRE_TCLSH` requires it.
fn tclsh_on_path() -> Option<PathBuf> {
    tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V8_6).map(|tclsh| tclsh.path)
}

const DEMO_PACKAGE: &str = "package provide demo 1.0\n\
namespace eval demo {}\n\
# One to three words.\n\
proc demo::flex {a {b x} {c y}} {return \"$a$b$c\"}\n\
# Exactly two.\n\
proc demo::fixed {a b} {return \"$a$b\"}\n";

fn demo_pack(flex: &str, fixed: &str) -> String {
    format!(
        "speclib demo 2.0 {{\n    command demo::flex {{\n        arity {flex}\n        \
         required_package demo\n    }}\n    command demo::fixed {{\n        arity {fixed}\n        \
         required_package demo\n    }}\n}}\n"
    )
}
