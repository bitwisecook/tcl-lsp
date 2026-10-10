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

//! Widget/registered-instance diagnostics from exact original factory receipts.
//! Human-readable class maps remain reporting data and never select a schema.

use super::super::state::Analyser;
use super::super::types::{Diagnostic, Severity};
use super::super::{
    DiagnosticSubject, RegisteredInstanceSourceDiagnosticKind as Kind,
    RegisteredInstanceSourceDiagnosticSubject,
};
use super::validity::{
    ScannedInvocation, SeenOption, SeenPositional, arity_verdict, option_relation_diagnostics,
};
use crate::command_binding::OriginalSourceRegisteredInstanceWords;
use std::sync::Arc;
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token};
use tcl_registry::CommandRegistry;

/// An original occurrence to join to the completed immutable source owner.
/// This offset is never sufficient without the whole source/tape correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetDispatchSite {
    /// Original whole command extent, without reconstructed argv or class labels.
    pub cmd_span: Span,
}

fn subject(words: &Arc<OriginalSourceRegisteredInstanceWords>, kind: Kind) -> DiagnosticSubject {
    DiagnosticSubject::RegisteredInstanceSource(Arc::new(
        RegisteredInstanceSourceDiagnosticSubject::new(Arc::clone(words), kind),
    ))
}

fn source_text(words: &OriginalSourceRegisteredInstanceWords, ordinal: usize) -> Option<String> {
    std::str::from_utf8(words.argument_bytes(ordinal)?)
        .ok()
        .map(str::to_owned)
}

fn method_relations(
    words: &Arc<OriginalSourceRegisteredInstanceWords>,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    display: &str,
    extent: Span,
) -> Vec<Diagnostic> {
    let Some(relations) = schema.authored_source_option_relationships() else {
        return Vec::new();
    };
    let options = relations
        .scan
        .options
        .iter()
        .filter_map(|option| {
            let span = words.argument_word(option.argument)?.span();
            let value = option
                .values
                .as_ref()
                .and_then(|values| source_text(words, values.start));
            Some(SeenOption {
                name: option.option.name,
                span,
                value,
            })
        })
        .collect();
    let positionals = relations
        .positionals
        .iter()
        .filter_map(|&ordinal| {
            Some(SeenPositional {
                value: source_text(words, ordinal),
                span: words.argument_word(ordinal)?.span(),
            })
        })
        .collect();
    let call = ScannedInvocation {
        options,
        positionals,
        complete: relations.complete,
    };
    option_relation_diagnostics(
        display,
        &relations.relations,
        relations.constraints,
        &call,
        extent,
    )
    .into_iter()
    .map(|(_, diagnostic)| diagnostic.with_subject(subject(words, Kind::OptionRelation)))
    .collect()
}

