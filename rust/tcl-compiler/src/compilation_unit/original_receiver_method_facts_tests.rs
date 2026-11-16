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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

#[test]
// Implementation contract: naming.variable.original-entered-receiver-body-context
// docs/design/analysis/name-resolution-proofs/original-entered-receiver-body-context.md
fn original_method_facts_keep_counted_formals_without_class_label_variable_union() {
    let source = r"oo::class create C {variable state; method m {arg\uD800} {return ${arg\uD800}}}; oo::define C variable later";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, "tcl8.6");
    let method = fixture_id_if_present(&unit.ir_module.methods, "::C::m")
        .and_then(|id| unit.ir_module.methods.get(&id))
        .expect("original method");
    assert!(method.body.executed_source.is_some());
    let facts = MethodBodyFacts::from_method(method);
    let formals = facts
        .original_parameters
        .as_ref()
        .expect("exact original ParamList");
    assert_eq!(formals.parameters()[0].name, b"arg\xed\xa0\x80");
    assert!(method.instance_vars.is_empty());
    assert!(facts.instance_vars.is_empty());
    assert!(!facts.known_bound_at_entry().contains("state"));
    assert!(!facts.known_bound_at_entry().contains("later"));
    assert!(facts.original_receiver_context.is_none());
}

#[test]
// Implementation contract: naming.variable.original-entered-receiver-body-context
// docs/design/analysis/name-resolution-proofs/original-entered-receiver-body-context.md
fn original_source_context_never_inherits_compatibility_instance_or_execution_fields() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = CompilationUnit::build_for_dialect("set x 1", registry, false, "tcl8.6");
    let mut cfg = unit.top_level.cfg.clone();
    let mut original = cfg
        .command_binding_sites
        .iter()
        .find_map(|site| site.variable_context.as_ref())
        .expect("original point")
        .as_ref()
        .clone();
    assert!(original.execution_name_policy.is_some());
    original.execution = None;
    original.instance_vars.clear();
    original.instance_owner.clear();
    let mut overlay = crate::var_resolve::ResolveContext::default();
    overlay
        .instance_vars
        .insert("compatibility_donor".to_owned());
    overlay.instance_owner = "::WrongProvider".to_owned();
    for site in &mut cfg.command_binding_sites {
        site.variable_context = Some(Arc::new(original.clone()));
    }
    let entry = function_source_entry(&cfg, Some(&overlay));
    assert!(entry.instance_vars.is_empty());
    assert_eq!(entry.instance_owner, "");
    assert_eq!(entry.execution, original.execution);
    assert_eq!(entry.interpreter, original.interpreter);
}
