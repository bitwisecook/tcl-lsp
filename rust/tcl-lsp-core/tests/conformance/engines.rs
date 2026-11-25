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

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use regex::Regex;
use tcl_dialect::TclVersion;

const TIMEOUT: Duration = Duration::from_secs(60);
const OUTPUT_LIMIT: usize = 600;

/// One way of running a fixture file: an interpreter column of the row.
pub struct Interpreter {
    pub column: String,
    program: PathBuf,
    leading: Vec<String>,
    pub release: Option<TclVersion>,
}

/// The interpreters every row runs through. A missing one is a harness
/// error, so a row never silently drops a column.
pub struct Interpreters {
    pub all: Vec<Interpreter>,
}

fn required_path(variable: &str) -> PathBuf {
    let path = std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{variable} is required; run `cargo xtask conformance`"));
    assert!(path.is_file(), "{variable}={} is not a file", path.display());
    path
}

impl Interpreters {
    pub fn from_environment() -> Self {
        let mut all = Vec::new();
        for version in TclVersion::ALL {
            let tclsh = tcl_test_support::locate_tclsh(version)
                .unwrap_or_else(|error| panic!("{error}"))
                .unwrap_or_else(|| panic!("C Tcl {} is required", version.version_string()));
            all.push(Interpreter {
                column: format!("tclsh{}", version.version_string()),
                program: tclsh.path,
                leading: Vec::new(),
                release: Some(version),
            });
        }
        let jimsh = tcl_test_support::require_jimsh().unwrap_or_else(|error| panic!("{error}"));
        all.push(Interpreter {
            column: "jim".to_owned(),
            program: jimsh.path,
            leading: Vec::new(),
            release: None,
        });
        let vm = required_path("TCL_LSP_TCLVM");
        let runtime = required_path("TCL_LSP_RUN_SCRIPT");
        for version in TclVersion::ALL {
            let release = version.version_string();
            all.push(Interpreter {
                column: format!("vm{release}"),
                program: vm.clone(),
                leading: vec!["--tcl-version".to_owned(), release.to_owned()],
                release: Some(version),
            });
        }
        for version in TclVersion::ALL {
            let release = version.version_string();
            all.push(Interpreter {
                column: format!("rt{release}"),
                program: runtime.clone(),
                leading: vec![
                    "--quiet".to_owned(),
                    "--tcl-version".to_owned(),
                    release.to_owned(),
                ],
                release: Some(version),
            });
        }
        Self { all }
    }

    pub fn tclsh(&self, version: TclVersion) -> &Interpreter {
        self.all
            .iter()
            .find(|interpreter| {
                interpreter.release == Some(version) && interpreter.column.starts_with("tclsh")
            })
            .expect("every release has a C Tcl column")
    }
}

/// What one interpreter did with one file.
#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    pub exit: String,
    pub output: String,
}

impl Interpreter {
    /// Run `file` from its own directory, so relative `source` and
    /// `info script` results do not depend on the checkout location.
    pub fn run(&self, file: &Path, root: &Path) -> Outcome {
        let directory = file.parent().expect("fixture has a directory");
        let name = file.file_name().expect("fixture has a name");
        let mut child = Command::new(&self.program)
            .args(&self.leading)
            .arg(name)
            .current_dir(directory)
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{}: {error}", self.program.display()));
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let out = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            bytes
        });
        let err = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        });
        let started = Instant::now();
        let exit = loop {
            if let Some(status) = child.try_wait().expect("child status") {
                break status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string());
            }
            if started.elapsed() > TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                break "timeout".to_owned();
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let mut bytes = out.join().expect("stdout reader");
        bytes.extend(err.join().expect("stderr reader"));
        Outcome {
            exit,
            output: normalise(&String::from_utf8_lossy(&bytes), directory, root),
        }
    }
}

static VOLATILE: LazyLock<[(Regex, &str); 5]> = LazyLock::new(|| {
    [
        (Regex::new(r"thread '([^']*)' \(\d+\)").unwrap(), "thread '$1' (#)"),
        (Regex::new(r"::oo::Obj\d+").unwrap(), "::oo::Obj#"),
        (Regex::new(r"0x[0-9a-fA-F]{6,}").unwrap(), "0x#"),
        (
            Regex::new(r"\b(file|sock|pipe|rc|zlib|refchan)[0-9a-f]{1,16}\b").unwrap(),
            "$1#",
        ),
        (Regex::new(r"/tmp/[^\s\x22']+").unwrap(), "<tmp>"),
    ]
});

/// Remove what changes between runs or checkouts, then join lines with
/// ` ¶ ` so each outcome stays on one TSV line.
fn normalise(text: &str, directory: &Path, root: &Path) -> String {
    let mut text = text
        .replace(&directory.display().to_string(), "<fixture-dir>")
        .replace(&root.display().to_string(), "<root>");
    for (pattern, replacement) in VOLATILE.iter() {
        text = pattern.replace_all(&text, *replacement).into_owned();
    }
    let mut joined = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join(" ¶ ")
        .replace('\t', " ");
    if joined.len() > OUTPUT_LIMIT {
        let mut end = OUTPUT_LIMIT;
        while !joined.is_char_boundary(end) {
            end -= 1;
        }
        joined.truncate(end);
        joined.push('…');
    }
    joined
}
