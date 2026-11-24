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

//! Shared naming keys over positioned source allocations.

use tcl_compiler::command_binding::{CommandRefInterner, SourceCommandBindings};
use tcl_core_types::{Answer, CommandRef};

fn reference(
    bindings: &SourceCommandBindings,
    interner: &mut CommandRefInterner,
    source: &str,
    marker: &str,
) -> Answer<CommandRef> {
    let offset = u32::try_from(source.find(marker).unwrap() + 1).unwrap();
    bindings
        .invocation_at_source("", offset)
        .original_evaluated_command_reference()
        .expect("positioned original command")
        .command_ref(interner)
}

#[test]
fn analysis_command_refs_survive_rename_and_split_on_recreation() {
    let source =
        "proc p {} {return p}\np\nrename p q\nq\nproc p {} {return newp}\np; # recreated\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let bindings =
        SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
    let mut interner = CommandRefInterner::new(bindings.source_origin().unwrap());
    let initial = reference(&bindings, &mut interner, source, "\np\n")
        .into_proven()
        .unwrap();
    let moved = reference(&bindings, &mut interner, source, "\nq\n")
        .into_proven()
        .unwrap();
    let recreated = reference(&bindings, &mut interner, source, "\np; # recreated")
        .into_proven()
        .unwrap();
    assert_eq!(initial, moved);
    assert_ne!(initial, recreated);
    let mut counts = tcl_core_types::answer::ConformanceCounts::default();
    counts.observe(&Answer::proven(initial == moved), Some(&true));
    counts.observe(&Answer::proven(initial != recreated), Some(&true));
    println!("analysis: {counts}");
}

#[test]
fn deleted_procedure_fixture_has_a_fresh_identity_after_recreation() {
    let source = alloc_fixture();
    let registry = tcl_registry::CommandRegistry::build_default();
    let bindings =
        SourceCommandBindings::analyse(&source, tcl_lexer::LexerConfig::default(), &registry);
    let mut interner = CommandRefInterner::new(bindings.source_origin().unwrap());
    let before = bindings
        .invocation_at_source("", u32::try_from(source.find("[f]").unwrap() + 1).unwrap())
        .original_evaluated_command_reference()
        .expect("initial f")
        .command_ref(&mut interner)
        .into_proven()
        .unwrap();
    let after = reference(&bindings, &mut interner, &source, "\nf; # recreated")
        .into_proven()
        .unwrap();
    assert_ne!(before, after);
}

fn alloc_fixture() -> String {
    let original = include_str!(
        "../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e09-delete-proc.tcl"
    );
    format!("{original}\nproc f {{}} {{return fresh}}\nf; # recreated\n")
}

#[test]
fn rename_fixture_uses_one_identity_for_moved_command_and_another_for_new_name() {
    let source = include_str!(
        "../../../docs/work/name-handling/handover/oo-dispatch-probe/rename-alias/r01-rename-basic.tcl"
    );
    let registry = tcl_registry::CommandRegistry::build_default();
    let bindings =
        SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
    let mut interner = CommandRefInterner::new(bindings.source_origin().unwrap());
    let before = bindings
        .invocation_at_source(
            "",
            u32::try_from(source.find("rename p q").unwrap()).unwrap(),
        )
        .command_answer("p", 8)
        .into_proven()
        .expect("p before the rename")
        .command_ref(&mut interner)
        .into_proven()
        .unwrap();
    let after =
        bindings.invocation_at_source("", u32::try_from(source.rfind("puts").unwrap()).unwrap());
    let moved = after
        .command_answer("q", 8)
        .into_proven()
        .expect("q after the rename")
        .command_ref(&mut interner)
        .into_proven()
        .unwrap();
    let new_name = after
        .command_answer("p", 8)
        .into_proven()
        .expect("fresh p")
        .command_ref(&mut interner)
        .into_proven()
        .unwrap();
    assert_eq!(before, moved);
    assert_ne!(before, new_name);
}
