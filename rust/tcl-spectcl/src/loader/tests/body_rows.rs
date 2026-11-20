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

//! `arg` and `option` rows: the argument index bound, option taint and timing metadata, callback taint inputs, Tk geometry and the sealed row projection.

use super::*;
use tcl_registry::body_kind::BodyKind;
use tcl_registry::hover::CallbackTaintInput;
use tcl_registry::hover::OptionValue;
use tcl_registry::hover::VariableScope;
use tcl_registry::types::TclType;

#[test]
fn an_argument_index_above_the_table_width_drops() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command demo { arg 300 -role Body\n arg 2 -role Body }\n\
             }",
    );
    let demo = pack.command("demo").expect("demo loads");
    assert_eq!(demo.spec.arg_roles, &[(2, ArgRole::Body)]);
    assert!(pack.notices.iter().any(|n| n.message.contains("above the")));
}

#[test]
fn option_external_input_link_round_trips_as_taint_metadata() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             arity 1..\n \
             option -textvariable -takes variable -role VarWrite \
                 -also-role VarRead -taints-var-write -variable-scope Global\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("widget").expect("widget loads").spec;
    let option = spec
        .find_option("-textvariable", None, None)
        .expect("linked option loads");
    assert!(option.taints_var_write());
    assert_eq!(option.value_role(), Some(ArgRole::VarWrite));
    assert_eq!(option.value_also_role(), Some(ArgRole::VarRead));
    assert_eq!(option.value_variable_scope(), Some(VariableScope::Global));
}

#[test]
fn option_script_timing_is_loaded_independently_of_body_scope() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             option -command -takes script -role Body -body-kind Structural \
                 -script-timing Deferred\n \
             option -body -takes script -role Body -body-kind Structural \
                 -script-timing SameInvocation\n \
             option -remove -takes command-prefix -role CommandPrefix \
                 -script-timing ReferenceOnly\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("widget").expect("widget loads").spec;
    let callback = spec
        .find_option("-command", None, None)
        .expect("callback option loads");
    let immediate = spec
        .find_option("-body", None, None)
        .expect("immediate body option loads");
    let reference = spec
        .find_option("-remove", None, None)
        .expect("reference-only option loads");
    assert_eq!(callback.value_script_timing(), Some(ScriptTiming::Deferred));
    assert_eq!(
        immediate.value_script_timing(),
        Some(ScriptTiming::SameInvocation)
    );
    assert_eq!(
        reference.value_script_timing(),
        Some(ScriptTiming::ReferenceOnly)
    );
    let OptionValue::Takes(callback_arg) = callback.value else {
        panic!("callback takes a value")
    };
    let OptionValue::Takes(immediate_arg) = immediate.value else {
        panic!("body takes a value")
    };
    let OptionValue::Takes(reference_arg) = reference.value else {
        panic!("removal reference takes a value")
    };
    assert_eq!(callback_arg.body_kind, BodyKind::Structural);
    assert_eq!(immediate_arg.body_kind, BodyKind::Structural);
    assert_eq!(reference_arg.role, ArgRole::CommandPrefix);
}

#[test]
fn callback_taint_inputs_are_authorable_but_reject_bookkeeping_markers() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command widget {\n \
             traits DEFERS_BODY\n \
             arity 2\n \
             arg 1 -role Body\n \
             callback_taint_inputs {{1 {%A %K %W}}}\n \
             option -validatecommand -takes script -role Body -body-kind Structural \
                 -script-timing Deferred -callback-taint-inputs {%P %s %S %V}\n \
             }\n}",
    );
    let spec = &pack.command("widget").expect("widget loads").spec;
    assert_eq!(spec.callback_taint_inputs[0].0, 1);
    assert_eq!(
        spec.callback_taint_inputs[0].1,
        &[
            CallbackTaintInput::TK_EVENT_CHAR,
            CallbackTaintInput::TK_EVENT_KEYSYM,
        ]
    );
    let option = spec
        .find_option("-validatecommand", None, None)
        .expect("callback option loads");
    assert_eq!(
        option.value_callback_taint_inputs(),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ]
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("%W"))
            && pack
                .notices
                .iter()
                .any(|notice| notice.message.contains("%V")),
        "metadata substitutions must be rejected: {:?}",
        pack.notices
    );
}

