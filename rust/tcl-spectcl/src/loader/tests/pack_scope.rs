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

//! Pack-scope statements: the `speclib` wrapper and statement separation, special variables, ambient packages, file extensions, `provides`, `include` and the pack source facade.

use super::*;
use tcl_registry::arity::Arity;

#[test]
fn a_pack_with_no_speclib_wrapper_loads_nothing_and_says_so() {
    let pack = evaluate_pack("command lonely { arity 1 }");
    assert!(pack.commands.is_empty());
    assert_eq!(pack.notices.len(), 1);
    assert!(pack.notices[0].message.contains("no `speclib`"));
}

#[test]
fn statement_separation_is_ordinary_tcl() {
    // `;` separates, `;#` is a trailing comment, and a bare `#` mid-line is
    // NOT one — it becomes flags on the row, all dropped with a notice.
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command demo { arity 2 # not a comment ; return_type String }\n\
             }",
    );
    let demo = pack.command("demo").expect("demo loads");
    assert_eq!(demo.spec.arity, Arity::exact(2));
    let dropped: Vec<&str> = pack
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    assert_eq!(
        dropped.len(),
        4,
        "each of `#`, `not`, `a`, `comment` drops: {dropped:?}"
    );
}

/// `special_var NAME -kind K -access A -origin O ?-dialects {…}?
/// ?-startup B?` builds a special-variable row as the shipped table
/// states one: the startup binding gates `initially_bound` (or, for a read
/// trace, `lazily_readable`) on the row's own dialects, and a row missing
/// one of its three descriptors is dropped rather than guessed.
#[test]
fn special_var_rows_build_registry_rows_and_an_incomplete_one_is_dropped() {
    use tcl_registry::{SpecialVarKind, StartupBinding, VarAccess, VarOrigin};
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n \
             special_var sim_home -kind Scalar -access ReadOnly -origin Dialect -startup Interpreter\n \
             special_var sim_opts -kind Array -access ReadWrite -origin Environment -dialects {tcl8.6}\n \
             special_var sim_lazy -kind Scalar -access ReadOnly -origin Interpreter -startup ReadTrace\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let [home, opts, lazy] = pack.special_vars.as_slice() else {
        panic!("three rows: {:?}", pack.special_vars);
    };
    assert_eq!(home.spec.name, "sim_home");
    assert_eq!(home.spec.kind, SpecialVarKind::Scalar);
    assert_eq!(home.spec.access, VarAccess::ReadOnly);
    assert_eq!(home.spec.origin, VarOrigin::Dialect);
    assert_eq!(home.spec.startup_binding, StartupBinding::Interpreter);
    let tcl86 = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "8.6",
    ));
    let tcl90 = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "9.0",
    ));
    assert!(home.spec.readable_at_startup_in(tcl86));
    assert!(home.spec.readable_at_startup_in(tcl90));
    // No `-startup`: recognised, never readable before user code; and its
    // `-dialects` gate its existence.
    assert_eq!(opts.spec.kind, SpecialVarKind::Array);
    assert!(!opts.spec.readable_at_startup_in(tcl86));
    assert!(opts.spec.available_in(tcl86));
    assert!(!opts.spec.available_in(tcl90));
    // A read trace is readable at startup, but not bound.
    assert!(lazy.spec.readable_at_startup_in(tcl90));
    assert_eq!(lazy.spec.initially_bound, []);

    let incomplete = evaluate_pack(
        "speclib probe 2.1 {\n \
             special_var sim_home -kind Scalar -origin Dialect\n \
             special_var sim_bad -kind Matrix -access ReadOnly -origin Dialect\n \
             special_var sim_nowhere -kind Scalar -access ReadOnly -origin Dialect -dialects {nosuch}\n \
             command demo { arity 1 }\n}",
    );
    assert!(
        incomplete.special_vars.is_empty(),
        "a row missing a descriptor, or naming dialects this build cannot read, is \
             dropped rather than widened: {:?}",
        incomplete.special_vars
    );
    assert!(
        incomplete
            .notices
            .iter()
            .any(|n| n.message.contains("needs `-kind`, `-access` and `-origin`")),
        "{:?}",
        incomplete.notices
    );
    assert!(
        incomplete
            .notices
            .iter()
            .any(|n| n.message.contains("-kind Matrix")),
        "{:?}",
        incomplete.notices
    );
}

