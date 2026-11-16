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

//! Lifecycle flags on every gateable row, impossible orderings and containment, and arity and stamp windows.

use super::*;
use tcl_registry::arity::Arity;
use tcl_registry::arity::ArityWindow;
use tcl_registry::deprecation::DeprecationFixHook;
use tcl_registry::deprecation::DeprecationFixSafety;
use tcl_registry::hooks::CodegenHookId;
use tcl_registry::hooks::InlineCodegenHookId;
use tcl_registry::intrinsic::IntrinsicId;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::stamp_window::StampWindow;

/// Every level the registry can gate reads the same three flags, so one
/// pack exercises the whole 1.1 vocabulary at once.
#[test]
fn every_gateable_row_reads_the_three_lifecycle_flags() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             values probe-codes {\n\
               value fast -detail {Quick path.} -min-tcl tcl8.6 -introduced 1.2 \
                 -deprecated 1.6 -retired 2.0\n\
             }\n\
             command demo {\n\
               introduced_version 1.0\n\
               form Default {demo ?-x? word} -introduced 1.1 -retired 2.0\n\
               side_effect FileIo -reads -introduced 1.1\n\
               option_conflict {-a -b} -introduced 1.1 -retired 2.0\n\
               arg 0 -values-from probe-codes\n\
               versioned_arg_value 0 fast -introduced 1.2 -retired 2.0\n\
               subcommand run {\n\
                 arity 1\n\
                 sub_subcommand now -detail {Right away.} -introduced 1.3 -deprecated 1.9\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;

    let form = spec.forms.first().expect("the form row loads");
    assert_eq!(form.lifecycle.introduced, Some("1.1"));
    assert_eq!(form.lifecycle.retired, Some("2.0"));

    let effect = spec
        .side_effects
        .lower_bound()
        .first()
        .expect("the side_effect loads");
    assert_eq!(effect.lifecycle.introduced, Some("1.1"));

    let constraint = spec
        .option_relations
        .first()
        .expect("the option_conflict loads");
    assert_eq!(constraint.lifecycle.introduced, Some("1.1"));
    assert_eq!(constraint.lifecycle.retired, Some("2.0"));

    let value = spec.arg_values[0].1[0];
    assert_eq!(value.min_tcl, Some(TclVersion::V8_6));
    assert_eq!(value.lifecycle.introduced, Some("1.2"));
    assert_eq!(value.lifecycle.deprecated, Some("1.6"));
    assert_eq!(value.lifecycle.retired, Some("2.0"));

    let gate = spec
        .versioned_arg_values
        .first()
        .expect("a command-level versioned_arg_value loads");
    assert_eq!((gate.index, gate.value), (0, "fast"));
    assert_eq!(gate.lifecycle.introduced, Some("1.2"));

    let row = spec.subcommands[0].sub_subcommands[0];
    assert_eq!(row.lifecycle.introduced, Some("1.3"));
    assert_eq!(row.lifecycle.deprecated, Some("1.9"));
}

/// The data form of the option's quick-fix hook, read by the same flag
/// reader as the command-level `deprecation_fix` statement.
#[test]
fn an_option_row_reads_a_deprecation_fix_block() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               option -old -deprecated 1.4 -deprecation-fix {-replace -new \
                 -description {Renamed in 1.4.} -safety SemanticsEquivalent}\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let option = &pack.command("demo").expect("demo loads").spec.options[0];
    assert_eq!(option.lifecycle.deprecated, Some("1.4"));
    match option.lifecycle.deprecation_fix {
        Some(DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        }) => {
            assert_eq!(replacement, "-new");
            assert_eq!(description, "Renamed in 1.4.");
            assert_eq!(safety, DeprecationFixSafety::SemanticsEquivalent);
        }
        other => panic!("expected a matched-word replacement, got {other:?}"),
    }
}

