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

//! Script-parameter contracts: the frame and timing each measured wrapper
//! shape runs its script parameter in (`script-parameters-design.md`,
//! tclsh 8.4.20 to 9.1.0).

use super::*;
use crate::analyser::types::{ScriptParameterFrame as Frame, ScriptParameterTiming as Timing};

/// Infer under the Tcl 8.6 generation, whose profile also selects the
/// expression grammar the deep pass reads a braced condition with.
fn contract(params: &[&str], body: &str, deep: bool) -> HashMap<String, ScriptParameter> {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let formals = tcl_syntax::list::join_list(params);
    let source = tcl_syntax::list::join_list(["proc", "__trait_probe", &formals, body]);
    let identities = crate::realm::document_realm_bindings(
        &source,
        registry.profile().expect("profiled generation"),
        registry,
    );
    let command = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default())
        .pop()
        .unwrap();
    let script = identities
        .executed_script_for_word(command.argv.last().unwrap().span)
        .unwrap();
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, None),
        config: LexerConfig::default(),
        identities: &identities,
        executed_source: Some(script),
        original_logical_body: None,
    };
    let mut traits = if deep {
        merge_traits(
            infer_param_traits(params, body, env),
            infer_param_traits_deep(params, body, env),
        )
    } else {
        infer_param_traits(params, body, env)
    };
    take_script_parameters(&mut traits)
}

fn now(frame: Frame) -> ScriptParameter {
    ScriptParameter {
        frame,
        timing: Timing::Now,
    }
}

#[test]
fn measured_wrapper_shapes_give_their_frames() {
    for (body, expected) in [
        ("uplevel 1 $s", now(Frame::Caller(1))),
        ("uplevel 2 $s", now(Frame::Caller(2))),
        ("eval $s", now(Frame::Own)),
        ("catch $s", now(Frame::Own)),
        ("while 1 $s", now(Frame::Own)),
        ("uplevel #0 $s", now(Frame::Global)),
        (
            "after 0 $s",
            ScriptParameter {
                frame: Frame::Global,
                timing: Timing::Deferred,
            },
        ),
    ] {
        assert_eq!(
            contract(&["s"], body, false).get("s"),
            Some(&expected),
            "{body}"
        );
    }
}

#[test]
fn a_wrapper_running_the_script_inside_its_condition_is_found_by_the_deep_pass() {
    let body = r#"if {[catch {uplevel 1 $script} r]} {return "ERR $r"} else {return $r}"#;
    assert!(contract(&["label", "script"], body, false).is_empty());
    assert_eq!(
        contract(&["label", "script"], body, true).get("script"),
        Some(&now(Frame::Caller(1)))
    );
}

#[test]
fn disagreeing_or_partial_uses_abstain() {
    assert_eq!(
        contract(&["c", "s"], "if {$c} {uplevel 1 $s} else {eval $s}", true)
            .get("s")
            .map(|c| c.is_known()),
        Some(false)
    );
    // A use whose frame the body does not fix leaves no known contract,
    // whether it records an unknown use or none.
    for body in ["eval $s extra", "uplevel $lvl $s", "subst $s"] {
        assert!(
            !contract(&["lvl", "s"], body, false)
                .get("s")
                .is_some_and(|c| c.is_known()),
            "{body}"
        );
    }
}

/// A lone computed `uplevel` operand is the script only on the successful
/// path, which the pre-dispatch role layout does not select.
#[test]
fn a_lone_uplevel_operand_has_no_contract() {
    assert!(contract(&["s"], "uplevel $s", false).is_empty());
}

#[test]
fn a_script_parameter_reassigned_before_it_runs_has_no_contract() {
    assert!(contract(&["s"], "set s {puts x}; uplevel 1 $s", false).is_empty());
}

/// A lambda's `uplevel 1` names this procedure's frame, not its caller's.
#[test]
fn a_script_run_inside_a_lambda_is_not_placed_in_the_procedure() {
    assert!(
        !contract(&["s"], "apply {x {uplevel 1 $x}} $s", true)
            .get("s")
            .is_some_and(|c| c.is_known())
    );
}
