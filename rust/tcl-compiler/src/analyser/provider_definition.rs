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

//! Conditional provider class inventory without installation or receiver entry.

use super::{
    Analyser,
    original_definer::ClassDefinerCall,
    scope::{scope_at, scope_at_mut},
    types::{ClassDef, MetaclassProvenance},
};
use crate::signature_scan::original_name::SourceOriginalNameOccurrence;
use tcl_lexer::Token;
use tcl_registry::{
    ArgRole,
    definer::{DefinerFamily, DefinitionBodyGrammar},
};

struct InventoryDefinition {
    class: ClassDef,
    name: SourceOriginalNameOccurrence,
    grammar: &'static DefinitionBodyGrammar,
    body: usize,
    disabled: bool,
}

impl Analyser {
    fn source_inventory_definition(
        &self,
        call: &ClassDefinerCall<'_>,
    ) -> Option<InventoryDefinition> {
        let command = call.cmd_tok?;
        let tokens = self.retained_invocation_tokens(command.span.start(), call.arg_tokens)?;
        let head = self.original_static_source_name_at_span(command.span)?;
        let generation = self.analysis_context();
        let namespace = scope_at(&self.result.global_scope, call.scope_path)
            .and_then(|scope| scope.naming_scope.as_ref());
        let metadata = crate::registry_invocation::original_conditional_registry_metadata(
            &generation,
            &tokens,
            head,
            namespace,
        )?;
        if !metadata.roles_complete()
            || metadata
                .original_words()
                .iter()
                .any(|word| word.group().expand)
        {
            return None;
        }
        let descriptor = generation
            .context()
            .resolve_spec(generation.commands(), metadata.command())?;
        let grammar = descriptor.definition_body?;
        if grammar.family != DefinerFamily::Clay {
            return None;
        }
        let ordinal = |wanted| {
            let mut selected = metadata.roles().iter().filter_map(|(index, role)| {
                (*role == wanted).then_some(metadata.argument_offset() + usize::from(*index))
            });
            let index = selected.next()?;
            selected.next().is_none().then_some(index)
        };
        let name_at = ordinal(ArgRole::Name)?;
        let body = ordinal(ArgRole::Body)?;
        let name_token = call.arg_tokens.get(name_at)?;
        let body_token = call.arg_tokens.get(body)?;
        let name = self.original_provider_class_name(name_token.span)?;
        (name.name_input().original_word() == metadata.original_words().get(name_at + 1)?)
            .then_some(())?;
        let source_name =
            crate::signature_scan::scope::SignatureSourceCommand::provider_advice_from_key(
                &self.declaration_namespace_scope(call.scope_path)?,
                name.name_input(),
            )?;
        let class = ClassDef {
            name: source_name.simple_name()?,
            qualified_name: source_name.reported_full_name()?,
            source_name: Some(source_name),
            name_span: name_token.span,
            body_span: body_token.span,
            metaclass: descriptor.name.to_owned(),
            metaclass_provenance: MetaclassProvenance::Observed,
            inheritance_unknown: true,
            member_set_incomplete: true,
            ..ClassDef::default()
        };
        Some(InventoryDefinition {
            class,
            name,
            grammar,
            body,
            disabled: !generation.context().spec_available(descriptor),
        })
    }

    pub(super) fn handle_source_inventory_definition(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> bool {
        self.handle_source_inventory_definer(ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: Some(cmd_tok),
            original: None,
        })
    }

