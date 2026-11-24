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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;
use tcl_dialect::TclVersion;
use tcl_dialect::model::{Family, SpecProvider};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;
use tcl_syntax::list::list_element;
use tcl_test_support::{reference_patchlevel, require_jimsh, required_tclshs};

const PROBES: &str = "../../docs/work/name-handling/handover/oo-dispatch-probe/definitions";
const DUMMY: &str = "__definition_probe__";
const PACKAGES: &[&str] = &[
    "http",
    "msgcat",
    "tcltest",
    "platform",
    "platform::shell",
    "opt",
];
const TCLLIB_PACKAGES: &[&str] = &["oo::util", "struct::list"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Tcl,
    Tk,
    Jim,
    Tcllib,
}

impl Lane {
    fn key(self) -> &'static str {
        match self {
            Self::Tcl => "tcl",
            Self::Tk => "tk",
            Self::Jim => "jim",
            Self::Tcllib => "tcllib",
        }
    }

    fn registry_member(self, provider: SpecProvider) -> bool {
        match provider {
            SpecProvider::Core(Family::Tcl) => self == Self::Tcl,
            SpecProvider::Core(Family::Jim) => self == Self::Jim,
            SpecProvider::Core(_) => false,
            SpecProvider::Package("Tk") => self == Self::Tk,
            SpecProvider::Package(name) => match self {
                Self::Tcl => PACKAGES.contains(&name),
                Self::Tcllib => TCLLIB_PACKAGES.contains(&name),
                Self::Tk | Self::Jim => false,
            },
        }
    }
}

fn roster(registry: &CommandRegistry, lane: Lane) -> BTreeSet<(&str, &str)> {
    registry
        .command_names()
        .flat_map(|name| registry.specs(name))
        .filter(|spec| {
            if lane == Lane::Tcllib
                && spec
                    .tcllib_package
                    .is_some_and(|p| TCLLIB_PACKAGES.contains(&p))
            {
                return true;
            }
            if lane == Lane::Tcl
                && (spec.required_package == Some("Tk") || spec.tcllib_package.is_some())
            {
                return false;
            }
            spec.surface
                .is_some_and(|rows| rows.iter().any(|row| lane.registry_member(row.provider)))
        })
        .flat_map(|spec| {
            let name = spec.name.trim_start_matches("::");
            std::iter::once((name, ""))
                .chain(spec.subcommands.iter().map(move |sub| (name, sub.name)))
        })
        .collect()
}

fn package_for<'a>(command: &'a str, lane: Lane, registry: &CommandRegistry) -> Option<&'a str> {
    if lane == Lane::Tk {
        return Some("Tk");
    }
    let declared = registry
        .specs(command)
        .iter()
        .filter_map(|spec| spec.required_package)
        .find(|&package| lane.registry_member(SpecProvider::Package(package)));
    declared.or_else(|| {
        let (namespace, _) = command.rsplit_once("::")?;
        match (lane, namespace) {
            (Lane::Tcl, name) if PACKAGES.contains(&name) => Some(name),
            (Lane::Tcllib, "oo::Helpers") => Some("oo::util"),
            (Lane::Tcllib, "struct::list") => Some("struct::list"),
            _ => None,
        }
    })
}

struct Observation {
    missing: bool,
    unknown_sub: bool,
    arity: bool,
    text: String,
}

fn analyse_call(
    analyser: &mut Analyser,
    dialect: &str,
    package: Option<&str>,
    command: &str,
    sub: &str,
    count: usize,
) -> Observation {
    let mut source = package.map_or_else(String::new, |name| format!("package require {name}\n"));
    let call_start = source.len();
    source.push_str(&list_element(command));
    if !sub.is_empty() {
        write!(source, " {}", list_element(sub)).unwrap();
    }
    let sub_end = source.len();
    for _ in 0..count {
        write!(source, " {DUMMY}").unwrap();
    }
    source.push('\n');
    let result = analyser.analyse(&source, dialect);
    let mut observation = Observation {
        missing: false,
        unknown_sub: false,
        arity: false,
        text: String::new(),
    };
    for diagnostic in result.diagnostics {
        let offset = diagnostic.span.as_range().start;
        if offset < call_start || offset >= source.len() {
            continue;
        }
        match diagnostic.code {
            DiagCode::W002 | DiagCode::W123 | DiagCode::W135 | DiagCode::W136 | DiagCode::W139
                if offset < sub_end =>
            {
                observation.missing = true;
            }
            DiagCode::W001 if offset < sub_end => observation.unknown_sub = true,
            DiagCode::E001
            | DiagCode::E002
            | DiagCode::E003
            | DiagCode::E004
            | DiagCode::E005
            | DiagCode::W149 => observation.arity = true,
            _ => continue,
        }
        write!(observation.text, " {}", diagnostic.message).unwrap();
    }
    observation
}

