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

use crate::analyser::Analyser;

#[test]
fn original_namespace_export_inventory_keeps_opaque_patterns_clear_and_prefix_abort() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    for version in tcl_dialect::TclVersion::ALL {
        let source = r"namespace export {p\uD800}; namespace export -clear p\uD801; namespace export -clear -clear; namespace export first ::invalid never";
        let mut result = Analyser::new().analyse(source, version.dialect_name());
        let events = result
            .original_namespace_exports()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            events.iter().filter(|event| event.clears()).count(),
            2,
            "{version:?}"
        );
        assert!(
            events
                .iter()
                .any(|event| event.pattern() == Some(b"p\\uD800".as_slice())),
            "braced word keeps its literal backslash"
        );
        assert!(
            events
                .iter()
                .any(|event| event.pattern() == Some(b"p\xed\xa0\x81".as_slice())),
            "{version:?}"
        );
        assert!(
            events
                .iter()
                .any(|event| event.pattern() == Some(b"-clear".as_slice()))
        );
        assert!(
            events
                .iter()
                .any(|event| event.pattern() == Some(b"first".as_slice()))
        );
        assert!(
            !events
                .iter()
                .any(|event| event.pattern() == Some(b"never".as_slice()))
        );
        assert!(events.iter().all(|event| event.context().is_root()
            && event.site().source.source_image() == &tcl_lexer::SourceImage::document(source)));
        result.namespace_exports.clear();
        assert_eq!(
            result
                .original_namespace_exports()
                .cloned()
                .collect::<Vec<_>>(),
            events
        );
    }
}

#[test]
fn original_namespace_export_inventory_retains_unknown_operand_as_may_only() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "namespace export $unavailable";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(result.original_namespace_exports().next().is_none());
    assert_eq!(result.original_namespace_export_unknowns().len(), 1);
    let span = result.original_namespace_export_unknowns()[0];
    assert!(span.start() >= u32::try_from(source.find('$').unwrap()).unwrap());
}

#[test]
fn original_namespace_inventory_keeps_conditional_source_scopes_after_unknown_import() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "set ::auto_index(unavailable_provider) {source lib.tcl}; unavailable_provider\nnamespace eval C {proc p {} {}; namespace export p}\nnamespace eval B {namespace import ::C::*; namespace export p}\nnamespace eval A {namespace import ::B::*}\n";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let exports = result.original_namespace_exports().collect::<Vec<_>>();
    let imports = result.original_namespace_patterns().collect::<Vec<_>>();
    assert_eq!(exports.len(), 2);
    assert_eq!(imports.len(), 2);
    assert_eq!(result.original_namespace_export_unknowns(), []);
    assert!(
        exports
            .iter()
            .any(|event| event.conditional_declaration().is_some())
    );
    assert!(
        imports
            .iter()
            .any(|event| event.conditional_declaration().is_some())
    );
    for event in &exports {
        assert_eq!(
            event
                .name_input()
                .original_word_key()
                .unwrap()
                .source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
    }
    assert_eq!(
        exports
            .iter()
            .map(|event| event.context().display().unwrap())
            .collect::<Vec<_>>(),
        ["::C", "::B"]
    );
    assert_eq!(
        imports
            .iter()
            .map(|event| event.context().display().unwrap())
            .collect::<Vec<_>>(),
        ["::B", "::A"]
    );
    assert!(
        result.original_completed_command_world().is_none(),
        "source declaration geometry cannot close the actual import prelude or source mode"
    );
}

#[test]
fn original_namespace_inventory_does_not_borrow_a_parent_for_unknown_source_scope() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "namespace eval $unknown {namespace export p; namespace import ::C::*}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(result.original_namespace_exports().next().is_none());
    assert!(result.original_namespace_patterns().next().is_none());
    assert_ne!(result.original_namespace_export_unknowns(), []);
    assert!(result.original_completed_command_world().is_none());
}

#[test]
fn original_namespace_inventory_does_not_extend_static_scope_through_an_unentered_callable() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    let source = "proc unused {} {namespace eval B {namespace export p; namespace import ::C::*}}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        result
            .original_namespace_exports()
            .all(|event| event.conditional_declaration().is_none())
    );
    assert!(
        result
            .original_namespace_patterns()
            .all(|event| event.conditional_declaration().is_none())
    );
}
