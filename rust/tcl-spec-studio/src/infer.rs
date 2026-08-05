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

//! Guess command specs from a package's own Tcl source.
//!
//! Importing a package runs the real compiler over its files — the same
//! analyser the language server uses — and turns each `proc` it finds into a
//! starting draft:
//!
//! - **arity** from the parameter list: parameters without a default are
//!   required, ones with a default are optional, and a trailing `args`
//!   makes the command variadic.
//! - **argument roles** from the analyser's parameter-trait inference
//!   (`docs/design/contracts/proc-arg-traits.md`), which watches how each
//!   parameter is *used* in the body — evaluated as a script, upvar'd and
//!   written, used as a `foreach` list, invoked as a command — and maps that
//!   usage onto the registry's [`ArgRole`] vocabulary.
//! - **traits** from the same evidence: a parameter eval'd as a script makes
//!   the command an `EVALUATES_CODE` dynamic barrier, an upvar'd write makes
//!   it a scope-alias creator, and so on.
//! - **hover text** from the proc's doc comment, and a synopsis rebuilt from
//!   the parameter list.
//! - **package gating** from `package provide`, so the drafts come back
//!   already scoped to the package that defines them.
//!
//! Everything inferred is a *starting point* carrying its own evidence
//! ([`Inferred::notes`]), not an assertion — the studio shows the reasoning
//! next to each guess so the author can accept or overrule it.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::analyser::types::ProcArgTrait;
use tcl_compiler::signature_scan::arity::arity_of;
use tcl_compiler::signature_scan::types::ParamDef;

use crate::draft::{self, Draft};

/// One file of an imported package.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Display name, used in the evidence notes.
    pub name: String,
    /// The file's Tcl source.
    pub text: String,
}

/// A draft the importer produced, with the evidence behind it.
#[derive(Debug, Clone)]
pub struct Inferred {
    /// The command name the draft is for.
    pub name: String,
    /// The seeded draft, ready for the form.
    pub draft: Draft,
    /// Why each guess was made, one line each.
    pub notes: Vec<String>,
}

impl Inferred {
    /// The JSON the front-end consumes.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "draft": Value::Object(self.draft.clone()),
            "notes": self.notes.clone(),
        })
    }
}

/// The result of importing a package.
#[derive(Debug, Clone, Default)]
pub struct Import {
    /// Package name from `package provide`, when the sources declare one.
    pub package: Option<String>,
    /// Package version from `package provide`.
    pub version: Option<String>,
    /// One draft per procedure found, in qualified-name order.
    pub commands: Vec<Inferred>,
    /// Files that produced no procedures, and why.
    pub warnings: Vec<String>,
}

impl Import {
    /// The JSON the front-end consumes.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "package": self.package,
            "version": self.version,
            "commands": self.commands.iter().map(Inferred::to_json).collect::<Vec<_>>(),
            "warnings": self.warnings.clone(),
        })
    }
}

/// The registry [`ArgRole`](tcl_registry::ArgRole) variant a parameter trait
/// implies, or `None` when the trait says nothing about the argument's role.
fn role_for_trait(t: ProcArgTrait) -> Option<&'static str> {
    match t {
        // An eval'd argument and a control body are both scripts as far as the
        // registry's role vocabulary is concerned; the difference is captured
        // in the traits, not the role.
        ProcArgTrait::Eval | ProcArgTrait::Body => Some("Body"),
        ProcArgTrait::VarWrite => Some("VarWrite"),
        ProcArgTrait::VarRead => Some("VarRead"),
        ProcArgTrait::Expr => Some("Expr"),
        ProcArgTrait::LoopList => Some("Value"),
        ProcArgTrait::Command => Some("CommandPrefix"),
        // Callee-local dynamic naming is not a caller-visible role: passing a
        // literal does not consume the caller's variable, so marking it
        // `VarWrite` would be wrong.
        ProcArgTrait::DynamicNameLocal => None,
    }
}

/// Rank a role so the strongest evidence wins when a parameter carries
/// several traits — a body that is also read as a variable name is a body.
fn role_rank(role: &str) -> u8 {
    match role {
        "Body" => 5,
        "Expr" => 4,
        "CommandPrefix" => 3,
        "VarWrite" => 2,
        "VarRead" => 1,
        _ => 0,
    }
}

