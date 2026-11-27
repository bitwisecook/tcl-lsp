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

use tcl_compiler::analyser::Analyser;

#[test]
fn conditional_body_roles_publish_both_source_declarations() {
    tcl_registry::install_shipped().expect("shipped surfaces");
    let result = Analyser::new().analyse(
        "if {$unknown} {proc yes {} {}} else {proc no {} {}}",
        "tcl8.6",
    );
    assert!(result.all_procs.contains_key("::yes"));
    assert!(result.all_procs.contains_key("::no"));
}

#[test]
fn lambda_body_locals_do_not_become_global_declarations() {
    tcl_registry::install_shipped().expect("shipped surfaces");
    let result = Analyser::new().analyse(
        "apply {{argument} {set local $argument}} 1\nputs $local",
        "tcl8.6",
    );
    assert!(!result.global_scope.variables.contains_key("argument"));
    assert!(!result.global_scope.variables.contains_key("local"));
}

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

#[test]
fn proc_body_statement_spans_are_absolute() {
    tcl_registry::install_shipped().expect("shipped surfaces");
    let src = "proc ::f {} { set x 1; return $x }";
    let r = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(src, &r, false);
    let mut procedures = cu.ir_module.summary_procedure_bodies();
    let (_, proc) = procedures.next().expect("one original procedure");
    assert!(procedures.next().is_none());
    let first = proc.body.statements.first().expect("body stmt");
    let span = first.span();
    let text = &src[span.as_range()];
    assert!(
        text.starts_with("set"),
        "expected absolute span pointing at `set`, got {text:?}",
    );
}

#[test]
fn ghost_recovery_retains_the_original_diagnostic_and_fix() {
    tcl_registry::install_shipped().expect("shipped surfaces");
    let source = "set x [foo bar\nset after 1\n";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::E201)
        .expect("unclosed bracket diagnostic");
    assert_eq!(
        usize::try_from(diagnostic.span.start()).unwrap(),
        source.find('[').unwrap()
    );
    let fix = diagnostic.fixes.first().expect("closing bracket insertion");
    assert_eq!(fix.new_text, "]");
    assert_eq!(
        usize::try_from(fix.span.start()).unwrap(),
        source.find('\n').unwrap()
    );
    assert_eq!(fix.span.start(), fix.span.end());
}
