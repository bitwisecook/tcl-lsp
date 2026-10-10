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

//! Describing the commands of a C Tcl extension from the sources that can
//! state them (`docs/design/compiler/registry-consumer-contracts.md` § *C Tcl
//! extensions*).
//!
//! Every command an extension registers starts at the conservative default
//! (`CommandSpec::extension_default`): nothing in a C source or a loaded
//! package says what a command does to state, so no source narrows it. What a
//! source *does* state is proposed beside it, with the evidence and the source
//! it came from — the arity a usage message states, the subcommands an option
//! table names, the package a `Tcl_PkgProvide` provides. Each row carries its
//! provenance, [`ExtensionSource::CScan`] (a mechanical scan of the C source)
//! or [`ExtensionSource::Probe`] (the commands a sandboxed `package require`
//! added to a real shell), or both, so a reviewer can tell what is read from
//! what is observed and overrule either.
//!
//! One description is one extension: the commands one entry point's library
//! registers. Sources that define several entry points (`Pkga_Init` in one
//! file, `Doors_Init` in another) are several extensions, and are described one
//! at a time, by naming the entry point ([`import_c_sources`]); each then has
//! the registrations its entry point reaches, by the functions it names and
//! those they name in turn.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use tcl_registry::CommandSpec;
use tcl_registry::arity::Arity;

use super::SourceFile;
use super::c_scan::{CBlindSpot, CDeclaredCommand, CName, CPackage, CScan, scan_c_source};
use crate::draft::{self, Draft};

/// Where the facts of a row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtensionSource {
    /// A mechanical scan of the extension's C source.
    CScan,
    /// A sandboxed `package require` in a real shell, and the commands it
    /// added.
    Probe,
}

impl ExtensionSource {
    /// The spelling a pack's evidence lines and the JSON carry.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CScan => "c-scan",
            Self::Probe => "probe",
        }
    }
}

/// What a sandboxed `package require` showed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProbeReport {
    /// The package that was required.
    pub package: String,
    /// The version it provided, when it said.
    pub version: Option<String>,
    /// The commands it added to the shell, qualified, in name order.
    pub commands: Vec<String>,
}

/// One command an extension registers: its draft, at the conservative
/// default, and the evidence for every proposal in it.
#[derive(Debug, Clone)]
pub struct ExtensionCommand {
    /// The command's name, as the extension spells it.
    pub name: String,
    /// The sources that found it, in a fixed order.
    pub sources: Vec<ExtensionSource>,
    /// The draft: the extension default with the proposals applied.
    pub draft: Draft,
    /// The evidence, one line each, each opening with its source.
    pub notes: Vec<String>,
}

/// A registration whose name the scan cannot read: a row marked dynamic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRegistration {
    /// The file the registration is in.
    pub file: String,
    /// Its line.
    pub line: usize,
    /// The registering call.
    pub api: String,
    /// The name expression as written.
    pub expression: String,
}

/// The result of describing an extension.
#[derive(Debug, Clone, Default)]
pub struct ExtensionImport {
    /// The package the sources provide, or the probe required.
    pub package: Option<String>,
    /// Its version, when something stated it.
    pub version: Option<String>,
    /// One row per command with a name, in name order.
    pub commands: Vec<ExtensionCommand>,
    /// The registrations whose names are computed.
    pub dynamic: Vec<DynamicRegistration>,
    /// The calls the scan is blind to, as `file:line: call: through what`.
    pub blind: Vec<String>,
    /// What the sources said that does not agree or does not read.
    pub warnings: Vec<String>,
    /// The entry point the description is of, when one was named.
    pub entry: Option<ExtensionEntry>,
}