/// The trait bits a parameter's usage implies for the whole command.
fn traits_for(t: ProcArgTrait) -> &'static [&'static str] {
    match t {
        ProcArgTrait::Eval => &["EVALUATES_CODE", "CREATES_DYNAMIC_BARRIER"],
        ProcArgTrait::Body => &["HAS_LOOP_BODY"],
        ProcArgTrait::VarWrite => &["CREATES_SCOPE_ALIAS", "TARGETS_VARIABLE_BY_NAME"],
        ProcArgTrait::VarRead => &["TARGETS_VARIABLE_BY_NAME"],
        ProcArgTrait::Expr => &["HAS_BOOLEAN_COND"],
        ProcArgTrait::LoopList => &["LOOP_LIST_HEADER"],
        ProcArgTrait::Command => &["BUILDS_COMMAND_PREFIX", "INVOKES_USER_PROC"],
        ProcArgTrait::DynamicNameLocal => &[],
    }
}

/// A one-line explanation of what a trait observation means.
fn evidence(param: &str, t: ProcArgTrait) -> String {
    let what = match t {
        ProcArgTrait::Eval => "is evaluated as a script",
        ProcArgTrait::Body => "is used as a control or loop body",
        ProcArgTrait::VarWrite => "names a caller variable the body writes via upvar",
        ProcArgTrait::VarRead => "names a caller variable the body reads via upvar",
        ProcArgTrait::Expr => "is evaluated as an expression",
        ProcArgTrait::LoopList => "is iterated as a list",
        ProcArgTrait::DynamicNameLocal => "names one of the proc's own locals",
        ProcArgTrait::Command => "is invoked as a command",
    };
    format!("`{param}` {what}")
}

/// Whether `params` ends in the variadic `args` collector.
///
/// Only a *trailing* `args` is variadic — `proc f {args value} {}` has an
/// ordinary required parameter that happens to be spelled `args`. See
/// [`tcl_compiler::signature_scan::arity::arity_of`], which applies the same
/// rule to the arity.
fn is_variadic(params: &[ParamDef]) -> bool {
    params.last().is_some_and(|p| p.name == "args")
}

/// Build the synopsis line a proc's parameter list implies.
fn synopsis(name: &str, params: &[ParamDef]) -> String {
    let variadic = is_variadic(params);
    let last = params.len().saturating_sub(1);
    let mut out = name.to_owned();
    for (index, param) in params.iter().enumerate() {
        out.push(' ');
        if variadic && index == last {
            out.push_str("?arg ...?");
        } else if param.has_default {
            out.push('?');
            out.push_str(&param.name);
            out.push('?');
        } else {
            out.push_str(&param.name);
        }
    }
    out
}

/// Turn one analysed procedure into a draft.
fn draft_for_proc(
    proc_def: &tcl_compiler::analyser::types::ProcDef,
    package: Option<&str>,
    version: Option<&str>,
    file: &str,
) -> Inferred {
    let mut d = draft::default_command_draft();
    let mut notes: Vec<String> = Vec::new();

    // The unqualified name is what a caller writes once the namespace is
    // imported; the qualified one is what resolution actually binds.
    let name = if proc_def.qualified_name.trim_start_matches("::").is_empty() {
        proc_def.name.clone()
    } else {
        proc_def.qualified_name.trim_start_matches("::").to_owned()
    };
    d.insert("name".into(), json!(name));
    notes.push(format!(
        "found `proc {}` in {file}",
        proc_def.qualified_name
    ));

    // Arity comes from the compiler's canonical rule, not a local re-derivation.
    // Tcl binds arguments strictly positionally, so a required parameter *after*
    // a defaulted one raises the minimum to its own position rather than leaving
    // the earlier default optional: `proc p {{a A} b} {}` takes exactly two.
    // `arity_of` encodes that (and the trailing-`args` rule), verified there
    // against real tclsh.
    let params = &proc_def.params;
    let arity = arity_of(params);
    let unbounded = arity.is_unlimited();
    d.insert(
        "arity".into(),
        json!({
            "min": arity.min,
            "max": if unbounded { Value::Null } else { json!(arity.max) },
            "step": 0,
            "also_exact": null,
        }),
    );
    notes.push(if unbounded {
        format!(
            "arity {}.. — {} argument(s) required, then a variadic `args`",
            arity.min, arity.min
        )
    } else {
        format!(
            "arity {}..{} — {} required, {} optional",
            arity.min,
            arity.max,
            arity.min,
            arity.max - arity.min
        )
    });

    infer_roles_and_traits(&mut d, proc_def, &mut notes);
    describe(&mut d, proc_def, &name, package, version, file, &mut notes);

    notes.push(
        "every guess above is a starting point — check it against the command's documentation"
            .to_owned(),
    );

    Inferred {
        name,
        draft: d,
        notes,
    }
}