struct Matrix<'a> {
    root: &'a Path,
    observed: BTreeMap<String, String>,
    count: usize,
}

impl Matrix<'_> {
    fn compare(&mut self, lane: Lane, dialect: &str, patch: &str, rows: &str) {
        let registry = static_context_for(dialect).commands();
        let mut analyser = Analyser::new();
        let mut usage = String::from(
            "command\tsubcommand\targc\treference usage\tregistry synopsis\tanalyser\n",
        );
        let mut lines = rows.lines();
        assert_eq!(
            lines.next(),
            Some(format!("V\t{patch}").as_str()),
            "{}",
            lane.key()
        );
        assert_eq!(
            lines.next_back(),
            Some("END"),
            "incomplete {} {patch} probe",
            lane.key()
        );
        for line in lines {
            let fields: Vec<_> = line.splitn(6, '\t').collect();
            let (command, sub, count, reference, message) = match fields.as_slice() {
                ["C", command, present @ ("0" | "1")] => (*command, "", 0, *present, ""),
                ["P", command, sub, count, reference, message] => {
                    (*command, *sub, count.parse().unwrap(), *reference, *message)
                }
                _ => panic!("invalid {} {patch} probe row {line:?}", lane.key()),
            };
            let package = package_for(command, lane, registry);
            let actual = analyse_call(&mut analyser, dialect, package, command, sub, count);
            self.count += 1;
            if reference == "arity" {
                let synopsis = registry
                    .get(command)
                    .and_then(|spec| {
                        if sub.is_empty() {
                            spec.primary_synopsis(
                                registry
                                    .source_descriptor_availability(
                                        spec,
                                        registry.own_surface_query(),
                                    )
                                    .package_version,
                            )
                        } else {
                            spec.subcommands
                                .iter()
                                .find(|row| row.name == sub)
                                .and_then(tcl_registry::SubCommand::primary_synopsis)
                        }
                    })
                    .unwrap_or_default()
                    .replace(['\t', '\r', '\n'], " ");
                writeln!(
                    usage,
                    "{command}\t{sub}\t{count}\t{message}\t{synopsis}\t{}",
                    actual.text.replace(['\t', '\r', '\n'], " ")
                )
                .unwrap();
            }
            let kind = match reference {
                "1" if actual.missing => "false-unknown",
                "0" if !actual.missing => "claimed-nonexistent",
                "ok" if actual.arity && !actual.missing && !actual.unknown_sub => "false-arity",
                "arity" if !actual.arity && !actual.missing && !actual.unknown_sub => {
                    "missed-arity"
                }
                "subcommand" if !actual.unknown_sub && !actual.missing => "claimed-subcommand",
                "ok" | "arity" if !sub.is_empty() && actual.unknown_sub => {
                    "false-unknown-subcommand"
                }
                "0" | "1" | "ok" | "arity" | "subcommand" | "value" => continue,
                _ => panic!("unknown oracle verdict {reference:?}"),
            };
            let key = format!("{}\t{patch}\t{command}\t{sub}\t{count}\t{kind}", lane.key());
            self.observed.insert(
                key,
                format!("oracle {message:?}; analyser {:?}", actual.text),
            );
        }
        let directory = self.root.join(format!("{}-{patch}", lane.key()));
        fs::write(directory.join("usage.tsv"), usage).unwrap();
        let mut report = String::new();
        for (key, detail) in &self.observed {
            writeln!(report, "{key}\t{detail}").unwrap();
        }
        fs::write(directory.join("differences.tsv"), report).unwrap();
    }

    fn run(
        &mut self,
        lane: Lane,
        dialect: &str,
        patch: &str,
        interpreter: &Path,
        tk_library: &Path,
        modules: &Path,
    ) {
        let id = format!("{}-{patch}", lane.key());
        let directory = self.root.join(&id);
        fs::create_dir_all(&directory).unwrap();
        let registry = static_context_for(dialect).commands();
        let mut entries = String::new();
        for (name, sub) in roster(registry, lane) {
            writeln!(entries, "{name}\t{sub}").unwrap();
        }
        fs::write(directory.join("entries.tsv"), entries).unwrap();
        let probe = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(PROBES)
            .join("definition_probe.tcl");
        let mut command = Command::new("timeout");
        command.arg("120");
        if lane == Lane::Tk {
            command.args(["xvfb-run", "-a"]);
            let library = tk_library.parent().unwrap().join("../library");
            command.env("TK_LIBRARY", library);
        }
        let output = command
            .arg(interpreter)
            .arg(probe)
            .args(["entries.tsv", "oracle.tsv", lane.key()])
            .arg(tk_library)
            .arg(modules)
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(PROBES)
                    .join("confirmed.tsv"),
            )
            .arg(dialect.strip_prefix("tcl").unwrap_or(dialect))
            .env("LC_ALL", "C.UTF-8")
            .env_remove("TCL_LIBRARY")
            .current_dir(&directory)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        assert!(
            output.status.success(),
            "{id}: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let rows = fs::read_to_string(directory.join("oracle.tsv")).unwrap();
        self.compare(lane, dialect, patch, &rows);
        println!("{id}: {} calls", self.count);
    }
}