/// An impossible ordering drops the lifecycle and keeps the entity: a pack
/// is never refused for a fact it got wrong.
#[test]
fn an_impossible_lifecycle_is_dropped_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               option -x -introduced 2.0 -retired 1.0\n\
               form Default {demo} -deprecated 2.0 -retired 1.0\n\
             }\n\
             }",
    );
    let spec = pack.command("demo").expect("demo still loads").spec;
    assert_eq!(spec.options[0].name, "-x");
    assert!(spec.options[0].lifecycle.is_unspecified());
    assert!(spec.forms[0].lifecycle.is_unspecified());
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        said("option `-x`: retired release predates the introducing release; lifecycle dropped"),
        "{:?}",
        pack.notices
    );
    assert!(
        said("retired release predates the deprecating release"),
        "{:?}",
        pack.notices
    );
}

/// Containment is notice-only: the declaration stays, and the author is
/// told it claims availability its parent does not have.
#[test]
fn a_child_reaching_outside_its_parent_is_a_notice_not_a_drop() {
    let pack = evaluate_pack(
        "speclib probe 1.1 {\n\
             command demo {\n\
               introduced_version 1.5\n\
               retired_version 3.0\n\
               option -early -introduced 1.0\n\
               subcommand run { arity 1 ; retired_version 4.0 }\n\
             }\n\
             }",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.options[0].lifecycle.introduced, Some("1.0"));
    assert_eq!(spec.subcommands[0].lifecycle.retired, Some("4.0"));
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        said("option `-early`: introduced 1.0 reaches outside"),
        "{:?}",
        pack.notices
    );
    assert!(
        said("subcommand `run`: retired 4.0 reaches outside"),
        "{:?}",
        pack.notices
    );
}

/// An `arity` row without lifecycle flags is the command's plain arity,
/// exactly as before 1.2; one *with* them is a window, and the two
/// coexist — the plain row stays the fallback for a floor no window
/// covers.
///
/// Consecutive windows are written **closed**: a window with no
/// `-retired` never ends, so "two arguments from 3.0, three from 5.0" is
/// spelled with the first retiring where the second begins. Leaving it
/// open would overlap, which is what
/// `overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped`
/// covers.
#[test]
fn a_gated_arity_row_becomes_a_window_and_leaves_the_plain_arity_alone() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0 -retired 5.0\n \
             arity 3 -introduced 5.0\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.arity, Arity::exact(1), "the ungated row is the arity");
    assert_eq!(spec.arity_windows.len(), 2, "{:?}", spec.arity_windows);
    assert_eq!(spec.arity_windows[0].arity, Arity::exact(2));
    assert_eq!(spec.arity_windows[0].lifecycle.introduced, Some("3.0"));
    assert_eq!(spec.arity_windows[1].arity, Arity::exact(3));

    // The selection contract the windows exist for.
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("3.0")).map(|w| w.arity),
        Some(Arity::exact(2))
    );
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("5.0")).map(|w| w.arity),
        Some(Arity::exact(3))
    );
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("1.0")),
        None,
        "below every window, the plain arity is the fallback"
    );
}

#[test]
fn positional_arity_preserves_scope_and_release_selection() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n command demo {\n \
             arity 0..1 -positionals\n \
             arity 0..2 -positionals -introduced 3.0\n \
             option -safe\n \
             subcommand child { arity 1.. -positionals -step 2 -also 0 }\n \
             refine Alternate { arity 2 -positionals }\n \
             }\n command ordinary { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").unwrap().spec;
    assert_eq!(spec.arity, Arity::new(0, 1).with_positionals());
    assert_eq!(
        ArityWindow::select(spec.arity_windows, Some("3.0")).map(|row| row.arity),
        Some(Arity::new(0, 2).with_positionals())
    );
    assert_eq!(
        spec.subcommand("child").unwrap().arity,
        Arity::stepped(1, Arity::UNLIMITED, 2)
            .with_also_exact(0)
            .with_positionals()
    );
    assert_eq!(
        spec.command_forms[0].arity,
        Arity::exact(2).with_positionals()
    );
    assert_eq!(
        pack.command("ordinary").unwrap().spec.arity,
        Arity::exact(1)
    );
    let older = evaluate_pack("speclib probe 2.0 { command demo { arity 0..1 -positionals } }");
    assert_eq!(older.command("demo").unwrap().spec.arity, spec.arity);
    assert!(older.notices.iter().any(|notice| {
        notice.message.contains("-positionals") && notice.message.contains("2.1")
    }));
}

