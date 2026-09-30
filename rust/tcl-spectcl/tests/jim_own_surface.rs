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

//! Jim's own command surface, end to end: the compiled-in pack reaches a real
//! `jim` registry generation, each command carries the release window a
//! `jimsh` build of every upstream tag 0.76-0.84 measured, and no other
//! environment sees any of it.

use std::collections::BTreeSet;

use tcl_dialect::model::{Family, SpecProvider, SurfaceQuery, surface_admits};
use tcl_registry::ArgRole;
use tcl_registry::model::resolve_environment;
use tcl_registry::spec::CommandSpec;

/// The releases on Jim's ladder.
const LADDER: &[&str] = &[
    "0.76", "0.77", "0.78", "0.79", "0.80", "0.81", "0.82", "0.83", "0.84",
];

/// Every declared command with the first and last release whose `jimsh` lists
/// it in `info commands -all`. No name has a gap.
const MEASURED: &[(&str, &str, &str)] = &[
    ("*", "0.76", "0.84"),
    ("+", "0.76", "0.84"),
    ("-", "0.76", "0.84"),
    ("/", "0.76", "0.84"),
    ("alarm", "0.76", "0.84"),
    ("alias", "0.76", "0.84"),
    ("class", "0.76", "0.84"),
    ("collect", "0.76", "0.84"),
    ("curry", "0.76", "0.84"),
    ("defer", "0.78", "0.84"),
    ("ensemble", "0.82", "0.84"),
    ("env", "0.76", "0.84"),
    ("errorInfo", "0.76", "0.84"),
    ("exists", "0.76", "0.84"),
    ("finalize", "0.76", "0.84"),
    ("function", "0.76", "0.84"),
    ("getref", "0.76", "0.84"),
    ("json::decode", "0.79", "0.84"),
    ("json::encode", "0.79", "0.84"),
    ("json::subencode", "0.81", "0.84"),
    ("kill", "0.76", "0.84"),
    ("lambda", "0.76", "0.84"),
    ("load_ssl_certs", "0.77", "0.84"),
    ("local", "0.76", "0.84"),
    ("loop", "0.76", "0.84"),
    ("lsubst", "0.84", "0.84"),
    ("os.fork", "0.76", "0.84"),
    ("os.gethostname", "0.76", "0.84"),
    ("os.getids", "0.76", "0.84"),
    ("os.umask", "0.84", "0.84"),
    ("os.uptime", "0.76", "0.84"),
    ("os.wait", "0.76", "0.77"),
    ("pack", "0.76", "0.84"),
    ("parray", "0.76", "0.84"),
    ("pipe", "0.78", "0.84"),
    ("popen", "0.76", "0.84"),
    ("rand", "0.76", "0.84"),
    ("range", "0.76", "0.84"),
    ("readdir", "0.76", "0.84"),
    ("ref", "0.76", "0.84"),
    ("setref", "0.76", "0.84"),
    ("signal", "0.76", "0.84"),
    ("sleep", "0.76", "0.84"),
    ("stackdump", "0.76", "0.84"),
    ("stacktrace", "0.76", "0.84"),
    ("stderr", "0.76", "0.84"),
    ("stdin", "0.76", "0.84"),
    ("stdout", "0.76", "0.84"),
    ("super", "0.76", "0.84"),
    ("syslog", "0.76", "0.84"),
    ("taint", "0.84", "0.84"),
    ("tcl::prefix", "0.76", "0.84"),
    ("timerate", "0.82", "0.84"),
    ("tree", "0.76", "0.84"),
    ("unpack", "0.76", "0.84"),
    ("untaint", "0.84", "0.84"),
    ("upcall", "0.76", "0.84"),
    ("wait", "0.78", "0.84"),
    ("xtrace", "0.81", "0.84"),
];

/// Names `jimsh` lists only because a library proc is public by accident, or
/// that are a stub: none is a command a script is meant to call.
const OMITTED: &[&str] = &[
    "binary::nextarg",
    "debug",
    "glob.explode",
    "glob.glob",
    "glob.globdir",
    "json::encode.",
    "json::encode.bool",
    "json::encode.list",
    "json::encode.mixed",
    "json::encode.num",
    "json::encode.obj",
    "json::encode.str",
    "lambda.finalizer",
    "tcl::autocomplete",
    "tcl::stdhint",
];

fn declared() -> Vec<&'static CommandSpec> {
    tcl_spectcl::core_surfaces::ensure();
    tcl_spectcl::core_surfaces::builtin_commands()
}

fn own_family(spec: &CommandSpec) -> bool {
    spec.surface.is_some_and(|rows| {
        rows.iter()
            .any(|row| row.provider == SpecProvider::Core(Family::Jim))
    })
}

fn jim() -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
    tcl_spectcl::core_surfaces::ensure();
    resolve_environment("jim").default_context_registry()
}

fn at(release: &str) -> SurfaceQuery<'_> {
    SurfaceQuery::core(Family::Jim, release)
}

/// The pack declares the measured names and only those.
#[test]
fn the_pack_declares_the_measured_names_and_no_others() {
    let declared: BTreeSet<&str> = declared().iter().map(|spec| spec.name).collect();
    let measured: BTreeSet<&str> = MEASURED.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(declared, measured);
    for name in OMITTED {
        assert!(!declared.contains(name), "{name} is not a script command");
    }
}