/// Assign argument roles and command traits from how the body uses each
/// parameter, recording the evidence for every observation.
fn infer_roles_and_traits(
    d: &mut Draft,
    proc_def: &tcl_compiler::analyser::types::ProcDef,
    notes: &mut Vec<String>,
) {
    // Roles and traits, from how each parameter is used in the body.
    let mut roles: Vec<Value> = Vec::new();
    let mut trait_keys: BTreeSet<&'static str> = BTreeSet::new();
    for (index, param) in proc_def.params.iter().enumerate() {
        let Some(observed) = proc_def.param_traits.get(&param.name) else {
            continue;
        };
        let mut sorted: Vec<ProcArgTrait> = observed.iter().copied().collect();
        sorted.sort_by_key(|t| std::cmp::Reverse(role_for_trait(*t).map_or(0, role_rank)));
        let mut best: Option<&'static str> = None;
        for t in &sorted {
            notes.push(evidence(&param.name, *t));
            for key in traits_for(*t) {
                trait_keys.insert(key);
            }
            if let Some(role) = role_for_trait(*t)
                && role != "Value"
                && best.is_none_or(|current| role_rank(role) > role_rank(current))
            {
                best = Some(role);
            }
        }
        if let Some(role) = best {
            roles.push(json!({ "index": index, "role": role }));
        }
    }
    if !roles.is_empty() {
        d.insert("arg_roles".into(), Value::Array(roles));
    }
    if !trait_keys.is_empty() {
        d.insert(
            "traits".into(),
            Value::Array(trait_keys.iter().map(|k| json!(k)).collect()),
        );
    }
}

/// Attach the package gate, the synopsis, and the hover text.
fn describe(
    d: &mut Draft,
    proc_def: &tcl_compiler::analyser::types::ProcDef,
    name: &str,
    package: Option<&str>,
    version: Option<&str>,
    file: &str,
    notes: &mut Vec<String>,
) {
    // A proc defined by a package is only visible once that package is loaded.
    if let Some(package) = package {
        d.insert("required_package".into(), json!(package));
        notes.push(format!("gated on `package require {package}`"));
        if let Some(version) = version {
            d.insert("introduced_version".into(), json!(version));
        }
    }

    let synopsis_line = synopsis(name, &proc_def.params);
    d.insert(
        "forms".into(),
        json!([{ "kind": "Default", "synopsis": synopsis_line, "dialects": null }]),
    );

    let doc = proc_def.doc.trim();
    d.insert(
        "hover".into(),
        json!({
            "summary": doc,
            "synopsis": [synopsis_line],
            "snippet": doc,
            "source": format!("inferred from {file}"),
            "examples": "",
            "return_value": "",
        }),
    );
    if doc.is_empty() {
        notes.push("no doc comment above the proc — hover summary is empty".to_owned());
    } else {
        notes.push("hover summary taken from the proc's doc comment".to_owned());
    }
}

