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

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_compiler::command_binding::SourceCommandSlotPresence;

fn analyse(source: &str) -> AnalysisResult {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    Analyser::new().analyse(source, "jim")
}

fn presence(prefix: &str, name: &str) -> SourceCommandSlotPresence {
    let source = format!("{prefix}; missing_after_class");
    let result = analyse(&source);
    result
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref()
        .diagnostic_command_slot_presence_at(
            name,
            u32::try_from(source.rfind("missing_after_class").unwrap()).unwrap(),
        )
}

#[test]
fn empty_base_class_retains_every_stock_publication_without_native_entry() {
    for name in [
        "C",
        "C new",
        "C finalize",
        "C method",
        "C vars",
        "C classvars",
        "C classname",
        "C methods",
        "C defaultconstructor",
        "C constructor",
        "C destroy",
        "C get",
        "C eval",
    ] {
        assert_eq!(
            presence("class C {x 0}", name),
            SourceCommandSlotPresence::present(),
            "{name}"
        );
    }
}

#[test]
fn earlier_unknown_and_changed_dependencies_do_not_recover_a_stock_world() {
    for source in [
        "eval [gets stdin]; class C {x 0}",
        "proc proc args {}; class C {x 0}",
        "class Base {x 0}; class Other {z 0}; class C {Base Other} {y 0}",
    ] {
        assert_ne!(
            presence(source, "C vars"),
            SourceCommandSlotPresence::present(),
            "{source}"
        );
    }
}

#[test]
fn fresh_stock_method_retains_original_source_publication_without_entry() {
    let source = "class C {x 0}; C method m {arg} {missing_jim_method $arg $self $x}";
    assert_eq!(
        presence(source, "C m"),
        SourceCommandSlotPresence::present()
    );
    let result = analyse(source);
    let class = result.all_classes.values().find(|c| c.name == "C").unwrap();
    let method = &class.methods["m"];
    assert_eq!(
        &source[tcl_lexer::word_span_at(source, method.body_span).as_range()],
        "{missing_jim_method $arg $self $x}"
    );
}

#[test]
fn stock_method_publication_keeps_missing_and_replaced_owners_opaque() {
    for source in [
        "class C {}; proc proc args {}; C method m {} {}",
        "class C {}; proc {C method} args {}; C method m {} {}",
        "class C {}; proc C args {}; C method m {} {}",
        "class Base {}; class Other {}; class C {Base Other} {}; C method m {} {}",
        "eval [gets stdin]; class C {}; C method m {} {}",
        "class C {}; C method m {} {}; C method m {} {}",
    ] {
        assert_ne!(
            presence(source, "C m"),
            SourceCommandSlotPresence::present(),
            "{source}"
        );
    }
}

#[test]
fn one_stock_parent_retains_alias_overwrite_allocations_and_variable_owners() {
    let source = "class Base {x 0}; class C Base {y 0}";
    for name in ["C vars", "C baseclass", "C constructor"] {
        assert_eq!(
            presence(source, name),
            SourceCommandSlotPresence::present(),
            "{name}"
        );
    }
    let result = analyse(source);
    let class = result.all_classes.values().find(|c| c.name == "C").unwrap();
    assert_eq!(class.variables, ["x", "y"]);
}

#[test]
fn former_single_base_method_refusal_has_its_original_publication_owner() {
    let source = "class Base {}; class C Base {}; C method m {} {}";
    assert_eq!(
        presence(source, "C m"),
        SourceCommandSlotPresence::present()
    );
    let result = analyse(source);
    assert!(
        result
            .all_classes
            .values()
            .find(|c| c.name == "C")
            .unwrap()
            .methods
            .contains_key("m")
    );
}

#[test]
fn extra_parent_commands_and_changed_parent_helpers_keep_inheritance_opaque() {
    for source in [
        "class Base {x 0}; proc {Base foreign} {} {}; class C Base {y 0}",
        "class Base {x 0}; proc {Base vars} {} {}; class C Base {y 0}",
        "class Base {x 0}; eval [gets stdin]; class C Base {y 0}",
    ] {
        assert_ne!(
            presence(source, "C vars"),
            SourceCommandSlotPresence::present(),
            "{source}"
        );
    }
}