fn source_method_diagnostics(
    words: &Arc<OriginalSourceRegisteredInstanceWords>,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<Diagnostic> {
    let Some(method) = words.argument_word(0) else {
        return Vec::new();
    };
    let Some(spelling) = source_text(words, 0) else {
        return Vec::new();
    };
    let Some(factory) = words.instance().descriptor(context) else {
        return Vec::new();
    };
    let extent = Span::new(
        words.original_words()[0].span().start(),
        words.original_words().last().unwrap().span().end(),
    );
    match words.method_selection(context) {
        Some(tcl_registry::abbrev::KeywordMatch::Unknown)
            if !factory.object_class.unwrap().allow_unknown_methods =>
        {
            return vec![
                Diagnostic::new(
                    DiagCode::W001,
                    method.span(),
                    format!(
                        "Unknown subcommand '{spelling}' for widget '{}'",
                        factory.name
                    ),
                    Severity::Warning,
                )
                .with_subject(subject(words, Kind::MethodName)),
            ];
        }
        Some(tcl_registry::abbrev::KeywordMatch::Unique(_)) => {}
        _ => return Vec::new(),
    }
    words
        .with_source_schema(context, |schema| {
            let display = format!("{} {spelling}", factory.name);
            let mut diagnostics = method_relations(words, schema, &display, extent);
            if let Some(selected) = schema.authored_source_arity() {
                let floor = factory
                    .required_package
                    .and_then(|package| context.context().placement_floor(package))
                    .map(tcl_dialect::model::Version::as_str);
                let arity = tcl_registry::arity::ArityWindow::select(selected.windows, floor)
                    .map_or(selected.arity, |window| window.arity);
                if let Some(count) = schema.authored_source_count_for_arity(arity)
                    && let Some(diagnostic) = arity_verdict(
                        &display,
                        arity,
                        usize::from(count.minimum),
                        count.indeterminate,
                        extent,
                        None,
                        selected.synopsis,
                    )
                {
                    diagnostics.push(diagnostic.with_subject(subject(words, Kind::Arity)));
                }
            }
            diagnostics
        })
        .unwrap_or_default()
}

impl Analyser {
    /// Buffer only an occurrence; the retained source owner supplies all words.
    pub(in crate::analyser) fn record_widget_dispatch_candidate(
        &mut self,
        _cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        _arg_tokens: &[Token],
        _arg_expand_in: &[bool],
    ) {
        if !args.is_empty() {
            self.widget_dispatch_sites.push(WidgetDispatchSite {
                cmd_span: cmd_tok.span,
            });
        }
    }

    /// Query genuine source-order factory/handle receipts under the actual
    /// `ContextRegistry`, preserving known shadow, deletion and alias barriers.
    pub(in crate::analyser) fn flush_widget_dispatch_diagnostics(
        &mut self,
        registry: &CommandRegistry,
    ) {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let sites = std::mem::take(&mut self.widget_dispatch_sites);
        let Some(input) = &self.result.resolved_input else {
            return;
        };
        let context = input.context_registry();
        if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key() {
            return;
        }
        for site in sites {
            let Some(words) =
                crate::registry_invocation::source_structure::source_registered_instance_words_at(
                    &self.source,
                    &self.result,
                    site.cmd_span.start(),
                )
            else {
                continue;
            };
            self.result
                .diagnostics
                .extend(source_method_diagnostics(&Arc::new(words), &context));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn codes(src: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn has(src: &str, code: &str) -> bool {
        codes(src).iter().any(|(c, _)| c == code)
    }

    #[test]
    fn w001_fires_for_unknown_widget_subcommand_bareword() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let src = "ttk::treeview .t\n.t bogus\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_fires_for_unknown_widget_subcommand_var() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let src = "set lb [listbox .l]\n$lb bogus\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_silent_for_known_widget_subcommand() {
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn w001_silent_for_configure_and_cget_though_unmodelled() {
        // The selected Registry instance table supplies both methods.
        let src = "ttk::treeview .t\n.t configure -show tree\n.t cget -show\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn e002_fires_for_widget_subcommand_arity() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        // `curselection` takes no further arguments.
        let src = "set lb [listbox .l]\n$lb curselection extra\n";
        assert!(has(src, "E003"), "{:?}", codes(src));
    }

    #[test]
    fn e002_fires_for_too_few_widget_subcommand_args() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        // `move` requires exactly 3 args (item parent index).
        let src = "ttk::treeview .t\n.t move onlyone\n";
        assert!(has(src, "E002"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_when_receiver_is_ambiguous_across_procs() {
        // `.t` is a treeview in one proc and a listbox in another —
        // `bind_registry_instance_class` drops the name as ambiguous, so
        // neither dispatch is diagnosed (never a confident wrong answer).
        let src = "proc a {} {\n    ttk::treeview .t\n    .t bogus\n}\nproc b {} {\n    listbox .t\n    .t alsobogus\n}\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn resolves_when_widget_created_after_the_proc_that_uses_it_is_defined() {
        // The proc dispatching `.t` is *defined* (and so walked) before the
        // `ttk::treeview .t` call that creates it — only valid because the
        // proc isn't actually *called* until after — proving the post-walk
        // two-phase design, not a single top-to-bottom pass, is what makes
        // this resolve.
        let src = "proc setup {} {\n    .t bogus\n}\nttk::treeview .t\nsetup\n";
        assert!(has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_for_untracked_bareword() {
        // `.q` was never created by anything — must not be confused with an
        // ordinary unknown-command (that's W123's job, not this module's).
        let src = ".q bogus\n";
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }

    #[test]
    fn abstains_for_expanded_dispatch() {
        let src = "ttk::treeview .t\nset args {selected}\n.t instate {*}$args bogus_extra_bogus\n";
        // Whatever this resolves to, the `{*}`-expanded tail's true count
        // is unknowable — must not fire a false arity diagnostic.
        assert!(!has(src, "E002") && !has(src, "E003"), "{:?}", codes(src));
    }

    #[test]
    fn tcloo_dispatch_is_unaffected() {
        // A plain TclOO `$obj method` dispatch must keep going through
        // `var_command.rs`'s W308, not this module (registry.get(class)
        // finds no CommandSpec for a user class, so this module silently
        // no-ops for it).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d meow\n";
        assert!(has(src, "W308"), "{:?}", codes(src));
        assert!(!has(src, "W001"), "{:?}", codes(src));
    }
}
