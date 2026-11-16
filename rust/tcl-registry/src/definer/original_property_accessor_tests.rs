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
fn original_property_accessor_layout_keeps_selector_names_and_values_separate() {
    // Implementation contract: naming.tcloo.original-property-accessor-source-advice
    // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let recipe = tcl_syntax::naming::NativeNameProtocol::C(version);
        let layout = |count| {
            TCLOO_CONFIGURABLE_GRAMMAR
                .source_property_accessor_layout(b"configure", recipe, count)
                .unwrap()
        };
        assert_eq!(layout(0).mode(), SourcePropertyAccessorMode::ListReadable);
        assert_eq!(layout(0).property_ordinals().count(), 0);
        assert_eq!(layout(1).mode(), SourcePropertyAccessorMode::Read);
        assert_eq!(layout(1).property_ordinals().collect::<Vec<_>>(), vec![0]);
        assert_eq!(layout(4).mode(), SourcePropertyAccessorMode::WritePairs);
        assert_eq!(
            layout(4).property_ordinals().collect::<Vec<_>>(),
            vec![0, 2]
        );
        let advice = TCLOO_CONFIGURABLE_GRAMMAR.source_property_accessor_advice(recipe);
        assert_eq!(
            advice.completion_name_role(b"configure", 0, 0),
            Some(SourcePropertyCompletionRole::ReadOrWrite)
        );
        assert_eq!(
            advice.completion_name_role(b"configure", 0, 1),
            Some(SourcePropertyCompletionRole::Readable)
        );
        assert_eq!(
            advice.completion_name_role(b"configure", 2, 2),
            Some(SourcePropertyCompletionRole::Writable)
        );
        assert_eq!(
            advice.completion_name_role(b"configure", 2, 3),
            Some(SourcePropertyCompletionRole::Writable)
        );
        assert!(advice.completion_name_role(b"configure", 1, 2).is_none());
        assert!(advice.completion_name_role(b"configure", 4, 2).is_none());
        assert!(
            TCLOO_CONFIGURABLE_GRAMMAR
                .source_property_accessor_layout(b"configure", recipe, 3)
                .is_none()
        );
        for selector in [b"cget".as_slice(), b"config", b"configure\0ignored"] {
            assert!(
                TCLOO_CONFIGURABLE_GRAMMAR
                    .source_property_accessor_layout(selector, recipe, 1)
                    .is_none()
            );
        }
        assert!(
            TCLOO_GRAMMAR
                .source_property_accessor_layout(b"configure", recipe, 1)
                .is_none()
        );
        assert!(
            SNIT_GRAMMAR
                .source_property_accessor_layout(b"configure", recipe, 1)
                .is_none()
        );
    }
    for recipe in [
        tcl_syntax::naming::NativeNameProtocol::C(TclVersion::V8_6),
        tcl_syntax::naming::NativeNameProtocol::Jim084,
    ] {
        assert!(
            TCLOO_CONFIGURABLE_GRAMMAR
                .source_property_accessor_layout(b"configure", recipe, 1)
                .is_none()
        );
    }
}