/// `ambient_package NAME VERSION` names a package the pack's dialect
/// provides without a `package require`. Both words are required: a row
/// with no version would floor at nothing, which is the situation the row
/// exists to end.
#[test]
fn ambient_package_rows_load_and_an_incomplete_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk 8.6\n \
             ambient_package Itcl 4.0\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let named: Vec<(&str, &str)> = pack
        .ambient_packages
        .iter()
        .map(|row| (row.name, row.version))
        .collect();
    assert_eq!(named, vec![("Tk", "8.6"), ("Itcl", "4.0")]);

    let missing = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk\n \
             command demo { arity 1 }\n}",
    );
    assert!(
        missing.ambient_packages.is_empty(),
        "a version-less row is dropped: {:?}",
        missing.ambient_packages
    );
    assert!(
        missing
            .notices
            .iter()
            .any(|n| n.message.contains("needs the version the runtime provides")),
        "{:?}",
        missing.notices
    );

    // 1.2 vocabulary: the same per-site notice every other new word gets.
    let older = evaluate_pack(
        "speclib probe 1.1 {\n \
             ambient_package Tk 8.6\n \
             command demo { arity 1 }\n}",
    );
    assert_eq!(
        older.ambient_packages.len(),
        1,
        "the word still loads — additions never gate"
    );
    assert!(
        older.notices.iter().any(|n| n
            .message
            .contains("`ambient_package` is SpecTcl 1.2 vocabulary")),
        "{:?}",
        older.notices
    );
}

/// The version of an `ambient_package` row is a floor, so a word that is not a
/// package version is no floor: the row is dropped with a notice, and the rows
/// beside it stand.
#[test]
fn an_ambient_package_row_whose_version_is_not_a_version_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n \
             ambient_package Tk junk\n \
             ambient_package Itcl 4.0.x\n \
             ambient_package Tcllib 1.21\n \
             ambient_package Thread 2.8a1\n \
             command demo { arity 1 }\n}",
    );
    let named: Vec<(&str, &str)> = pack
        .ambient_packages
        .iter()
        .map(|row| (row.name, row.version))
        .collect();
    assert_eq!(named, vec![("Tcllib", "1.21"), ("Thread", "2.8a1")]);
    for word in ["junk", "4.0.x"] {
        let said = format!("names `{word}`, which is not a package version");
        assert!(
            pack.notices.iter().any(|n| n.message.contains(&said)),
            "{word}: {:?}",
            pack.notices
        );
    }
}