/// Every command is admitted at exactly the releases it was measured in: no
/// package gate is invented and no window is widened.
#[test]
fn every_command_carries_exactly_its_measured_window() {
    let specs = declared();
    for (name, first, last) in MEASURED {
        let spec = specs
            .iter()
            .find(|spec| spec.name == *name)
            .unwrap_or_else(|| panic!("{name} is declared"));
        let rows = spec.surface.expect("a stated window");
        assert!(
            rows.iter()
                .all(|row| row.provider == SpecProvider::Core(Family::Jim)),
            "{name}: only the jim family provides it"
        );
        assert!(
            spec.required_package.is_none(),
            "{name}: no package gates a jim command"
        );
        for release in LADDER {
            let present = release >= first && release <= last;
            assert_eq!(
                surface_admits(rows, Some(&at(release))),
                present,
                "{name} at {release}: measured {first}-{last}"
            );
        }
    }
}

/// A `jim` document resolves every command the pack declares.
#[test]
fn a_jim_document_resolves_every_declared_command() {
    let generation = jim();
    for spec in declared() {
        let resolved = generation
            .resolve_command(spec.name)
            .unwrap_or_else(|| panic!("{} resolves for a jim document", spec.name));
        assert!(
            std::ptr::eq(resolved, spec),
            "{}: the jim row is the one selected",
            spec.name
        );
    }
}

/// Nothing declared here reaches another environment: where a Tcl-family
/// environment has a command of the same name, it is that environment's own.
#[test]
fn no_other_environment_sees_jims_own_rows() {
    let jim = jim();
    for environment in [
        "tcl",
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "tk",
        "f5-tcl",
        "f5-irules",
        "f5-iapps",
        "expect",
    ] {
        let generation = resolve_environment(environment).default_context_registry();
        assert!(
            !std::sync::Arc::ptr_eq(generation.commands(), jim.commands()),
            "{environment}: shares no store with jim"
        );
        for spec in declared() {
            if let Some(resolved) = generation.resolve_command(spec.name) {
                assert!(
                    !own_family(resolved),
                    "{environment} resolves `{}` to a jim row",
                    spec.name
                );
            }
        }
    }
}

/// `tcl::prefix`, `parray` and `timerate` exist in Tcl 8.6 with another
/// shape: a `jim` document gets Jim's, a Tcl document keeps Tcl's.
#[test]
fn shared_names_resolve_to_each_documents_own_family() {
    let jim = jim();
    for name in ["tcl::prefix", "parray", "timerate"] {
        let jims = jim
            .resolve_command(name)
            .unwrap_or_else(|| panic!("{name} for jim"));
        assert!(own_family(jims), "{name}: the jim row for a jim document");

        let tcl = resolve_environment("tcl8.6").default_context_registry();
        let tcls = tcl
            .resolve_command(name)
            .unwrap_or_else(|| panic!("{name} for tcl8.6"));
        assert!(
            !own_family(tcls),
            "{name}: Tcl's own row for a Tcl document"
        );
        assert!(
            tcls.surface.is_some_and(|rows| rows
                .iter()
                .any(|row| row.provider == SpecProvider::Core(Family::Tcl))),
            "{name}: from the Tcl surface"
        );
    }
}

/// `loop`'s body is its last word, whichever arity the call has, so the
/// position that runs a script is declared at each.
#[test]
fn a_loop_body_is_declared_at_each_arity() {
    let generation = jim();
    let store = generation.commands();
    let body = |args: &[&str]| store.arg_indices_for_role("loop", args, ArgRole::Body);
    assert_eq!(body(&["i", "3", "{}"]), vec![2]);
    assert_eq!(body(&["i", "0", "3", "{}"]), vec![3]);
    assert_eq!(body(&["i", "0", "10", "2", "{}"]), vec![4]);
    let counter = |args: &[&str]| store.arg_indices_for_role("loop", args, ArgRole::VarWrite);
    assert_eq!(counter(&["i", "0", "3", "{}"]), vec![0]);
}

/// The two arities a release ladder splits `loop` on: the bare-limit form is
/// a 0.81 addition and the others span the whole ladder.
#[test]
fn the_bare_limit_loop_form_starts_at_0_81() {
    let specs = declared();
    let spec = specs
        .iter()
        .find(|spec| spec.name == "loop")
        .expect("loop is declared");
    let form = |name: &str| {
        spec.command_forms
            .iter()
            .find(|form| form.name == name)
            .unwrap_or_else(|| panic!("loop form {name}"))
    };
    let admitted = |name: &str, release: &str| {
        form(name)
            .surface
            .is_none_or(|rows| surface_admits(rows, Some(&at(release))))
    };
    assert!(!admitted("limit", "0.80"));
    assert!(admitted("limit", "0.81"));
    for release in LADDER {
        assert!(admitted("bounds", release), "{release}");
        assert!(admitted("stepped", release), "{release}");
    }
}

/// The arithmetic commands take no operator-command trait, which would hide
/// them from a family whose math operators are not command heads.
#[test]
fn arithmetic_command_forms_are_visible_to_jim() {
    let generation = jim();
    for name in ["+", "-", "*", "/"] {
        assert!(generation.resolve_command(name).is_some(), "{name}");
    }
}
