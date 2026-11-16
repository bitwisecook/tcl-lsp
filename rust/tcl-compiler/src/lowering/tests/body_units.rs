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

//! `namespace eval` and `apply` bodies as their own units: written spans and the namespace a nested `proc` homes to.

use super::*;

/// A Windows document: the cooked `namespace eval` body folds CRLF while
/// the lowered body unit keeps original offsets, so its span must cover
/// the whole written region up to the closing brace.
#[test]
fn crlf_namespace_eval_body_unit_spans_its_written_region() {
    let source = "namespace eval ::x {\r\n    set a 1\r\n    set b 2\r\n}\r\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let module = super::super::lower_to_ir_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        Some(tcl_dialect::DialectProfile::find("tcl8.6").expect("profile")),
    );
    let open = u32::try_from(source.find('{').unwrap() + 1).unwrap();
    let close = u32::try_from(source.rfind('}').unwrap()).unwrap();
    let spans = module
        .body_units
        .values()
        .filter(|unit| unit.body_kind == crate::ir::ProcedureBodyKind::NamespaceScript)
        .map(|unit| unit.span)
        .collect::<Vec<_>>();
    assert_eq!(spans, vec![tcl_lexer::Span::new(open, close)]);
}

#[test]
fn apply_body_proc_registers_in_lambda_namespace_not_callers() {
    // `apply`'s body runs in the namespace named by the lambda — or the
    // *global* namespace when none is given — NOT the caller's namespace.
    // So a nested `proc` registers as `::helper`, never `::foo::helper`
    // (Tcl `apply` manual).
    let src = "namespace eval foo {\n\
                   \x20   apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_some(),
        "apply body proc must register in the global namespace: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::foo::helper").is_none(),
        "apply body proc must NOT inherit the caller namespace: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_body_proc_uses_explicit_lambda_namespace() {
    // A lambda whose element 2 pins a namespace runs its body there.
    let src = "apply {{} { proc helper {} { return 1 } } ::bar}\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::bar::helper").is_some(),
        "explicit lambda namespace must be honoured: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

/// TP — a relative lambda namespace element homes against the GLOBAL
/// namespace even from inside a qualified-name proc (`doc/apply.n`:
/// "interpreted relative to the global namespace even if its name does
/// not start with ::"; tclsh 9.0.4 probe `a2b_apply.tcl`). Mirrors the
/// analyser's `handle_apply_command` so IR and analysis agree.
#[test]
fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
    let src = "proc ::caller::p {} { apply {{} { proc helper {} { return 1 } } sub} }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::sub::helper").is_some(),
        "relative lambda ns must resolve against ::: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::caller::sub::helper").is_none(),
        "relative lambda ns must NOT pin against the caller: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}