fn required_path(key: &str) -> PathBuf {
    let path = std::env::var_os(key).map_or_else(|| panic!("{key} is required"), PathBuf::from);
    path.canonicalize()
        .unwrap_or_else(|error| panic!("{key}={}: {error}", path.display()))
}

struct Reference {
    lane: Lane,
    dialect: &'static str,
    patch: String,
    interpreter: PathBuf,
    tk: PathBuf,
    modules: PathBuf,
}

impl Reference {
    fn directory(&self, root: &Path) -> PathBuf {
        root.join(format!("{}-{}", self.lane.key(), self.patch))
    }

    fn check(&self, root: &Path) {
        let mut matrix = Matrix {
            root,
            observed: BTreeMap::new(),
            count: 0,
        };
        matrix.run(
            self.lane,
            self.dialect,
            &self.patch,
            &self.interpreter,
            &self.tk,
            &self.modules,
        );
        fs::write(self.directory(root).join("count"), matrix.count.to_string()).unwrap();
    }
}

fn definition_references() -> Vec<Reference> {
    let mut jobs = Vec::new();
    for reference in required_tclshs(&TclVersion::ALL).unwrap() {
        let version = reference.version;
        let patch = reference_patchlevel(version);
        let dialect = version.dialect_profile_name();
        let tk = required_path(&format!(
            "TCL_LSP_TK{}",
            version.version_string().replace('.', "")
        ));
        jobs.push(Reference {
            lane: Lane::Tcl,
            dialect,
            patch: patch.to_owned(),
            interpreter: reference.path.clone(),
            tk: PathBuf::new(),
            modules: PathBuf::new(),
        });
        jobs.push(Reference {
            lane: Lane::Tk,
            dialect,
            patch: patch.to_owned(),
            interpreter: reference.path.clone(),
            tk,
            modules: PathBuf::new(),
        });
        if version == TclVersion::V8_6 {
            jobs.push(Reference {
                lane: Lane::Tcllib,
                dialect,
                patch: patch.to_owned(),
                interpreter: reference.path,
                tk: PathBuf::new(),
                modules: required_path("TCL_LSP_TCLLIB_MODULES"),
            });
        }
    }
    let jim = require_jimsh().unwrap();
    jobs.push(Reference {
        lane: Lane::Jim,
        dialect: "jim",
        patch: jim.patchlevel,
        interpreter: jim.path,
        tk: PathBuf::new(),
        modules: PathBuf::new(),
    });
    jobs
}

const WORKER: &str = "TCL_LSP_DEFINITIONS_WORKER";

#[test]
fn command_definition_matrix() {
    let Some(root) = std::env::var_os("TCL_LSP_DEFINITIONS_WORK") else {
        return; // The complete matrix is run by `cargo xtask definitions-gate`.
    };
    let root = PathBuf::from(root);
    assert!(root.is_absolute());
    let jobs = definition_references();
    if let Ok(index) = std::env::var(WORKER) {
        jobs[index.parse::<usize>().unwrap()].check(&root);
        return;
    }

    // Each process owns its caches. Parallel analysis in one process
    // serialises on the environment cache and obscures lane failures.
    let executable = std::env::current_exe().unwrap();
    let workers: Vec<_> = (0..jobs.len())
        .map(|index| {
            Command::new(&executable)
                .args(["command_definition_matrix", "--exact", "--nocapture"])
                .env(WORKER, index.to_string())
                .spawn()
                .unwrap()
        })
        .collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|mut worker| worker.wait().unwrap())
        .collect();
    assert!(
        outcomes.iter().all(std::process::ExitStatus::success),
        "{outcomes:?}"
    );
    check_definition_reports(&root, &jobs);
}