/// Two windows covering one release would make the selected signature
/// depend on declaration order. The pack keeps the first and is told.
#[test]
fn overlapping_arity_windows_draw_a_notice_and_the_later_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arity 9 -introduced 4.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.arity_windows.len(), 1, "{:?}", spec.arity_windows);
    assert_eq!(spec.arity_windows[0].arity, Arity::exact(2), "the first");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("overlaps") && n.message.contains("dropped")),
        "{:?}",
        pack.notices
    );
}

/// Windows and gated argument rows join the containment pass every other
/// lifecycle already goes through: a row that outlives the command
/// declaring it is a pack defect the author can only find if told.
#[test]
fn a_window_reaching_outside_the_command_lifecycle_is_noticed() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             introduced_version 2.0\n \
             retired_version 4.0\n \
             arity 1\n \
             arity 2 -introduced 5.0\n \
             }\n}",
    );
    let said = |needle: &str| pack.notices.iter().any(|n| n.message.contains(needle));
    assert!(said("arity window"), "{:?}", pack.notices);
}

/// A stamp and its three releases, as the tests read a window.
type StampRow<T> = (
    T,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
);

fn stamp_rows<T: Copy>(windows: &[StampWindow<T>]) -> Vec<StampRow<T>> {
    windows
        .iter()
        .map(|window| {
            (
                window.value,
                window.lifecycle.introduced,
                window.lifecycle.deprecated,
                window.lifecycle.retired,
            )
        })
        .collect()
}

/// A stamp statement that ends in lifecycle flags is one window of the
/// stamp, and the plain statement beside it stays the stamp for every
/// release no window covers (`SpecTcl` 2.2). The same at command and
/// subcommand scope.
#[test]
fn stamp_windows_load_beside_the_plain_stamp() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign\n \
             codegen_hook -native Llength -introduced 9.0\n \
             inline_codegen_hook -native Expr -retired 9.0\n \
             semantic_operation {Intrinsic StringLength} -introduced 9.0 -deprecated 9.1\n \
             subcommand get {\n \
             codegen_hook -native Dict -introduced 9.0\n \
             inline_codegen_hook -native Expr -introduced 8.6 -retired 9.0\n \
             semantic_operation {Intrinsic Concat} -introduced 9.0\n \
             }\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.codegen_hook, Some(CodegenHookId::Lassign));
    assert_eq!(
        stamp_rows(spec.codegen_hook_windows),
        [(CodegenHookId::Llength, Some("9.0"), None, None)]
    );
    assert_eq!(spec.inline_codegen_hook, None);
    assert_eq!(
        stamp_rows(spec.inline_codegen_hook_windows),
        [(InlineCodegenHookId::Expr, None, None, Some("9.0"))]
    );
    assert_eq!(spec.semantic_operation, None);
    assert_eq!(
        stamp_rows(spec.semantic_operation_windows),
        [(
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
            Some("9.0"),
            Some("9.1"),
            None
        )]
    );
    let sub = &spec.subcommands[0];
    assert_eq!(sub.codegen_hook, None);
    assert_eq!(
        stamp_rows(sub.codegen_hook_windows),
        [(CodegenHookId::Dict, Some("9.0"), None, None)]
    );
    assert_eq!(
        stamp_rows(sub.inline_codegen_hook_windows),
        [(InlineCodegenHookId::Expr, Some("8.6"), None, Some("9.0"))]
    );
    assert_eq!(
        stamp_rows(sub.semantic_operation_windows),
        [(
            SemanticOperationId::Intrinsic(IntrinsicId::Concat),
            Some("9.0"),
            None,
            None
        )]
    );
}

