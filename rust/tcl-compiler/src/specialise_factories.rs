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

//! Bounded analysis coverage for bodies installed by native procedure factories.
//!
//! Lowering retains exact installation receipts and evaluated body origins.
//! This pass only limits that metadata. Factory calls stay in the executable IR;
//! neither a lexical declaration shape nor an analysis body installs a callable.

use std::collections::BTreeMap;

use tcl_registry::CommandRegistry;

use crate::ir::Module;

/// Default number of analysis bodies retained per actual allocation instruction.
pub const DEFAULT_FACTORY_CAP: usize = 64;

/// Limit factory body metadata without changing native calls or declarations.
pub fn specialise_factories(module: &mut Module, registry: &CommandRegistry) {
    specialise_factories_with_cap(module, registry, DEFAULT_FACTORY_CAP);
}

/// Apply an explicit cap per exact source allocation site. Names are display
/// labels; source instance and allocation instruction identify the grouping.
pub fn specialise_factories_with_cap(module: &mut Module, _registry: &CommandRegistry, cap: usize) {
    let mut counts = BTreeMap::new();
    let removed: Vec<_> = module
        .installed_procedure_body_units
        .iter()
        .filter_map(|(label, allocation)| {
            let count = counts.entry(allocation.site.clone()).or_insert(0usize);
            *count += 1;
            (*count > cap).then(|| label.clone())
        })
        .collect();
    for label in removed {
        module.installed_procedure_body_units.remove(&label);
        module.body_units.remove(&label);
        module.lambda_body_units.remove(&label);
    }
    let removed: Vec<_> = module
        .original_declaration_body_units
        .iter()
        .filter_map(|(label, allocation)| {
            let count = counts.entry(allocation.site.clone()).or_insert(0usize);
            *count += 1;
            (*count > cap).then(|| label.clone())
        })
        .collect();
    for label in removed {
        module.original_declaration_body_units.remove(&label);
        module.body_units.remove(&label);
        module.lambda_body_units.remove(&label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn factory(source: &str) -> Module {
        lower_to_ir(source, &registry())
    }

    fn installed(module: &Module) -> Vec<&crate::ir::Procedure> {
        module
            .installed_procedure_body_units
            .keys()
            .map(|key| &module.body_units[key])
            .collect()
    }

    #[test]
    fn installed_factory_body_is_analysis_only_and_call_is_retained() {
        let mut module = factory(
            "proc F {name value} {proc $name {x} [subst -nocommands {return $value}]}\nF made 7",
        );
        let before = module.top_level.clone();
        specialise_factories(&mut module, &registry());
        let units = installed(&module);
        assert_eq!(
            units.len(),
            1,
            "{:#?}",
            module.installed_procedure_body_units
        );
        assert_eq!(units[0].name, "::made");
        assert_eq!(units[0].body_source.as_deref(), Some("return 7"));
        assert_eq!(units[0].params, ["x"]);
        assert!(!module.procedures.contains_key("::made"));
        assert_eq!(module.top_level, before);
        assert!(
            module
                .executable_script_roots()
                .iter()
                .all(|(body, _)| *body != &units[0].body)
        );
    }

    #[test]
    fn metadata_requires_reached_normal_installation() {
        for source in [
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}",
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nF $unknown 7",
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nrename subst oldsubst\nproc subst args {error BOOM}\ncatch {F made 7}",
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nF made 7\nrename made {}",
        ] {
            let module = factory(source);
            assert!(installed(&module).is_empty(), "{source}");
        }
    }

    #[test]
    fn metadata_uses_actual_definition_namespace_not_caller_namespace() {
        let module = factory(
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nnamespace eval n {F made 7}",
        );
        let units = installed(&module);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "::made");
        assert!(units[0].qualified_name.starts_with("::installed-body#"));
    }

    #[test]
    fn jim_definition_result_and_native_call_are_preserved() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut module = lower_to_ir(
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nset result [F made 7]",
            registry,
        );
        let before = module.top_level.clone();
        specialise_factories(&mut module, registry);
        assert_eq!(installed(&module).len(), 1);
        assert_eq!(module.top_level, before);
        assert!(!module.procedures.contains_key("::made"));
        assert_eq!(
            module.parameter_grammar(),
            Some(tcl_dialect::ParameterGrammar::Jim)
        );
    }

    #[test]
    fn cap_limits_metadata_and_preserves_calls() {
        let mut module = factory(
            "proc F {name value} {proc $name {} [subst -nocommands {return $value}]}\nF a 1\nF b 2",
        );
        assert_eq!(installed(&module).len(), 2);
        let before = module.top_level.clone();
        specialise_factories_with_cap(&mut module, &registry(), 1);
        assert_eq!(installed(&module).len(), 1);
        assert_eq!(module.top_level, before);
        assert!(!module.procedures.contains_key("::a"));
        assert!(!module.procedures.contains_key("::b"));
    }
}