impl ExtensionImport {
    /// The JSON the command line prints.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "package": self.package,
            "version": self.version,
            "commands": self.commands.iter().map(|command| json!({
                "name": command.name,
                "provenance": command.sources.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                "draft": Value::Object(command.draft.clone()),
                "notes": command.notes.clone(),
            })).collect::<Vec<_>>(),
            "dynamic": self.dynamic.iter().map(|row| json!({
                "provenance": [ExtensionSource::CScan.as_str()],
                "dynamic": true,
                "file": row.file,
                "line": row.line,
                "api": row.api,
                "name_expression": row.expression,
            })).collect::<Vec<_>>(),
            "blind": self.blind.clone(),
            "warnings": self.warnings.clone(),
            "entry": self.entry.as_ref().map(|entry| json!({
                "prefix": entry.prefix,
                "function": entry.function,
                "file": entry.file,
                "line": entry.line,
            })),
        })
    }

    /// Add what a probe observed: each command it saw gains the
    /// [`ExtensionSource::Probe`] source, and one the scan did not find is a
    /// row of its own, at the default and carrying no proposal.
    pub fn merge_probe(&mut self, probe: &ProbeReport) {
        match (&self.package, probe.package.as_str()) {
            (Some(known), required) if known != required => self.warnings.push(format!(
                "probe: required `{required}`, but the C source provides `{known}`"
            )),
            (None, required) => self.package = Some(required.to_owned()),
            _ => {}
        }
        if self.version.is_none() {
            self.version.clone_from(&probe.version);
        }
        for name in probe.commands.iter().map(|name| normalised(name)) {
            let note = format!(
                "probe: `package require {}` added `{name}` to the shell",
                probe.package
            );
            if let Some(row) = self.commands.iter_mut().find(|row| row.name == name) {
                row.sources.push(ExtensionSource::Probe);
                row.sources.sort();
                row.notes.push(note);
            } else {
                let mut row = default_row(name, self.package.as_deref());
                row.sources = vec![ExtensionSource::Probe];
                row.notes.push(note);
                self.commands.push(row);
            }
        }
        self.commands.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

/// An extension's entry point in the sources: the prefix `load` names it by,
/// and where its `PREFIX_Init` is defined (or its `PREFIX_SafeInit`, when that
/// is the only one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionEntry {
    /// The prefix (`Pkga`).
    pub prefix: String,
    /// The entry point's function (`Pkga_Init`).
    pub function: String,
    /// The file it is defined in.
    pub file: String,
    /// The line of its definition.
    pub line: usize,
}

impl std::fmt::Display for ExtensionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}:{})", self.function, self.file, self.line)
    }
}

/// Why the sources were not described: they define several extensions and
/// none was named, or the one named is not among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnchosenExtension {
    /// Several entry points, in file and line order.
    Several(Vec<ExtensionEntry>),
    /// The prefix named, and the entry points the sources do define.
    NotDefined {
        /// The prefix named.
        prefix: String,
        /// The entry points defined, in file and line order.
        defined: Vec<ExtensionEntry>,
    },
}

/// Describe the commands an extension registers, from a scan of the C sources
/// in `files`.
///
/// With `entry` `None`, the sources must hold one extension: every
/// registration in them is described when they define one entry point or none,
/// and [`UnchosenExtension::Several`] lists the entry points when they define
/// more. With `entry` the prefix of one (`Pkga`), the description is that
/// extension's: the registrations, packages and unreadable calls in the
/// functions its entry point reaches, by the names each function's body
/// mentions, across the files. A registration no entry point reaches is left
/// out with a warning, since nothing in the sources says whose it is.
///
/// # Errors
///
/// [`UnchosenExtension`] when the sources are not one extension's and `entry`
/// does not choose one of theirs.
pub fn import_c_sources(
    files: &[SourceFile],
    entry: Option<&str>,
) -> Result<ExtensionImport, UnchosenExtension> {
    let mut import = ExtensionImport::default();
    let scans: Vec<(&SourceFile, CScan)> = files
        .iter()
        .map(|file| (file, scan_c_source(&file.text)))
        .collect();
    let entries = entry_points(&scans);
    let scans = match entry {
        None if entries.len() > 1 => return Err(UnchosenExtension::Several(entries)),
        None => scans,
        Some(prefix) => {
            let Some(chosen) = entries.iter().find(|known| known.prefix == prefix) else {
                return Err(UnchosenExtension::NotDefined {
                    prefix: prefix.to_owned(),
                    defined: entries,
                });
            };
            import.entry = Some(chosen.clone());
            let scope = Scope::of(&scans, &entries, prefix);
            scans
                .into_iter()
                .map(|(file, scan)| (file, scope.narrow(file, scan, &mut import.warnings)))
                .collect()
        }
    };
    choose_package(&mut import, &scans);
    for (file, scan) in &scans {
        for blind in &scan.blind {
            import.blind.push(blind_text(&file.name, blind));
        }
        for command in &scan.commands {
            match &command.name {
                CName::Literal(name) => add_command(&mut import, normalised(name), command, file),
                CName::Dynamic(expression) => import.dynamic.push(DynamicRegistration {
                    file: file.name.clone(),
                    line: command.line,
                    api: command.api.clone(),
                    expression: expression.clone(),
                }),
            }
        }
    }
    import.commands.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(import)
}

/// The entry points the sources define, one per prefix (its `PREFIX_Init`
/// rather than its `PREFIX_SafeInit`), in file and line order.
fn entry_points(scans: &[(&SourceFile, CScan)]) -> Vec<ExtensionEntry> {
    let mut entries: Vec<ExtensionEntry> = Vec::new();
    for (file, scan) in scans {
        for point in &scan.entry_points {
            let entry = ExtensionEntry {
                prefix: point.prefix.clone(),
                function: point.function.clone(),
                file: file.name.clone(),
                line: point.line,
            };
            match entries
                .iter_mut()
                .find(|known| known.prefix == point.prefix)
            {
                Some(known) if known.function.ends_with("_SafeInit") => *known = entry,
                Some(_) => {}
                None => entries.push(entry),
            }
        }
    }
    entries
}