/// `ambient_package NAME VERSION -dialects {…}`'s scoping flag is
/// refused, and — the point of the test — it takes the row with it.
///
/// Dropping the flag alone would leave the claim *unscoped*, flooring
/// the package in every dialect including the ones the pack just said
/// it is absent from. That is §6.1's fail-closed rule read backwards,
/// so the row fails closed instead, and the notice names the 2.0
/// spelling that says the same thing properly.
#[test]
fn an_ambient_package_row_scoped_with_dialects_fails_closed() {
    for declared in ["1.2", "2.0"] {
        let pack = evaluate_pack(&format!(
            "speclib probe {declared} {{\n \
                 ambient_package Tk 8.6 -dialects {{tcl8.6}}\n \
                 ambient_package Itcl 4.0\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        let named: Vec<&str> = pack.ambient_packages.iter().map(|row| row.name).collect();
        assert_eq!(
            named,
            vec!["Itcl"],
            "the scoped row is dropped whole, the unscoped one is untouched ({declared})"
        );
        let scoping = pack
            .notices
            .iter()
            .find(|n| n.message.contains("-dialects"))
            .unwrap_or_else(|| panic!("a notice names the flag ({declared}): {:?}", pack.notices));
        assert_eq!(
            scoping.class,
            VocabularyClass::Semantic,
            "an availability-narrowing word is semantic ({declared})"
        );
        assert!(
            scoping
                .message
                .contains("environment NAME { ambient Tk 8.6 }"),
            "the notice names the 2.0 spelling ({declared}): {}",
            scoping.message
        );
    }
}

/// `display_name` and `file_extension` are pack-level metadata: names
/// for humans, extensions for editors, and `-dialect` routing that must
/// resolve to a canonical profile — a typo keeps the row and drops only
/// the routing, with a notice.
#[test]
fn display_name_and_file_extension_rows_load() {
    let pack = evaluate_pack(
        "speclib upfdemo 1.1 {\n             display_name {IEEE 1801 UPF}\n             file_extension .UPF -name {Unified Power Format} -dialect synopsys-eda-tcl\n             file_extension pwr -dialect no-such-dialect\n             file_extension upf -name {duplicate}\n             command demo { arity 1 }\n}",
    );
    assert_eq!(pack.display_name.as_deref(), Some("IEEE 1801 UPF"));
    assert_eq!(pack.file_extensions.len(), 2, "{:?}", pack.file_extensions);
    let upf = &pack.file_extensions[0];
    assert_eq!(upf.extension, "upf", "lower-cased, dot stripped");
    assert_eq!(upf.display_name.as_deref(), Some("Unified Power Format"));
    assert_eq!(upf.dialect, Some("synopsys-eda-tcl"));
    let pwr = &pack.file_extensions[1];
    assert_eq!(pwr.extension, "pwr");
    assert_eq!(pwr.dialect, None, "typo'd dialect drops only the routing");
    let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`-dialect no-such-dialect` names no dialect profile")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`file_extension upf` redeclared")),
        "{messages:?}"
    );
}

/// `provides` declares the pack's package trains and defaults command
/// providers; an explicit `default required_package` still wins.
#[test]
fn provides_defaults_the_command_provider() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n provides upf 1.0 2.1\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(pack.provides.len(), 1);
    assert_eq!(pack.provides[0].name, "upf");
    assert_eq!(pack.provides[0].versions, vec!["1.0", "2.1"]);
    assert_eq!(
        pack.command("demo").expect("demo").spec.required_package,
        Some("upf")
    );

    let explicit = evaluate_pack(
        "speclib probe 2.0 {\n provides upf\n \
             default required_package sdc\n command demo { arity 1 }\n}",
    );
    assert_eq!(
        explicit
            .command("demo")
            .expect("demo")
            .spec
            .required_package,
        Some("sdc"),
        "an explicit default beats the provides fallback"
    );
}

/// `co_provides` parses its predicated relation and carries it as data.
#[test]
fn co_provides_carries_the_predicated_relation() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n provides tk\n \
             co_provides Tk -requires-exact tk -when {without TK_NO_DEPRECATED}\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let relation = &pack.co_provides[0];
    assert_eq!(relation.name, "Tk");
    assert_eq!(relation.requires_exact, Some("tk"));
    assert_eq!(relation.when, Some("without TK_NO_DEPRECATED"));
}