/// Two windows covering one release make the stamp depend on declaration
/// order. The pack keeps the first and is told.
#[test]
fn overlapping_stamp_windows_draw_a_notice_and_the_later_one_is_dropped() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 8.6\n \
             codegen_hook -native Llength -introduced 9.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(
        stamp_rows(spec.codegen_hook_windows),
        [(CodegenHookId::Lassign, Some("8.6"), None, None)],
        "the first"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("codegen_hook window")
                && n.message.contains("overlaps")
                && n.message.contains("dropped")),
        "{:?}",
        pack.notices
    );
}

/// A rejected lifecycle comes back UNSPECIFIED, which as a window would
/// cover every release. An arity window degrades to the plain arity, which
/// only gates less; a stamp widened to every release would specialise where
/// its author never meant it to, so the row is dropped.
#[test]
fn a_stamp_window_with_an_impossible_lifecycle_is_dropped_and_not_widened() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 9.0 -retired 8.6\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.codegen_hook, None, "not the plain stamp");
    assert!(spec.codegen_hook_windows.is_empty());
    let said = |needle: &str| pack.notices.iter().any(|n| n.message.contains(needle));
    assert!(said("codegen hook window"), "{:?}", pack.notices);
    assert!(said("dropped"), "{:?}", pack.notices);
}

/// A form is already a shape the call's arguments pick out and has no window
/// list, so a gated stamp on one is dropped with a notice — never read as an
/// ungated stamp that applies at every release.
#[test]
fn a_stamp_on_a_form_takes_no_window() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n arity 0..\n \
             refine pair {\n arity 2\n codegen_hook -native Lassign -introduced 9.0\n }\n \
             refine whole {\n arity 1\n codegen_hook -native Llength\n }\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    let hook = |name: &str| {
        spec.command_forms
            .iter()
            .find(|form| form.name == name)
            .expect("the form loads")
            .codegen_hook
    };
    assert_eq!(hook("pair"), None, "the gated stamp is not applied");
    assert_eq!(
        hook("whole"),
        Some(CodegenHookId::Llength),
        "an ungated one is"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("takes no lifecycle window")),
        "{:?}",
        pack.notices
    );
}

/// Anything after a stamp's id that is not a lifecycle flag is a notice, not
/// silence: a mistyped flag would otherwise leave a window ungated.
#[test]
fn an_unknown_flag_on_a_stamp_statement_is_noticed() {
    let pack = evaluate_pack(
        "speclib probe 2.2 {\n command demo {\n \
             codegen_hook -native Lassign -introdced 9.0\n \
             }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("unknown flag") && n.message.contains("codegen_hook")),
        "{:?}",
        pack.notices
    );
}

/// The window flags are 2.2 vocabulary, noticed per site under an older
/// declaration as every newer word is.
#[test]
fn stamp_window_flags_under_an_older_declaration_draw_a_per_site_notice() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n \
             codegen_hook -native Lassign -introduced 9.0\n \
             }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("is SpecTcl 2.2 vocabulary")
                && n.message.contains("declare `speclib probe 2.2`")),
        "{:?}",
        pack.notices
    );
}

/// An impossibly-ordered window lifecycle comes back UNSPECIFIED, which
/// as a window would cover every release and shadow the well-formed ones.
/// The row keeps its shape and loses only the gate — the same degradation
/// every other rejected lifecycle takes.
#[test]
fn an_arity_window_with_an_impossible_lifecycle_falls_back_to_the_plain_arity() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n \
             arity 7 -introduced 5.0 -retired 2.0\n \
             }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert!(
        spec.arity_windows.is_empty(),
        "a rejected gate must not become an always-on window: {:?}",
        spec.arity_windows
    );
    assert_eq!(spec.arity, Arity::exact(7), "the shape survives");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("arity window")),
        "{:?}",
        pack.notices
    );
}