/// The functions one entry point reaches, and those any entry point reaches.
struct Scope {
    prefix: String,
    chosen: BTreeSet<String>,
    any: BTreeSet<String>,
}

impl Scope {
    /// The scope of the extension `prefix` among `entries`.
    fn of(scans: &[(&SourceFile, CScan)], entries: &[ExtensionEntry], prefix: &str) -> Self {
        let mut references: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (_, scan) in scans {
            for (function, named) in &scan.references {
                references
                    .entry(function)
                    .or_default()
                    .extend(named.iter().map(String::as_str));
            }
        }
        let reach = |prefix: &str| {
            let mut reached: BTreeSet<String> = BTreeSet::new();
            let mut pending: Vec<String> = [format!("{prefix}_Init"), format!("{prefix}_SafeInit")]
                .into_iter()
                .filter(|function| references.contains_key(function.as_str()))
                .collect();
            while let Some(function) = pending.pop() {
                if !reached.insert(function.clone()) {
                    continue;
                }
                if let Some(named) = references.get(function.as_str()) {
                    pending.extend(
                        named
                            .iter()
                            .filter(|name| references.contains_key(*name))
                            .map(|name| (*name).to_owned()),
                    );
                }
            }
            reached
        };
        Self {
            prefix: prefix.to_owned(),
            chosen: reach(prefix),
            any: entries
                .iter()
                .flat_map(|entry| reach(&entry.prefix))
                .collect(),
        }
    }

    /// Whether the extension's entry point reaches `function`.
    fn reaches(&self, function: Option<&String>) -> bool {
        function.is_some_and(|function| self.chosen.contains(function))
    }

    /// `scan` with only what the extension's entry point reaches, and a warning
    /// for each registration no entry point reaches.
    fn narrow(&self, file: &SourceFile, mut scan: CScan, warnings: &mut Vec<String>) -> CScan {
        for command in &scan.commands {
            let reached = command
                .function
                .as_ref()
                .is_some_and(|function| self.any.contains(function));
            if !reached {
                warnings.push(format!(
                    "c-scan: {}:{}: `{}` is registered {}, which no entry point reaches, so the \
                     pack for `{}` leaves it out",
                    file.name,
                    command.line,
                    expression_of(&command.name),
                    command
                        .function
                        .as_ref()
                        .map_or_else(|| "outside a function".to_owned(), |f| format!("in `{f}`")),
                    self.prefix
                ));
            }
        }
        scan.commands
            .retain(|command| self.reaches(command.function.as_ref()));
        scan.packages
            .retain(|package| self.reaches(package.function.as_ref()));
        scan.blind
            .retain(|blind| self.reaches(blind.function.as_ref()));
        scan
    }
}

/// The package the sources provide: the first literal one, with a warning for
/// each further name and for one whose name or version is not a literal.
fn choose_package(import: &mut ExtensionImport, scans: &[(&SourceFile, CScan)]) {
    for (file, scan) in scans {
        for package in &scan.packages {
            let CPackage {
                name,
                version,
                line,
                ..
            } = package;
            let CName::Literal(name) = name else {
                import.warnings.push(format!(
                    "c-scan: {}:{line}: the package name is computed ({})",
                    file.name,
                    expression_of(name)
                ));
                continue;
            };
            match &import.package {
                None => {
                    import.package = Some(name.clone());
                    match version {
                        CName::Literal(version) => import.version = Some(version.clone()),
                        CName::Dynamic(expression) => import.warnings.push(format!(
                            "c-scan: {}:{line}: the version of `{name}` is not a literal ({expression})",
                            file.name
                        )),
                    }
                }
                Some(chosen) if chosen != name => import.warnings.push(format!(
                    "c-scan: {}:{line}: also provides `{name}`; the pack is for `{chosen}`",
                    file.name
                )),
                Some(_) => {}
            }
        }
    }
}

fn expression_of(name: &CName) -> &str {
    match name {
        CName::Literal(text) | CName::Dynamic(text) => text,
    }
}

fn blind_text(file: &str, blind: &CBlindSpot) -> String {
    format!(
        "{file}:{}: {}: registered through {}, which the scan cannot read",
        blind.line, blind.api, blind.through
    )
}

/// A command name without its leading `::`: the global namespace spells it
/// either way, and a scan and a probe must agree on which row a name is.
fn normalised(name: &str) -> &str {
    name.strip_prefix("::").unwrap_or(name)
}

/// Add the row for `command`, or a note on the row already there when a
/// second registration of the same name is found.
fn add_command(
    import: &mut ExtensionImport,
    name: &str,
    command: &CDeclaredCommand,
    file: &SourceFile,
) {
    if let Some(row) = import.commands.iter_mut().find(|row| row.name == name) {
        row.notes.push(format!(
            "c-scan: registered again by {} at {}:{}; the first registration's facts stand",
            command.api, file.name, command.line
        ));
        return;
    }
    let package = import.package.clone();
    let mut row = default_row(name, package.as_deref());
    row.sources = vec![ExtensionSource::CScan];
    describe(&mut row, command, file);
    import.commands.push(row);
}

