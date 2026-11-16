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

use super::*;

#[test]
fn logical_receiver_inventory_uses_actual_frame_and_refuses_unknown_members() {
    for (effect, expected) in [("", true), ("mystery;", false)] {
        let source = format!(
            "oo::class create C {{{effect} method known {{}} {{}}; method body {{}} {{my knwon}}}}"
        );
        let mut analyser = Analyser::new();
        analyser.analyse(&source, "tcl");
        assert!(analyser.result.allows_retained_logical_declaration_advice());
        let input = analyser.result.resolved_input.as_ref().unwrap();
        let image = tcl_lexer::SourceImage::document(&source);
        let cursor = u32::try_from(source.find("my knwon").unwrap()).unwrap();
        let caller = analyser
            .result
            .logical_source_member_frame_at(&image, input.lexer_config(), cursor)
            .unwrap();
        let class = analyser
            .result
            .logical_source_member_class(&caller)
            .unwrap();
        let inventory = analyser.logical_receiver_method_inventory(
            class,
            tcl_registry::definer::MethodReach::SelfDispatch,
        );
        assert_eq!(inventory.is_some(), expected, "{source}");
        if let Some((names, _)) = inventory {
            assert!(names.contains("known"));
            assert!(!names.contains("knwon"));
        }
        let mut foreign = analyser.result.clone();
        foreign
            .all_classes
            .values_mut()
            .next()
            .unwrap()
            .methods
            .clear();
        assert!(foreign.logical_source_member_class(&caller).is_none());
    }
}
