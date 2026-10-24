// SPDX-License-Identifier: AGPL-3.0-or-later
//! Private first-entry command advice from the original selected Jim wrapper.

use super::{Arc, ResolvedAnalysisInput, SourceCommandBindings};

impl SourceCommandBindings {
    /// Readonly first-head advice from the fresh source stock-call recipe.
    /// Scope coordinates only join the original body; complete current source,
    /// factory, world and inert-argv checks remain separate proof obligations.
    pub(crate) fn first_jim_method_presence_in_scope(
        &self,
        document: &str,
        input: &ResolvedAnalysisInput,
        scope_body: tcl_lexer::Span,
        offset: u32,
    ) -> Option<super::super::SourceCommandSlotPresence> {
        let image = self.root_origin.as_ref()?.source_image();
        if image.channel() != tcl_lexer::SourceChannel::Document
            || image.bytes() != document.as_bytes()
            || self.dispatch_points.contains_key(&offset)
        {
            return None;
        }
        let mut wrappers = self.future_body_inventories().iter().filter_map(|future| {
            let wrapper = future.original_jim_wrapper()?;
            (wrapper.input == *input
                && wrapper.body.tokens().iter().any(|token| {
                    token.kind == tcl_lexer::TokenType::Str && token.span == scope_body
                }))
            .then_some(wrapper)
        });
        let wrapper = wrappers.next()?;
        if wrappers.next().is_some()
            || !wrapper.call.matches_source_context(
                image,
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
        {
            return None;
        }
        let entry = super::super::jim_stock_class::OriginalJimStockMethodEntry::capture(
            self,
            Arc::clone(&self.final_state),
            &wrapper.published,
            wrapper.call.original_words(),
        )?;
        if entry.input() != Some(input) || !entry.owns_world(&self.final_state) {
            return None;
        }
        let (first_offset, presence) = wrapper.first_stock_method_presence(&entry)?;
        (first_offset == offset).then_some(presence)
    }
}

#[cfg(test)]
mod tests {
    fn presence(
        analysis: &crate::analyser::AnalysisResult,
        text: &str,
        offset: u32,
    ) -> Option<super::super::super::SourceCommandSlotPresence> {
        let body = crate::analyser::scope::selected_jim_method_body_span_at(
            &analysis.global_scope,
            offset,
        )?;
        analysis
            .retained_command_realm()?
            .source_bindings_ref()
            .first_jim_method_presence_in_scope(
                text,
                analysis.resolved_input.as_ref()?,
                body,
                offset,
            )
    }

    #[test]
    fn original_single_parent_definition_prefix_issues_first_entry_absence() {
        tcl_spectcl::core_surfaces::ensure();
        let text = "# tcl-dialect: jim\nclass Point {x 0 y 0}\nclass Point3 Point {z 0}\nPoint method norm {} { return [expr {abs($x) + abs($y)}] }\nPoint3 method who {} { not_a_command_anywhere $self $x $z }\n";
        let offset = u32::try_from(text.find("not_a_command_anywhere").unwrap()).unwrap();
        for per_item in [false, true] {
            let mut analyser = crate::analyser::Analyser::new();
            let analysis = if per_item {
                analyser.analyse_per_item(text, "jim")
            } else {
                analyser.analyse(text, "jim")
            };
            assert_eq!(
                presence(&analysis, text, offset),
                Some(super::super::super::SourceCommandSlotPresence::Absent)
            );
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .any(|d| d.code.as_str() == "W123"
                        && d.message.contains("not_a_command_anywhere"))
            );
            let generic = analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref();
            assert_eq!(
                generic.diagnostic_slot_presence_at(offset),
                super::super::super::SourceCommandSlotPresence::Unknown
            );
            assert!(presence(&analysis, "foreign bytes", offset).is_none());
            let future = generic
                .future_body_inventories()
                .iter()
                .find_map(|future| future.invocation_at(offset))
                .unwrap();
            assert!(future.proved_execution_target().is_none());
        }
    }

    #[test]
    fn prior_body_effects_substitutions_and_generated_heads_remain_unknown() {
        tcl_spectcl::core_surfaces::ensure();
        for body in [
            "set q value; missing_first_boundary $self",
            "if {[exists -command {::<reference.<C______>.00000000000000000000>}]} {proc missing_first_boundary {} {}}; missing_first_boundary",
            "missing_first_boundary [ref value tag finalizer]",
            "missing_first_boundary $a(k)",
            "missing_first_boundary $foreign",
        ] {
            let text = format!("class C {{x 0}}; C method m {{}} {{{body}}}");
            let analysis = crate::analyser::Analyser::new().analyse(&text, "jim");
            let offset = u32::try_from(text.rfind("missing_first_boundary").unwrap()).unwrap();
            assert!(presence(&analysis, &text, offset).is_none());
        }
        let text = "class C {}; C method m {} {{::<reference.<C______>.00000000000000000000>}}";
        let analysis = crate::analyser::Analyser::new().analyse(text, "jim");
        let offset = u32::try_from(text.find("::<reference").unwrap()).unwrap() - 1;
        assert_eq!(
            presence(&analysis, text, offset),
            Some(super::super::super::SourceCommandSlotPresence::Unknown)
        );
    }

    #[test]
    fn unknown_history_overrides_and_conflicting_links_do_not_issue_absence() {
        tcl_spectcl::core_surfaces::ensure();
        for text in [
            "ref value tag finalizer; class C {}; C method m {} {missing_entry}",
            "class C {}; C method m {} {missing_entry}; ref value tag finalizer",
            "opaque; class C {}; C method m {} {missing_entry}",
            "class A {}; class B {}; class C A B {}; C method m {} {missing_entry}",
            "class C {}; C method constructor {} {proc missing_entry {} {}}; C method m {} {missing_entry}",
            "class C {self 0}; C method m {} {missing_entry}",
            "class C {x 0}; C method m {x} {missing_entry}",
            "class C {}; C method m {} {missing_entry}; proc lsort args {}",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(text, "jim");
            let offset = u32::try_from(text.rfind("missing_entry").unwrap()).unwrap();
            assert_ne!(
                presence(&analysis, text, offset),
                Some(super::super::super::SourceCommandSlotPresence::Absent)
            );
        }
    }
}