#[test]
fn contradictory_option_and_callback_properties_are_not_installed() {
    let pack = evaluate_pack(
        r"speclib probe 1.2 {
 command bad {
   arg 0 -role Value
   callback_taint_inputs {{0 {%A}}}
   option -timing -takes value -role Value -script-timing Deferred
   option -input -takes script -role Body -script-timing SameInvocation \
       -callback-taint-inputs {%P}
   option -scope -takes value -role Value -variable-scope Global
   option -taint -takes variable -role VarRead -taints-var-write
   subcommand child {
     arg 0 -role Body
     callback_taint_inputs {{0 {%S}}}
   }
 }
}",
    );
    let spec = &pack.command("bad").expect("command survives").spec;
    assert!(spec.callback_taint_inputs.is_empty());
    let OptionValue::Takes(timing) = spec.options[0].value else {
        panic!("option takes a value")
    };
    assert_eq!(timing.script_timing, ScriptTiming::SameInvocation);
    let OptionValue::Takes(input) = spec.options[1].value else {
        panic!("option takes a value")
    };
    assert!(input.callback_taint_inputs.is_empty());
    let OptionValue::Takes(scope) = spec.options[2].value else {
        panic!("option takes a value")
    };
    assert_eq!(scope.variable_scope, VariableScope::CurrentFrame);
    let OptionValue::Takes(taint) = spec.options[3].value else {
        panic!("option takes a value")
    };
    assert!(!taint.taints_var_write);
    assert!(spec.subcommands[0].callback_taint_inputs.is_empty());

    for fragment in [
        "non-executable value",
        "not a deferred executable",
        "non-variable value",
        "without a VarWrite role",
        "argument 0, which is not a deferred executable position",
    ] {
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains(fragment)),
            "missing `{fragment}` notice: {:?}",
            pack.notices
        );
    }
}

#[test]
fn tk_geometry_form_and_release_metadata_loads() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command layout {\n \
             tk_geometry Exclusive -container-option -inside -direct-form \
                 -placement-subcommand arrange \
                 -release-subcommands {release unmanage}\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let geometry = pack
        .command("layout")
        .expect("layout loads")
        .spec
        .tk_geometry
        .expect("geometry loads");
    assert_eq!(
        geometry.container_policy,
        tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive
    );
    assert_eq!(geometry.container_option, Some("-inside"));
    assert!(geometry.direct_form);
    assert_eq!(geometry.placement_subcommand, Some("arrange"));
    assert_eq!(geometry.release_subcommands, ["release", "unmanage"]);
}

/// [`ArgRows::seal`] is the one projection from the authored row form to
/// the parallel slices the registry stores, so a row with every column
/// set must land in every one of the six vectors. A column added to
/// [`ArgRow`] and not projected fails here rather than vanishing.
#[test]
fn projection_carries_every_row_column() {
    let values = leak_slice(vec![ArgValue {
        value: "alnum",
        ..ArgValue::DEFAULT
    }]);
    let rows = ArgRows {
        rows: vec![ArgRow {
            index: 3,
            role: Some(ArgRole::Body),
            type_hint: Some(ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            }),
            values,
            closed: true,
            presentation: Some(ArgPresentation::InlineScript),
            appends: Some(AppendedArity::Exactly(2)),
            domain: Some(tcl_registry::hover::IntegerDomain::Range(0, 9)),
        }],
    };

    let sealed = rows.seal();

    assert_eq!(sealed.roles, vec![(3, ArgRole::Body)]);
    assert_eq!(
        sealed.types,
        vec![(
            3,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            }
        )]
    );
    assert_eq!(sealed.values, vec![(3, values)]);
    assert_eq!(sealed.closed, vec![3]);
    assert_eq!(
        sealed.presentation,
        vec![(3, ArgPresentation::InlineScript)]
    );
    assert_eq!(
        sealed.domains,
        vec![(3, tcl_registry::hover::IntegerDomain::Range(0, 9))]
    );
    assert_eq!(sealed.prefixes, vec![(3, AppendedArity::Exactly(2))]);
}

#[test]
fn argument_integer_domains_load_at_both_scopes() {
    use tcl_registry::hover::IntegerDomain;
    let pack = evaluate_pack(
        "speclib probe 2.3 {\n command demo {\n arity 0..2\n \
             arg 0 -domain {Range 0 max}\n arg 1 -domain Port\n \
             subcommand replace {\n arity 3\n arg 0 -domain {Range 0 4294967295}\n }\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("demo").expect("demo loads").spec;
    assert_eq!(
        spec.arg_domains,
        &[
            (0, IntegerDomain::Range(0, i64::MAX)),
            (1, IntegerDomain::Port)
        ]
    );
    assert_eq!(
        spec.subcommands[0].arg_domains,
        &[(0, IntegerDomain::Range(0, 4_294_967_295))]
    );
    let older = evaluate_pack("speclib probe 2.2 {\n command demo { arg 0 -domain Port }\n}");
    assert!(
        older.notices.iter().any(|n| n.message.contains("-domain")),
        "{:?}",
        older.notices
    );
}