/// A row for `name` at the extension default, carrying no proposal.
fn default_row(name: &str, package: Option<&str>) -> ExtensionCommand {
    let mut draft = draft::from_command_spec(&CommandSpec::extension_default(""));
    draft.insert("name".into(), json!(name));
    if let Some(package) = package {
        draft.insert("required_package".into(), json!(package));
    }
    ExtensionCommand {
        name: name.to_owned(),
        sources: Vec::new(),
        draft,
        notes: Vec::new(),
    }
}

/// Apply what the scan read of `command` to its row: the proposals, and a
/// note for each fact with the line it came from.
fn describe(row: &mut ExtensionCommand, command: &CDeclaredCommand, file: &SourceFile) {
    let at = |line: usize| format!("{}:{line}", file.name);
    row.notes.push(match &command.procedure {
        Some(procedure) => format!(
            "c-scan: registered by {} at {}, procedure `{procedure}`",
            command.api,
            at(command.line)
        ),
        None => format!(
            "c-scan: registered by {} at {}; the procedure is not defined in the scanned text, \
             so nothing is read of it",
            command.api,
            at(command.line)
        ),
    });
    if !command.subcommands.is_empty() {
        let subcommands = command
            .subcommands
            .iter()
            .map(|name| {
                let mut sub = draft::default_subcommand_draft();
                sub.insert("name".into(), json!(name));
                Value::Object(sub)
            })
            .collect();
        row.draft
            .insert("subcommands".into(), Value::Array(subcommands));
        row.notes.push(format!(
            "c-scan: subcommands {} from the option table `{}` reads",
            command.subcommands.join(", "),
            command.procedure.as_deref().unwrap_or("its procedure")
        ));
    }
    propose_arity(row, command, &at);
    note_evidence(row, command);
}

/// Propose an arity from the usage messages that describe the command's own
/// arguments, or from the existence of subcommands.
fn propose_arity(
    row: &mut ExtensionCommand,
    command: &CDeclaredCommand,
    at: &dyn Fn(usize) -> String,
) {
    let own: Vec<_> = command
        .usage
        .iter()
        .filter(|usage| usage.words == 1)
        .collect();
    for usage in command.usage.iter().filter(|usage| usage.words > 1) {
        row.notes.push(format!(
            "c-scan: usage `{}` for a form that repeats {} leading words ({})",
            usage.text,
            usage.words,
            at(usage.line)
        ));
    }
    let mut arities: Vec<Arity> = own
        .iter()
        .map(|usage| arity_of_usage(&usage.text))
        .collect();
    if arities.is_empty() && !command.subcommands.is_empty() {
        arities.push(Arity::at_least(1));
    }
    let Some(proposed) = union_of(&arities) else {
        return;
    };
    for usage in &own {
        row.notes.push(format!(
            "c-scan: usage `{}` ({})",
            usage.text,
            at(usage.line)
        ));
    }
    row.notes.push(format!(
        "c-scan: proposes arity {}",
        describe_arity(proposed)
    ));
    row.draft.insert("arity".into(), draft::arity(proposed));
}

/// The arity that admits every alternative.
fn union_of(arities: &[Arity]) -> Option<Arity> {
    let first = arities.first()?;
    let min = arities
        .iter()
        .map(|arity| arity.min)
        .min()
        .unwrap_or(first.min);
    let max = arities
        .iter()
        .map(|arity| arity.max)
        .max()
        .unwrap_or(first.max);
    Some(Arity::new(min, max))
}

fn describe_arity(arity: Arity) -> String {
    if arity.is_unlimited() {
        format!("at least {}", arity.min)
    } else if arity.max == arity.min {
        format!("exactly {}", arity.min)
    } else {
        format!("{} to {}", arity.min, arity.max)
    }
}

/// The arity a `Tcl_WrongNumArgs` message states: a word outside `?…?` is
/// required, a word inside is optional, and `...` leaves the end open.
fn arity_of_usage(text: &str) -> Arity {
    let (mut required, mut optional) = (0u16, 0u16);
    let mut open = false;
    let mut inside = false;
    for word in text.split_whitespace() {
        if word.starts_with('?') {
            inside = true;
        }
        let ends_group = word.ends_with('?');
        if word.contains("...") {
            open = true;
        } else if inside {
            optional = optional.saturating_add(1);
        } else {
            required = required.saturating_add(1);
        }
        if ends_group && (word.len() > 1 || inside) {
            inside = false;
        }
    }
    if open {
        Arity::at_least(required)
    } else {
        Arity::new(required, required.saturating_add(optional))
    }
}