/// `include` splices a fragment's declarations in place — provenance
/// inherited, registrations carrying the included statements — and the
/// determinism guards hold: no context, a cycle, and a path-shaped name
/// each drop the row with a semantic notice.
#[test]
fn include_splices_a_fragment_under_the_determinism_contract() {
    let fragment = "command extra {\n arity 2\n}\n";
    let context = IncludeContext::new(move |name| match name {
        "extra.tclspec-frag" => Ok(fragment.to_owned()),
        "self.tclspec-frag" => Ok("include self.tclspec-frag\n".to_owned()),
        other => Err(format!("no such fragment `{other}`")),
    });
    let context = std::rc::Rc::new(context);
    let including = |source: &str| {
        evaluate_pack_in(
            source,
            &EvalOptions::default(),
            Some(std::rc::Rc::clone(&context)),
        )
    };
    let pack = including(
        "speclib probe 2.0 {\n include extra.tclspec-frag\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(
        pack.command("extra").is_some(),
        "the fragment's command loads"
    );
    assert!(pack.command("demo").is_some());
    assert!(
        pack.registrations.iter().any(|reg| reg.arg(1) == "extra"),
        "the record carries the included statements"
    );
    assert!(
        !pack.registrations.iter().any(|reg| reg.word() == "include"),
        "…and not the include row itself"
    );

    // No context: the row drops with a semantic notice.
    let dropped = evaluate_pack("speclib probe 2.0 {\n include extra.tclspec-frag\n}");
    assert!(
        dropped.notices.iter().any(|n| {
            n.class == VocabularyClass::Semantic && n.message.contains("needs a pack search path")
        }),
        "{:?}",
        dropped.notices
    );

    // A cycle is rejected by content hash.
    let cyclic = including("speclib probe 2.0 {\n include self.tclspec-frag\n}");
    assert!(
        cyclic
            .notices
            .iter()
            .any(|n| n.message.contains("include cycle")),
        "{:?}",
        cyclic.notices
    );

    // A path-shaped name never reaches the resolver.
    let hostile = including("speclib probe 2.0 {\n include ../outside\n}");
    assert!(
        hostile
            .notices
            .iter()
            .any(|n| n.message.contains("not a plain file name")),
        "{:?}",
        hostile.notices
    );

    // An unresolvable name reports the resolver's reason.
    let missing = including("speclib probe 2.0 {\n include nowhere.tclspec-frag\n}");
    assert!(
        missing
            .notices
            .iter()
            .any(|n| n.message.contains("did not resolve")),
        "{:?}",
        missing.notices
    );
}

#[test]
fn original_pack_include_navigation_and_loader_share_name_and_body_rules() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let source = "\u{feff}speclib p 2.0 {include {shared α}; include ~; include ../outside; command c {include nested}}";
    let config = pack_source_lexer_config(tcl_lexer::SourceSegmentationPurpose::WholeInput);
    let rows = tcl_source::spectcl::pack_source_includes(source, config);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let reached = Rc::clone(&calls);
    let context = Rc::new(IncludeContext::new(move |name| {
        reached.borrow_mut().push(name.to_owned());
        Ok(String::new())
    }));
    let _pack = evaluate_pack_in(source, &EvalOptions::default(), Some(context));
    assert_eq!(
        *calls.borrow(),
        rows.iter()
            .map(|row| row.name().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(*calls.borrow(), ["shared α", "~"]);
    assert!(uses_include(source));
    assert!(!uses_include(
        "speclib p 2.0 {include from tcl into jim {a}}"
    ));
    assert!(!uses_include("speclib p 2.0 {command c {include nested}}"));
}

#[test]
fn original_pack_source_facade_separates_file_prologue_from_block_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    use tcl_lexer::SourceSegmentationPurpose::{NestedScript, WholeInput};
    let source = "\u{feff}speclib probe {1.1} {\ncommand visible {arity 0}\n\u{feff}command hidden {arity 0}\n}";
    let pack = evaluate_pack(source);
    assert!(pack.command("visible").is_some());
    assert!(pack.command("hidden").is_none());
    let (version, value) = speclib_version_span(source).expect("original file version");
    assert_eq!(&source[version], "1.1");
    assert_eq!(value, "1.1");
    assert_eq!(pack_statements(source)[0].words[0].text, "speclib");
    assert_eq!(
        statements(source, 1, FileBom::Content)[0].words[0].text,
        "\u{feff}speclib"
    );
    let whole = pack_source_lexer_config(WholeInput);
    let body = pack_source_lexer_config(NestedScript);
    assert_eq!(whole.leading_bom, LeadingBom::Skip);
    assert_eq!(body.leading_bom, LeadingBom::Content);
    assert_eq!(body, NestedScript.config(whole));
    assert_eq!(pack_source_lexer_config(WholeInput), whole);
}