    pub(super) fn handle_source_inventory_definer(&mut self, call: ClassDefinerCall<'_>) -> bool {
        let Some(InventoryDefinition {
            mut class,
            name,
            grammar,
            body,
            disabled,
        }) = self.source_inventory_definition(&call)
        else {
            return false;
        };
        class.doc = std::mem::take(&mut self.last_comment);
        self.parse_oo_definition_body(
            &call.args[body],
            call.arg_tokens[body],
            &mut class,
            call.scope_path,
            Some(grammar),
            disabled,
        );
        let qualified = class.qualified_name.clone();
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.retain_original_provider_class(Some(&name), &class);
        self.result
            .retain_class_declaration(qualified, class.clone());
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, call.scope_path) {
            scope.classes.insert(class.name.clone(), class);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{SourceImage, Span};
    use tcl_registry::definer::DefinitionMemberNamePurpose;

    #[test]
    fn clay_inventory_retains_original_class_and_method_sites() {
        let source = "::clay::define ::Probe {method connect {sock ip port} {return $sock}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let declaration = analysis
            .original_class_declarations()
            .next()
            .expect("original class header");
        let class = declaration.metadata();
        assert_eq!(class.qualified_name, "::Probe");
        assert_eq!(declaration.declaration_site().offset, 0);
        let method = class
            .original_members
            .declarations()
            .next()
            .expect("original method words");
        assert_eq!(method.original_name_input().bytes(), b"connect");
        assert_eq!(
            method.name_purpose(),
            DefinitionMemberNamePurpose::SourceValue
        );
        assert_eq!(method.metadata().name_span, Span::new(31, 38));
        let image = SourceImage::document(source);
        assert_eq!(method.parameters_word().unwrap().image(), &image);
        assert_eq!(method.body_word().unwrap().image(), &image);
        assert_eq!(
            method
                .metadata()
                .params
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<Vec<_>>(),
            ["sock", "ip", "port"]
        );
    }

    #[test]
    fn clay_inventory_uses_original_static_names_and_namespace_geometry() {
        for (source, expected, method_name) in [
            (
                r"clay::define C\u0041 {method {two words} {} {}}",
                "::CA",
                "two words",
            ),
            (
                "namespace eval ::box {clay::define Local {method m {} {}}}",
                "::box::Local",
                "m",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let declaration = analysis
                .original_class_declarations()
                .next()
                .expect("original static class name");
            assert_eq!(declaration.metadata().qualified_name, expected);
            let method = declaration
                .metadata()
                .original_members
                .declarations()
                .next()
                .unwrap();
            assert_eq!(method.original_name_input().bytes(), method_name.as_bytes());
        }
    }

    #[test]
    fn clay_inventory_refuses_computed_expanded_and_replaced_definers() {
        for source in [
            "clayish::define ::Wrong {method connect {} {}}",
            "set name ::Wrong; clay::define $name {method connect {} {}}",
            "{*}{clay::define ::Wrong} {method connect {} {}}",
            "proc ::clay::define args {}; ::clay::define ::Wrong {method connect {} {}}",
            "rename ::clay::define {}; ::clay::define ::Wrong {method connect {} {}}",
            "::clay::define ::Wrong method connect {} {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                analysis.original_class_declarations().next().is_none(),
                "{source}"
            );
            assert!(!analysis.all_classes.contains_key("::Wrong"), "{source}");
        }
    }

    #[test]
    fn clay_inventory_cannot_supply_installation_or_transformed_method_bodies() {
        let source = "clay::define ::Probe {method connect {} {}; constructor {} {}; destructor {}; Method changed dictargs {} {}; Class_Method factory {} {}; Ensemble generated {} {}}; ::Probe new";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let class = &analysis.all_classes["::Probe"];
        assert_eq!(class.original_members.declarations().count(), 1);
        assert!(
            class
                .original_special_members
                .declarations()
                .next()
                .is_none()
        );
        assert!(class.constructors.is_empty() && class.destructor.is_none());
        assert!(class.factory.is_none());
        assert!(class.inheritance_unknown && class.member_set_incomplete);
        assert!(
            crate::registry_invocation::source_structure::source_class_declaration_at(
                source, &analysis, 0
            )
            .is_none()
        );
        let call = u32::try_from(source.find("::Probe new").unwrap()).unwrap();
        assert!(
            crate::registry_invocation::source_structure::source_constructor_call_at(
                source, &analysis, call
            )
            .is_none()
        );
        let registry =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(
            crate::signature_scan::extract_signatures(source, registry.commands())
                .classes
                .is_empty()
        );
    }
}