fn check_definition_reports(root: &Path, jobs: &[Reference]) {
    let count: usize = jobs
        .iter()
        .map(|job| {
            fs::read_to_string(job.directory(root).join("count"))
                .unwrap()
                .parse::<usize>()
                .unwrap()
        })
        .sum();
    assert!(count > 10_000, "only {count} calls analysed");
    let report: String = jobs
        .iter()
        .map(|job| fs::read_to_string(job.directory(root).join("differences.tsv")).unwrap())
        .collect();
    fs::write(root.join("differences.tsv"), &report).unwrap();

    let allowed: BTreeSet<_> = include_str!(
        "../../../docs/work/name-handling/handover/oo-dispatch-probe/definitions/known-differences.tsv"
    )
    .lines()
    .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    .flat_map(|line| {
        let fields: Vec<_> = line.splitn(7, '\t').collect();
        let &[lane, patch, command, sub, counts, kind, reason] = fields.as_slice() else {
            panic!("invalid allow-list row {line:?}");
        };
        assert!(!reason.trim().is_empty(), "{line}");
        counts.split(',').map(move |count| {
            let count: usize = count.parse().unwrap();
            format!("{lane}\t{patch}\t{command}\t{sub}\t{count}\t{kind}")
        })
    })
    .collect();
    let regressions: Vec<_> = report
        .lines()
        .map(|line| line.rsplit_once('\t').unwrap().0)
        .filter(|key| !allowed.contains(*key))
        .collect();
    assert!(
        regressions.is_empty(),
        "{} new differences across {count} analysed calls. See {}\n{:#?}",
        regressions.len(),
        root.join("differences.tsv").display(),
        &regressions[..regressions.len().min(30)]
    );
    println!("Analysed {count} independent calls across Tcl, Tk, Jim and Tcllib");
}

#[test]
fn confirmed_command_definitions() {
    const CASES: &str = include_str!(
        "../../../docs/work/name-handling/handover/oo-dispatch-probe/definitions/confirmed.tsv"
    );
    let mut failures = Vec::new();
    for line in CASES.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.splitn(4, '\t').collect();
        let [first, last, expected, call] = fields.as_slice() else {
            panic!("invalid confirmation row {line:?}");
        };
        let source = call.replace("\\n", "\n");
        for version in TclVersion::ALL {
            if !(first..=last).contains(&&version.version_string()) {
                continue;
            }
            let analysis = Analyser::new().analyse(&source, version.dialect_profile_name());
            let verdicts: BTreeSet<_> = analysis
                .diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic_verdict(diagnostic.code))
                .collect();
            let passed = match *expected {
                "ok" => verdicts.is_empty(),
                "unavailable" => {
                    let head = source
                        .lines()
                        .last()
                        .unwrap()
                        .split_whitespace()
                        .next()
                        .unwrap();
                    let context = static_context_for(version.dialect_profile_name());
                    context
                        .context()
                        .resolve_spec(context.commands(), head)
                        .is_none()
                }
                expected => verdicts.contains(expected),
            };
            if !passed {
                let diagnostics: Vec<_> = analysis
                    .diagnostics
                    .iter()
                    .map(|diagnostic| (diagnostic.code, diagnostic.message.as_str()))
                    .collect();
                failures.push(format!(
                    "{version:?} {source:?}: expected {expected}, got {verdicts:?} {diagnostics:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn diagnostic_verdict(code: DiagCode) -> Option<&'static str> {
    match code {
        DiagCode::W002 | DiagCode::W123 | DiagCode::W135 | DiagCode::W136 | DiagCode::W139 => {
            Some("unknown")
        }
        DiagCode::W001 => Some("subcommand"),
        DiagCode::W004 => Some("option"),
        DiagCode::W146 => Some("value"),
        DiagCode::E001
        | DiagCode::E002
        | DiagCode::E003
        | DiagCode::E004
        | DiagCode::E005
        | DiagCode::W149 => Some("arity"),
        _ => None,
    }
}