/// Analyse `files` as one package and produce a draft per procedure.
///
/// `dialect` is a registry dialect name (`"tcl9.0"`, `"f5-irules"`, …); it
/// selects the lexer configuration and command registry the analyser runs
/// with, so a package written for an older Tcl is analysed as that Tcl.
#[must_use]
pub fn import_package(files: &[SourceFile], dialect: &str) -> Import {
    let mut out = Import::default();
    // Deduplicate by qualified name: a package that sources a file twice, or
    // redefines a proc in a later file, should yield one draft — the last
    // definition wins, matching what the interpreter would end up with.
    let mut by_name: BTreeMap<String, Inferred> = BTreeMap::new();

    for file in files {
        let mut analyser = Analyser::new();
        analyser.deep_param_traits = true;
        let result = analyser.analyse(&file.text, dialect);

        if out.package.is_none()
            && let Some(provide) = result.package_provides.first()
        {
            out.package = Some(provide.name.clone());
            out.version.clone_from(&provide.version);
        }

        if result.all_procs.is_empty() {
            out.warnings
                .push(format!("{}: no `proc` definitions found", file.name));
            continue;
        }

        for proc_def in result.all_procs.values() {
            let inferred = draft_for_proc(proc_def, None, None, &file.name);
            by_name.insert(proc_def.qualified_name.clone(), inferred);
        }
    }

    // The package gate is only known once every file has been read, so it is
    // applied here rather than per file.
    for mut inferred in by_name.into_values() {
        if let Some(package) = &out.package {
            inferred
                .draft
                .insert("required_package".into(), json!(package));
            inferred
                .notes
                .push(format!("gated on `package require {package}`"));
            if let Some(version) = &out.version {
                inferred
                    .draft
                    .insert("introduced_version".into(), json!(version));
            }
        }
        out.commands.push(inferred);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> Vec<SourceFile> {
        vec![SourceFile {
            name: "pkg.tcl".to_owned(),
            text: text.to_owned(),
        }]
    }

    fn find<'a>(import: &'a Import, name: &str) -> &'a Inferred {
        import
            .commands
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "no draft for {name} in {:?}",
                    import.commands.iter().map(|c| &c.name).collect::<Vec<_>>()
                )
            })
    }

    #[test]
    fn infers_arity_from_the_parameter_list() {
        let import = import_package(
            &file("proc greet {who {greeting hello}} { return \"$greeting $who\" }"),
            "tcl9.0",
        );
        let greet = find(&import, "greet");
        assert_eq!(greet.draft["arity"]["min"], json!(1));
        assert_eq!(greet.draft["arity"]["max"], json!(2));
    }

    #[test]
    fn a_required_parameter_after_a_defaulted_one_raises_the_minimum() {
        // Tcl binds positionally: `p 1` cannot fill `b` while skipping `a`'s
        // default, so both arguments are required. Counting non-default
        // parameters would wrongly say one.
        let import = import_package(&file("proc p {{a A} b} { return $b }"), "tcl9.0");
        let p = find(&import, "p");
        assert_eq!(p.draft["arity"]["min"], json!(2));
        assert_eq!(p.draft["arity"]["max"], json!(2));
    }

    #[test]
    fn args_is_only_variadic_when_it_is_last() {
        // `args` before another parameter is an ordinary required parameter.
        let import = import_package(&file("proc f {args value} { return $value }"), "tcl9.0");
        let f = find(&import, "f");
        assert_eq!(f.draft["arity"]["min"], json!(2));
        assert_eq!(f.draft["arity"]["max"], json!(2));
        assert_eq!(
            f.draft["forms"][0]["synopsis"],
            json!("f args value"),
            "a non-trailing `args` must not render as variadic"
        );
    }

    #[test]
    fn a_trailing_args_makes_the_command_variadic() {
        let import = import_package(
            &file("proc collect {first args} { return $args }"),
            "tcl9.0",
        );
        let collect = find(&import, "collect");
        assert_eq!(collect.draft["arity"]["min"], json!(1));
        assert_eq!(collect.draft["arity"]["max"], Value::Null);
    }

    #[test]
    fn an_upvar_written_parameter_becomes_a_var_write_role() {
        let import = import_package(
            &file("proc store {varName value} { upvar 1 $varName v; set v $value }"),
            "tcl9.0",
        );
        let store = find(&import, "store");
        let roles = store.draft["arg_roles"].as_array().expect("roles inferred");
        assert!(
            roles
                .iter()
                .any(|r| r["index"] == json!(0) && r["role"] == json!("VarWrite")),
            "expected a VarWrite role at index 0, got {roles:?}"
        );
        assert!(
            store.notes.iter().any(|n| n.contains("upvar")),
            "the evidence should name the upvar: {:?}",
            store.notes
        );
    }

    #[test]
    fn an_evaluated_parameter_becomes_a_body_role_and_a_barrier() {
        let import = import_package(&file("proc run {script} { uplevel 1 $script }"), "tcl9.0");
        let run = find(&import, "run");
        let roles = run.draft["arg_roles"].as_array().expect("roles inferred");
        assert!(
            roles
                .iter()
                .any(|r| r["index"] == json!(0) && r["role"] == json!("Body")),
            "expected a Body role at index 0, got {roles:?}"
        );
        let traits = run.draft["traits"].as_array().expect("traits inferred");
        assert!(traits.contains(&json!("EVALUATES_CODE")), "{traits:?}");
    }

    #[test]
    fn the_synopsis_marks_optional_and_variadic_parameters() {
        let import = import_package(&file("proc f {a {b 1} args} { return $a }"), "tcl9.0");
        let f = find(&import, "f");
        assert_eq!(f.draft["forms"][0]["synopsis"], json!("f a ?b? ?arg ...?"));
    }

    #[test]
    fn package_provide_gates_every_command_it_defines() {
        let import = import_package(
            &file("package provide mypkg 1.2\nproc mypkg::go {} { return 1 }"),
            "tcl9.0",
        );
        assert_eq!(import.package.as_deref(), Some("mypkg"));
        let go = find(&import, "mypkg::go");
        assert_eq!(go.draft["required_package"], json!("mypkg"));
        assert_eq!(go.draft["introduced_version"], json!("1.2"));
    }

    #[test]
    fn each_note_is_recorded_once() {
        let import = import_package(&file("proc f {a} { return $a }"), "tcl9.0");
        let f = find(&import, "f");
        let mut seen = f.notes.clone();
        seen.sort();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "duplicated evidence: {:?}", f.notes);
    }

    #[test]
    fn a_file_with_no_procs_is_reported_rather_than_dropped() {
        let import = import_package(&file("set x 1"), "tcl9.0");
        assert!(import.commands.is_empty());
        assert!(import.warnings.iter().any(|w| w.contains("no `proc`")));
    }
}
