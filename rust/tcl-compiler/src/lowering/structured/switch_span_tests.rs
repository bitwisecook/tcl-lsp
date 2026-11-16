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

use crate::compilation_unit::CompilationUnit;
use crate::ir::Statement;
use tcl_registry::CommandRegistry;

#[test]
fn switch_arm_and_default_spans_point_at_body_text() {
    let src = "switch foo { foo { puts one } bar { puts two } default { puts none } }";
    let r = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(src, &r, false);

    // Locate the switch statement in the IR (top-level).
    let switch = cu
        .ir_module
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Switch { .. }))
        .expect("switch statement");

    let Statement::Switch {
        arms,
        default_body,
        default_span,
        ..
    } = switch
    else {
        panic!("expected Switch variant");
    };

    assert_eq!(arms.len(), 2);
    for arm in arms {
        let span = arm.body_span.expect("arm body_span populated");
        let text = &src[span.as_range()];
        assert!(
            text.starts_with('{') && text.ends_with('}'),
            "expected braced body, got {text:?}",
        );
        assert!(
            text.contains("puts"),
            "expected body to contain `puts`, got {text:?}",
        );
    }

    assert!(default_body.is_some());
    let dspan = default_span.expect("default_span populated");
    let dtext = &src[dspan.as_range()];
    assert!(
        dtext.contains("puts none"),
        "expected default body to contain `puts none`, got {dtext:?}",
    );
}

/// An element of a braced case list collapses its backslashes under the
/// document's grammar: before Tcl 8.6 a `\x` takes every hex digit that
/// follows and keeps the last two, so `a\x41b` is `a` and U+001B there
/// and `aAb` from 8.6.
#[test]
fn a_braced_case_list_decodes_its_elements_under_the_documents_escapes() {
    let pattern_under = |dialect: &str, source: &str| -> String {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let module = crate::lowering::lower_to_ir_with_dialect(
            source,
            registry,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry.profile(),
        );
        let Statement::Switch { arms, .. } = &module.top_level.statements[0] else {
            panic!(
                "expected a Switch, got {:?}",
                module.top_level.statements[0]
            );
        };
        arms[0].pattern.clone()
    };
    for source in [
        r#"switch -- aAb {"a\x41b" {puts hit} default {puts miss}}"#,
        r"switch -- aAb {a\x41b {puts hit} default {puts miss}}",
    ] {
        assert_eq!(pattern_under("tcl8.4", source), "a\u{1b}", "{source}");
        assert_eq!(pattern_under("tcl8.5", source), "a\u{1b}", "{source}");
        assert_eq!(pattern_under("tcl8.6", source), "aAb", "{source}");
        assert_eq!(pattern_under("tcl9.0", source), "aAb", "{source}");
    }
}
