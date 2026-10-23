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
use std::path::PathBuf;

use tcl_dialect::model::{Family, SpecProvider, SurfaceQuery, surface_admits};
use tcl_registry::ArgRole;
use tcl_registry::definer::DefinerFamily;
use tcl_registry::model::resolve_environment;
use tcl_registry::spec::CommandSpec;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};

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
    ("proc", "0.76", "0.84"),
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

/// The pack commands the registry also compiles in as native Jim rows, which
/// carry the execution contract and supersede the pack's row.
const NATIVE_JIM_ROWS: &[&str] = &["alias", "exists"];

/// A `jim` document resolves every command the pack declares: the pack's own
/// row, or for [`NATIVE_JIM_ROWS`] the native Jim row; never Tcl's.
#[test]
fn a_jim_document_resolves_every_declared_command() {
    let generation = jim();
    for spec in declared() {
        let resolved = generation
            .resolve_command(spec.name)
            .unwrap_or_else(|| panic!("{} resolves for a jim document", spec.name));
        if NATIVE_JIM_ROWS.contains(&spec.name) {
            assert!(
                !std::ptr::eq(resolved, spec)
                    && own_family(resolved)
                    && resolved.native_compilation.is_some(),
                "{}: the native jim row is the one selected",
                spec.name
            );
            continue;
        }
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

/// `class` carries the Jim class grammar: one declared member, a `new`
/// manufacturer, an open member set and `self` in every method body. Its
/// second and third words are a base list and a variable dictionary, so no
/// word is a script to walk.
#[test]
fn class_carries_the_jim_class_grammar_and_no_body_word() {
    let specs = declared();
    let class = specs
        .iter()
        .find(|spec| spec.name == "class")
        .expect("class is declared");
    let grammar = class.definition_body.expect("a definition grammar");
    assert_eq!(grammar.family, DefinerFamily::JimClass);
    assert!(grammar.member("method").is_some());
    assert!(grammar.manufacturer("new").is_some());
    assert!(grammar.dynamic_method_dispatch);
    assert_eq!(grammar.implicit_vars, ["self"]);
    assert_eq!(grammar.implicit_receiver_vars, ["self"]);
    assert!(grammar.is_implicit_receiver_variable("self"));

    let generation = jim();
    let store = generation.commands();
    for words in [&["Point", "{x 0}"][..], &["Point", "Base", "{x 0}"][..]] {
        assert!(
            store
                .arg_indices_for_role("class", words, ArgRole::Body)
                .is_empty(),
            "{words:?}"
        );
    }
}

/// A Tcl command `jimsh` has never had resolves nowhere in a `jim` document:
/// the assembled generation and the point query the analyser resolves a
/// written head with both apply the roster, and both still resolve a command
/// the roster lists.
#[test]
fn both_resolution_paths_apply_the_roster() {
    let generation = jim();
    let store = generation.commands();
    for absent in ["coroutine", "yield", "trace", "chan", "case"] {
        assert!(
            generation.resolve_command(absent).is_none(),
            "{absent}: the assembled generation"
        );
        assert!(
            generation.context().resolve_spec(store, absent).is_none(),
            "{absent}: the point query"
        );
    }
    for present in ["set", "lmap", "dict", "proc", "loop"] {
        assert!(
            generation.resolve_command(present).is_some(),
            "{present}: the assembled generation"
        );
        assert!(
            generation.context().resolve_spec(store, present).is_some(),
            "{present}: the point query"
        );
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

/// Heads only `jimsh` has, which no other shipped surface declares under the
/// same name, so a store either resolves each to its jim row or not at all.
const JIM_ONLY: &[&str] = &[
    "local",
    "upcall",
    "stackdump",
    "getref",
    "setref",
    "ref",
    "finalize",
    "curry",
    "defer",
    "xtrace",
    "lsubst",
];

/// A pack set declaring one command no shipped surface has, so installing it
/// builds a pack-carrying store rather than returning the plain one.
fn a_pack_set() -> tcl_spectcl::PackSet {
    let pack = PackFile {
        tier: Tier::Bundled,
        path: PathBuf::from("probe.tclspec"),
        origin: Origin::Bundled,
        dependency_tier: None,
    };
    let source = "speclib probe 2.0 {\n    command probe::hello {\n        arity 0\n    }\n}\n";
    tcl_spectcl::pack::load_in_memory(vec![(pack, source.to_owned())])
}

/// A pack set installed for a `jim` profile layers over Jim's own surface: the
/// pack-carrying store holds every declared command beside the pack's, and
/// resolves each Jim-only head to the row the `jim` generation resolves it to,
/// exactly as the store without packs does. A `jim` generation asked for the
/// set's key reads that same store.
#[test]
fn a_pack_carrying_jim_store_keeps_jims_own_surface() {
    let profile = tcl_spectcl::environment::profile_for_dialect("jim");
    let packs = a_pack_set();
    assert!(!packs.is_empty(), "the probe pack loads");
    assert_ne!(packs.key, 0, "a loaded set has a content key");
    let with_packs = tcl_spectcl::install::registry_with_packs(profile, &packs);
    let without =
        tcl_spectcl::install::registry_with_packs(profile, &tcl_spectcl::PackSet::default());
    assert!(
        with_packs.get("probe::hello").is_some(),
        "the pack's own command is installed"
    );
    for spec in declared() {
        assert!(
            with_packs
                .specs(spec.name)
                .iter()
                .any(|held| std::ptr::eq(*held, spec)),
            "{}: the pack-carrying store holds the jim row",
            spec.name
        );
    }
    let generation = jim();
    for head in JIM_ONLY {
        let expected = generation
            .resolve_command(head)
            .unwrap_or_else(|| panic!("{head} resolves for a jim document"));
        for (label, store) in [("with packs", &with_packs), ("without packs", &without)] {
            assert!(store.has_command_in_this_dialect(head), "{head}: {label}");
            let held = store
                .get(head)
                .unwrap_or_else(|| panic!("{head}: unknown {label}"));
            assert!(
                std::ptr::eq(held, expected),
                "{head}: {label} selects the jim row"
            );
        }
    }
    // A `jim` analysis handed the set's key reads Jim's own pack store.
    let analysed = resolve_environment("jim")
        .context_registry(&tcl_registry::model::KeyedVersions::default(), packs.key)
        .expect("a jim generation finds the pack set installed for jim");
    assert!(
        std::sync::Arc::ptr_eq(analysed.commands(), &with_packs),
        "the generation reads the store the jim profile was handed"
    );
    assert!(
        analysed.commands().get("probe::hello").is_some(),
        "the generation carries the pack's command"
    );
    for head in JIM_ONLY {
        let resolved = analysed
            .resolve_command(head)
            .unwrap_or_else(|| panic!("{head} resolves in the pack-carrying generation"));
        assert!(
            std::ptr::eq(resolved, generation.resolve_command(head).expect("jim row")),
            "{head}: the generation with packs selects the jim row"
        );
    }
    let tcl = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    for head in JIM_ONLY {
        assert!(
            tcl.get(head).is_none(),
            "{head} stays out of a Tcl 8.6 store"
        );
    }
}