/// One note for what the procedure's own body calls, and one saying what the
/// absence of such a call does not mean.
fn note_evidence(row: &mut ExtensionCommand, command: &CDeclaredCommand) {
    let Some(procedure) = &command.procedure else {
        return;
    };
    let evidence = &command.evidence;
    let groups: [(&BTreeSet<String>, &str); 3] = [
        (
            &evidence.evaluates,
            "it evaluates a script or an expression",
        ),
        (&evidence.variables, "it touches a variable by name"),
        (&evidence.command_table, "it changes the command table"),
    ];
    let mut any = false;
    for (calls, what) in groups {
        if !calls.is_empty() {
            any = true;
            row.notes.push(format!(
                "c-scan: `{procedure}` calls {} ({what})",
                calls.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if !any {
        row.notes.push(format!(
            "c-scan: `{procedure}` itself calls nothing that evaluates, touches a variable or \
             changes the command table; its callees are not read, so every axis stays at the \
             extension default"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, text: &str) -> SourceFile {
        SourceFile {
            name: name.to_owned(),
            text: text.to_owned(),
        }
    }

    /// The real test extension, described: every row at the extension default,
    /// each with the provenance `c-scan`, the arity its usage message states,
    /// the subcommands of its table, the package it provides, and notes that say
    /// where each came from.
    #[test]
    fn pkga_is_described_at_the_default_fact_with_its_proposals() {
        let import = import_c_sources(
            &[file(
                "pkga.c",
                include_str!("../../../tcl-cshim/tests/c/pkga.c"),
            )],
            None,
        )
        .expect("one extension");
        assert_eq!(import.package.as_deref(), Some("pkga"));
        assert_eq!(import.version.as_deref(), Some("1.0"));
        assert!(import.dynamic.is_empty() && import.warnings.is_empty());
        let names: Vec<&str> = import
            .commands
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "pkga_calc",
                "pkga_count",
                "pkga_eq",
                "pkga_forget",
                "pkga_quote"
            ]
        );
        let default = draft::from_command_spec(&CommandSpec::extension_default(""));
        for row in &import.commands {
            assert_eq!(row.sources, [ExtensionSource::CScan], "{}", row.name);
            assert_eq!(row.draft["traits"], default["traits"], "{}", row.name);
            assert_eq!(
                row.draft["side_effects"], default["side_effects"],
                "{}",
                row.name
            );
            assert_eq!(row.draft["runtime_backing"], json!("host-native"));
            assert_eq!(row.draft["required_package"], json!("pkga"));
            assert!(
                row.notes.iter().all(|note| note.starts_with("c-scan:")),
                "{:?}",
                row.notes
            );
        }
        let row = |name: &str| import.commands.iter().find(|r| r.name == name).expect(name);
        assert_eq!(
            row("pkga_eq").draft["arity"],
            json!({"min": 2, "max": 2, "step": 0, "also_exact": null})
        );
        assert_eq!(row("pkga_count").draft["arity"]["max"], json!(0));
        assert_eq!(row("pkga_calc").draft["arity"]["min"], json!(1));
        assert_eq!(row("pkga_calc").draft["arity"]["max"], Value::Null);
        let subs = row("pkga_calc").draft["subcommands"]
            .as_array()
            .expect("subs")
            .clone();
        let sub_names: Vec<&str> = subs.iter().filter_map(|s| s["name"].as_str()).collect();
        assert_eq!(sub_names.len(), 10);
        assert_eq!(sub_names[0], "add");
        assert!(
            row("pkga_forget")
                .notes
                .iter()
                .any(|n| n.contains("Tcl_DeleteCommand"))
        );
        assert!(
            row("pkga_eq")
                .notes
                .iter()
                .any(|n| n.contains("every axis stays at the extension default")),
            "{:?}",
            row("pkga_eq").notes
        );
    }

    #[test]
    fn a_computed_registration_is_a_dynamic_row_and_names_no_command() {
        let import = import_c_sources(
            &[file(
                "factory.c",
                "int F(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {\n\
             Tcl_CreateObjCommand(interp, Tcl_GetString(objv[1]), P, NULL, NULL);\n return 0; }\n",
            )],
            None,
        )
        .expect("one extension");
        assert!(import.commands.is_empty());
        assert_eq!(
            import.dynamic,
            [DynamicRegistration {
                file: "factory.c".to_owned(),
                line: 2,
                api: "Tcl_CreateObjCommand".to_owned(),
                expression: "Tcl_GetString(objv[1])".to_owned(),
            }]
        );
        let json = import.to_json();
        assert_eq!(json["dynamic"][0]["dynamic"], json!(true));
        assert_eq!(json["dynamic"][0]["provenance"], json!(["c-scan"]));
    }

    #[test]
    fn a_leading_double_colon_does_not_make_a_second_row() {
        let mut import = import_c_sources(&[file(
            "x.c",
            r#"int Init(Tcl_Interp *i) { Tcl_CreateObjCommand(i, "::x::cmd", P, 0, 0); return 0; }"#,
        )], None)
        .expect("one extension");
        import.merge_probe(&ProbeReport {
            package: "x".to_owned(),
            version: None,
            commands: vec!["::x::cmd".to_owned()],
        });
        assert_eq!(import.commands.len(), 1);
        assert_eq!(import.commands[0].name, "x::cmd");
        assert_eq!(
            import.commands[0].sources,
            [ExtensionSource::CScan, ExtensionSource::Probe]
        );
    }

    #[test]
    fn a_usage_message_states_an_arity() {
        let arity = |text: &str| {
            let a = arity_of_usage(text);
            (a.min, if a.is_unlimited() { None } else { Some(a.max) })
        };
        assert_eq!(arity(""), (0, Some(0)));
        assert_eq!(arity("string1 string2"), (2, Some(2)));
        assert_eq!(arity("?-nocase? pattern string"), (2, Some(3)));
        assert_eq!(arity("?-flag value? name"), (1, Some(3)));
        assert_eq!(arity("subcommand ?arg ...?"), (1, None));
        assert_eq!(arity("name ..."), (1, None));
    }

    #[test]
    fn the_union_of_alternatives_admits_each() {
        let both = union_of(&[Arity::new(1, 1), Arity::new(2, 3)]).expect("one");
        assert_eq!((both.min, both.max), (1, 3));
        let open = union_of(&[Arity::new(1, 1), Arity::at_least(0)]).expect("one");
        assert_eq!(open.min, 0);
        assert!(open.is_unlimited());
        assert!(union_of(&[]).is_none());
    }

    #[test]
    fn a_probe_adds_its_source_to_a_known_row_and_a_row_for_a_new_name() {
        let mut import = import_c_sources(
            &[file(
                "x.c",
                r#"int Init(Tcl_Interp *interp) {
                   Tcl_PkgProvide(interp, "x", "1.0");
                   Tcl_CreateObjCommand(interp, "x_known", P, NULL, NULL);
                   return 0;
               }"#,
            )],
            None,
        )
        .expect("one extension");
        import.merge_probe(&ProbeReport {
            package: "x".to_owned(),
            version: Some("1.0".to_owned()),
            commands: vec!["x_known".to_owned(), "x_only_seen".to_owned()],
        });
        let sources = |name: &str| {
            import
                .commands
                .iter()
                .find(|row| row.name == name)
                .map(|row| row.sources.clone())
        };
        assert_eq!(
            sources("x_known"),
            Some(vec![ExtensionSource::CScan, ExtensionSource::Probe])
        );
        assert_eq!(sources("x_only_seen"), Some(vec![ExtensionSource::Probe]));
        assert!(import.warnings.is_empty());
        let only = import
            .commands
            .iter()
            .find(|row| row.name == "x_only_seen")
            .expect("row");
        assert_eq!(only.draft["required_package"], json!("x"));
        assert_eq!(only.draft["runtime_backing"], json!("host-native"));
    }

    #[test]
    fn a_probe_of_another_package_than_the_source_provides_is_a_warning() {
        let mut import = import_c_sources(
            &[file(
                "x.c",
                r#"int Init(Tcl_Interp *i) { Tcl_PkgProvide(i, "x", "1.0"); return 0; }"#,
            )],
            None,
        )
        .expect("one extension");
        import.merge_probe(&ProbeReport {
            package: "y".to_owned(),
            ..ProbeReport::default()
        });
        assert!(
            import.warnings.iter().any(|w| w.contains("required `y`")),
            "{:?}",
            import.warnings
        );
    }
    #[test]
    fn a_second_registration_of_a_name_is_a_note_and_no_second_row() {
        let import = import_c_sources(
            &[file(
                "x.c",
                "int Init(Tcl_Interp *i) {\n Tcl_CreateObjCommand(i, \"dup\", P, 0, 0);\n \
             Tcl_CreateCommand(i, \"dup\", Q, 0, 0);\n return 0; }",
            )],
            None,
        )
        .expect("one extension");
        assert_eq!(import.commands.len(), 1);
        assert!(
            import.commands[0]
                .notes
                .iter()
                .any(|n| n.contains("registered again by Tcl_CreateCommand at x.c:3")),
            "{:?}",
            import.commands[0].notes
        );
    }

    #[test]
    fn a_second_provided_package_and_an_unreadable_version_are_warnings() {
        let import = import_c_sources(
            &[file(
                "x.c",
                "int Init(Tcl_Interp *i) {\n Tcl_PkgProvide(i, \"first\", VERSION);\n \
             Tcl_PkgProvide(i, \"second\", \"2.0\");\n Tcl_PkgProvide(i, names[0], \"3.0\");\n \
             return 0; }",
            )],
            None,
        )
        .expect("one extension");
        assert_eq!(import.package.as_deref(), Some("first"));
        assert_eq!(import.version, None);
        let said = |part: &str| import.warnings.iter().any(|w| w.contains(part));
        assert!(
            said("the version of `first` is not a literal (VERSION)"),
            "{:?}",
            import.warnings
        );
        assert!(
            said("also provides `second`; the pack is for `first`"),
            "{:?}",
            import.warnings
        );
        assert!(
            said("the package name is computed (names[0])"),
            "{:?}",
            import.warnings
        );
    }

    #[test]
    fn subcommands_alone_propose_at_least_one_argument() {
        let import = import_c_sources(
            &[file(
                "x.c",
                r#"
            static const char *const subs[] = { "a", "b", NULL };
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_GetIndexFromObj(interp, objv[1], subs, "sub", 0, &i);
                return 0;
            }
            int Init(Tcl_Interp *interp) { Tcl_CreateObjCommand(interp, "ext", P, 0, 0); return 0; }
            "#,
            )],
            None,
        )
        .expect("one extension");
        assert_eq!(import.commands[0].draft["arity"]["min"], json!(1));
        assert_eq!(import.commands[0].draft["arity"]["max"], Value::Null);
    }

    #[test]
    fn a_usage_that_repeats_a_subcommand_word_is_a_note_and_proposes_no_arity() {
        let import = import_c_sources(
            &[file(
                "x.c",
                r#"
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_WrongNumArgs(interp, 2, objv, "n m");
                return 0;
            }
            int Init(Tcl_Interp *interp) { Tcl_CreateObjCommand(interp, "ext", P, 0, 0); return 0; }
            "#,
            )],
            None,
        )
        .expect("one extension");
        let default = draft::from_command_spec(&CommandSpec::extension_default(""));
        assert_eq!(import.commands[0].draft["arity"], default["arity"]);
        assert!(
            import.commands[0]
                .notes
                .iter()
                .any(|n| n.contains("usage `n m` for a form that repeats 2 leading words")),
            "{:?}",
            import.commands[0].notes
        );
    }

    #[test]
    fn a_probe_alone_names_the_package_the_version_and_every_command() {
        let mut import = ExtensionImport::default();
        import.merge_probe(&ProbeReport {
            package: "p".to_owned(),
            version: Some("2.1".to_owned()),
            commands: vec!["::p::one".to_owned(), "two".to_owned()],
        });
        assert_eq!(import.package.as_deref(), Some("p"));
        assert_eq!(import.version.as_deref(), Some("2.1"));
        let names: Vec<&str> = import
            .commands
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(names, ["p::one", "two"]);
        for row in &import.commands {
            assert_eq!(row.sources, [ExtensionSource::Probe]);
            assert_eq!(row.draft["required_package"], json!("p"));
            assert!(row.notes.iter().all(|n| n.starts_with("probe:")));
        }
    }

    #[test]
    fn what_a_procedure_calls_is_noted_by_axis() {
        let import = import_c_sources(
            &[file(
                "x.c",
                r#"
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_EvalObjEx(interp, objv[1], 0);
                Tcl_ObjSetVar2(interp, objv[2], NULL, objv[3], 0);
                return 0;
            }
            int Init(Tcl_Interp *interp) { Tcl_CreateObjCommand(interp, "ext", P, 0, 0); return 0; }
            "#,
            )],
            None,
        )
        .expect("one extension");
        let notes = &import.commands[0].notes;
        assert!(
            notes
                .iter()
                .any(|n| n
                    .contains("`P` calls Tcl_EvalObjEx (it evaluates a script or an expression)")),
            "{notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("`P` calls Tcl_ObjSetVar2 (it touches a variable by name)")),
            "{notes:?}"
        );
        assert!(
            !notes
                .iter()
                .any(|n| n.contains("every axis stays at the extension default")),
            "{notes:?}"
        );
    }

    /// The directory of the shim's test extensions holds two, each with its
    /// own entry point: described with neither named, the import lists both
    /// rather than describe one of them or a mixture of the two (negative);
    /// with one named, it describes that one's commands and package and none
    /// of the other's; and a prefix the sources do not define is refused with
    /// the ones they do.
    #[test]
    fn two_entry_points_are_two_extensions_described_one_at_a_time() {
        let files = [
            file(
                "doors.c",
                include_str!("../../../tcl-cshim/tests/c/doors.c"),
            ),
            file(
                "layout.c",
                include_str!("../../../tcl-cshim/tests/c/layout.c"),
            ),
            file("pkga.c", include_str!("../../../tcl-cshim/tests/c/pkga.c")),
        ];
        let Err(UnchosenExtension::Several(entries)) = import_c_sources(&files, None) else {
            panic!("two entry points are two extensions");
        };
        let listed: Vec<String> = entries.iter().map(ToString::to_string).collect();
        assert_eq!(
            listed,
            ["Doors_Init (doors.c:261)", "Pkga_Init (pkga.c:303)"]
        );

        let pkga = import_c_sources(&files, Some("Pkga")).expect("Pkga is defined");
        assert_eq!(pkga.package.as_deref(), Some("pkga"));
        let names: Vec<&str> = pkga.commands.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "pkga_calc",
                "pkga_count",
                "pkga_eq",
                "pkga_forget",
                "pkga_quote"
            ]
        );
        assert!(pkga.warnings.is_empty(), "{:?}", pkga.warnings);
        assert_eq!(pkga.entry.as_ref().map(|entry| entry.line), Some(303));

        let doors = import_c_sources(&files, Some("Doors")).expect("Doors is defined");
        assert_eq!(doors.package.as_deref(), Some("doors"));
        assert!(
            doors
                .commands
                .iter()
                .all(|row| row.name.starts_with("doors_")),
            "only doors' own commands"
        );
        assert_eq!(doors.commands.len(), 11);
        for row in &doors.commands {
            assert_eq!(
                row.draft["required_package"],
                json!("doors"),
                "{}",
                row.name
            );
        }

        let Err(UnchosenExtension::NotDefined { prefix, defined }) =
            import_c_sources(&files, Some("Nosuch"))
        else {
            panic!("an undefined prefix is refused");
        };
        assert_eq!(prefix, "Nosuch");
        assert_eq!(defined, entries);
    }

    /// An entry point owns the registrations of the functions it names, and of
    /// the functions those name, across files: a helper in another file that
    /// registers a command, and the command procedures an entry point registers,
    /// which can register commands of their own. A registration no entry point
    /// reaches is left out with a warning (negative), and a `PREFIX_SafeInit`
    /// beside a `PREFIX_Init` is the same extension, not a second one.
    #[test]
    fn an_entry_point_owns_what_the_functions_it_names_register() {
        let files = [
            file(
                "ext.c",
                r#"
                int Ext_Init(Tcl_Interp *interp) {
                    Tcl_PkgProvide(interp, "ext", "2.0");
                    Ext_Register(interp);
                    Tcl_CreateObjCommand(interp, "ext_make", ExtMakeCmd, 0, 0);
                    return 0;
                }
                int Ext_SafeInit(Tcl_Interp *interp) { return Ext_Init(interp); }
                static int ExtMakeCmd(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                    Tcl_CreateObjCommand(interp, "ext_made", ExtMakeCmd, 0, 0);
                    return 0;
                }
                "#,
            ),
            file(
                "register.c",
                r#"
                void Ext_Register(Tcl_Interp *interp) {
                    Tcl_CreateObjCommand(interp, "ext_helped", P, 0, 0);
                }
                void Orphan(Tcl_Interp *interp) {
                    Tcl_CreateObjCommand(interp, "lost", P, 0, 0);
                }
                "#,
            ),
            file(
                "other.c",
                r#"
                int Other_Init(Tcl_Interp *interp) {
                    Tcl_PkgProvide(interp, "other", "1.0");
                    Tcl_CreateObjCommand(interp, "other_cmd", P, 0, 0);
                    Tcl_NewMethod(interp, c, n, 1, &t, 0);
                    return 0;
                }
                "#,
            ),
        ];
        let Err(UnchosenExtension::Several(entries)) = import_c_sources(&files, None) else {
            panic!("two prefixes are two extensions");
        };
        let functions: Vec<&str> = entries
            .iter()
            .map(|entry| entry.function.as_str())
            .collect();
        assert_eq!(
            functions,
            ["Ext_Init", "Other_Init"],
            "the safe entry is Ext's"
        );

        let ext = import_c_sources(&files, Some("Ext")).expect("Ext is defined");
        assert_eq!(ext.package.as_deref(), Some("ext"));
        let names: Vec<&str> = ext.commands.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["ext_helped", "ext_made", "ext_make"]);
        assert_eq!(ext.warnings.len(), 1, "{:?}", ext.warnings);
        assert!(
            ext.warnings[0].contains(
                "register.c:6: `lost` is registered in `Orphan`, which no \
                                      entry point reaches, so the pack for `Ext` leaves it out"
            ),
            "{:?}",
            ext.warnings
        );

        assert!(
            ext.blind.is_empty(),
            "the other's unreadable call is the other's"
        );

        let other = import_c_sources(&files, Some("Other")).expect("Other is defined");
        let names: Vec<&str> = other.commands.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["other_cmd"]);
        assert_eq!(other.package.as_deref(), Some("other"));
        assert_eq!(other.blind.len(), 1, "{:?}", other.blind);

        let one = import_c_sources(&files[..1], None).expect("one prefix is one extension");
        assert_eq!(one.package.as_deref(), Some("ext"));
        assert!(one.entry.is_none(), "nothing was chosen");
    }
}
