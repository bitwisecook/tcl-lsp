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

//! Internal scan-state types used by the `signature_scan` walker.
//!
//! These are crate-private — they accumulate intermediate results
//! during the walk and are consumed by the second-pass factory
//! resolver (see [`super::factory::resolve_factory_defs`]) before
//! the public [`SignatureScanResult`] is returned.
//!
//! - [`ScanCtx`] is threaded as `&mut` through the walker and every
//!   handler; it owns the public `result` accumulator plus the two
//!   first-pass vectors `candidates` and `proc_bodies`.
//! - [`FactoryCandidate`] records each four-token call (`HEAD NAME
//!   ARGS BODY`) the walker spotted, deferring binding to a real
//!   factory until pass two.
//! - [`ProcBodyInfo`] records each proc body's text + params +
//!   home namespace so the factory detector can spot the canonical
//!   `proc $name $args $body` shape.
//! - [`FACTORY_SKIP_HEADS`] is the negative list of built-in heads
//!   that incidentally take the same four-token shape but are
//!   definitely not factory wrappers.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

use tcl_lexer::Token;

use super::scope::{SignatureNamespaceScope, SignatureSourceCommand};
use super::types::{SignatureProc, SignatureScanResult};

/// A factory-wrapper call captured during the first scan pass.
///
/// `signature_scan` recognises tcllib-style factory wrappers (e.g.
/// `DEFC name args body`) by their canonical four-token shape and
/// defers binding to a real factory until after the full source has
/// been scanned. `FactoryCandidate` records the call-site data the
/// second pass needs to attribute the synthetic proc to the right
/// namespace.
#[derive(Debug, Clone)]
pub(super) struct FactoryCandidate {
    /// The command head as written at the call site (e.g. `"DEFC"`,
    /// `"::foo::DEF"`).
    pub(super) head: String,
    /// The proc-name argument as written.
    pub(super) name: String,
    /// Token of the name argument (used for the synthetic proc's
    /// `name_range`).
    pub(super) name_tok: Token,
    /// Token of the body argument (used for the synthetic proc's
    /// `body_range`).
    pub(super) body_tok: Token,
    /// Effective rooted constructed namespace key at the call site.
    pub(super) ns_prefix: String,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
    pub(super) original_head: Option<super::scope::SignatureSourceNameKey>,
    pub(super) original_name: Option<super::scope::SignatureSourceNameKey>,
}

/// First-pass record of a proc body, used to identify factory
/// wrappers by their `proc $p1 $p2 $p3` body shape.
///
/// The wrapper's home namespace is the namespace any factory-emitted
/// procs end up in at runtime — `proc $name …` executed inside a
/// proc creates the command in the caller's current namespace, which
/// for a factory-wrapper INIT path is unambiguously the wrapper's
/// own home.
#[derive(Debug, Clone)]
pub(super) struct ProcBodyInfo {
    /// Fully-qualified proc name with leading `::`.
    pub(super) qname: String,
    /// Parameter names (in declaration order) — the factory body
    /// detector matches these against the `proc $a $b $c` body
    /// shape.
    pub(super) params: Vec<String>,
    /// Verbatim proc body text.
    pub(super) body_text: String,
    /// Namespace any synthetic procs created by this wrapper live
    /// in, as a rooted constructed namespace key.
    pub(super) ns_prefix: String,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
    pub(super) source_name: Option<SignatureSourceCommand>,
}

/// Mutable scan context threaded through the walker.
#[derive(Debug, Default)]
pub(super) struct ScanCtx<'r> {
    /// Public result accumulator.
    pub(super) result: SignatureScanResult,
    /// Factory-call candidates collected during pass 1.
    pub(super) candidates: Vec<FactoryCandidate>,
    /// Proc-body records collected during pass 1, used to identify
    /// real factory wrappers in pass 2.
    pub(super) proc_bodies: Vec<ProcBodyInfo>,
    /// Command heads that match the factory-wrapper token shape but
    /// are not factories — sourced from the registry's
    /// `NOT_PROC_FACTORY` trait plus [`FACTORY_SKIP_NONCOMMAND_HEADS`],
    /// built once by `extract_signatures`.  Empty in `Default`
    /// (used only by focused unit tests).
    pub(super) skip_heads: std::collections::HashSet<String>,
    /// The command registry: drives the walker's definer dispatch (class
    /// definers via their `definition_body` grammar family, procedure
    /// definers via `Traits::DEFINES_PROCEDURE`) and resolves
    /// `ArgRole::CommandPrefix` callback positions + arities so
    /// background-scanned files record callback heads (`lsort -command cb`)
    /// as command invocations.  `None` in `Default` (focused unit tests
    /// that bypass registry dispatch).
    pub(super) registry: Option<&'r tcl_registry::CommandRegistry>,
    /// Optional exact-image inventory supplied by a caller that already owns
    /// source analysis. Header-only callers never build this inventory here.
    pub(super) original_bindings: Option<&'r crate::command_binding::SourceCommandBindings>,
    /// The document dialect's word-value rules — how a braced word's
    /// `\<newline>` folds and how list text divides — used by every
    /// re-parse of a scanned word (a proc's parameter list, an OO member's).
    /// Threaded from the scan's entry point so the scanner reads a word the
    /// way the document's own runtime reads it, rather than re-deriving C
    /// Tcl's answer at each site.
    pub(super) rules: tcl_syntax::word_rules::WordValueRules,
    /// The document dialect's lexer config — every segmentation the walk
    /// performs (the top-level stream, a recursed body, a factory-wrapper
    /// body) reads the source under this grammar rather than the default
    /// one.  Threaded from the scan's entry point, which derives it from
    /// the registry's own profile.
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
    /// Complete original document, distinct from decoded body display text.
    pub(super) original_image: Option<tcl_lexer::SourceImage>,
    /// Canonical current command vector, saved across recursive source walks.
    pub(super) original_words: Vec<tcl_lexer::NativeWord>,
    pub(super) ambiguous_proc_names: std::collections::HashSet<String>,
}

impl ScanCtx<'_> {
    /// One readonly schema for this exact original argv. Supplied source
    /// ownership is authoritative; only a header-only scan uses catalogue
    /// compatibility. Positional handlers accept direct written operands only.
    pub(super) fn with_selected_schema<T>(
        &self,
        command: &crate::segmenter::SegmentedCommand,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> T,
    ) -> Option<T> {
        let registry = self.registry?;
        if let Some(bindings) = self.original_bindings {
            let binding = bindings.invocation_at_source("", command.span.start());
            let (original, mut tokens) = binding.original_recorded_command()?;
            if original.span != command.span || original.argv != command.argv {
                return None;
            }
            let owner = bindings.source_metadata_owner();
            let profile = owner
                .source_analysis_input()
                .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
                .or_else(|| registry.profile());
            let metadata = owner.metadata_context_for_source(registry, self.config, profile)?;
            tokens.source_binding = Some(binding.clone());
            if bindings.original_arguments_rejected_before_handler(&tokens) {
                return None;
            }
            let selected =
                crate::registry_invocation::original_callback_invocation_with_metadata_context(
                    registry, metadata, &tokens,
                )?;
            // Captured prefixes and expansion children have no representative
            // whole-word token. They cannot borrow the written handler offsets.
            if selected.effective.words.len() != command.argv.len()
                || !selected
                    .effective
                    .origins
                    .iter()
                    .enumerate()
                    .skip(1)
                    .all(|(index, origin)| {
                        *origin == crate::registry_invocation::InvocationWordOrigin::Written(index)
                    })
            {
                return None;
            }
            return selected.with_metadata_schema(
                registry,
                metadata,
                binding.invocation_realm()?,
                |schema| Some(project(schema)),
            );
        }
        let image = self.original_image.as_ref()?;
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            self.config,
            command,
        );
        let dialect = crate::environment_ingress::authoring_invocation_dialect(
            registry,
            self.authoring_profile(),
            self.config,
        );
        let values: Vec<_> = tokens
            .words()
            .iter()
            .map(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.config.escapes,
                    dialect.word_values,
                )
            })
            .collect();
        let words: Vec<_> = tokens
            .words()
            .iter()
            .zip(&values)
            .map(|(source, value)| {
                crate::registry_invocation::invocation_word_with_source(
                    source,
                    value,
                    self.config.escapes,
                )
            })
            .collect();
        let (head, arguments) = words.split_first()?;
        // A header cannot supply a positional source operand through argv
        // expansion, even when a particular string happens to look literal.
        if words
            .iter()
            .any(|word| word.kind() == tcl_registry::InvocationWordKind::Expanded)
        {
            return None;
        }
        let resolution = registry.resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(*head, arguments).with_dialect(dialect),
            registry.own_surface_query(),
        );
        Some(project(&resolution.resolved()?))
    }

    pub(super) fn original_callback_prefix(
        &self,
        command: &crate::segmenter::SegmentedCommand,
        written: usize,
    ) -> Option<crate::command_binding::OriginalCallbackPrefix> {
        let registry = self.registry?;
        if let Some(bindings) = self.original_bindings {
            let binding = bindings.invocation_at_source("", command.span.start());
            let recorded = binding.original_recorded_command();
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CALLBACK").is_some() {
                eprintln!(
                    "ORIGINAL_CALLBACK_SCAN written={written} span={:?} recorded={:?}",
                    command.span,
                    recorded
                        .as_ref()
                        .map(|(original, _)| (original.span, original.argv == command.argv))
                );
            }
            let (original, mut tokens) = recorded?;
            if original.span != command.span || original.argv != command.argv {
                return None;
            }
            tokens.source_binding = Some(binding.clone());
            // Matching original inventory is authoritative, including refusal.
            // A replaced/deleted installer never borrows a header's callback role.
            let owner = bindings.source_metadata_owner();
            owner.metadata_context_for_source(registry, self.config, self.authoring_profile())?;
            return match owner {
                crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(context) => {
                    binding.original_callback_prefix_in_context(&tokens, written, context)
                }
                crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(
                    input,
                ) => binding.original_callback_prefix_in_context(
                    &tokens,
                    written,
                    input.borrowed_context_registry(),
                ),
                crate::registry_invocation::OwnedInvocationMetadataContext::Standalone => {
                    binding.original_callback_prefix(&tokens, written, registry)
                }
                crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable => None,
            };
        }
        // Without a matching site issuer, only a readonly static operand can
        // survive. Registry metadata chooses its evaluator; lookup stays absent.
        let dialect = crate::environment_ingress::authoring_invocation_dialect(
            registry,
            self.authoring_profile(),
            self.config,
        );
        let native = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            &self.original_words,
            dialect.native_string_protocol()?,
        )
        .ok()?;
        let words: Vec<_> = self
            .original_words
            .iter()
            .enumerate()
            .map(|(ordinal, word)| {
                if word.group().expand {
                    return tcl_registry::InvocationWord::Expanded;
                }
                native
                    .literal(ordinal)
                    .and_then(|value| core::str::from_utf8(value).ok())
                    .map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::Literal,
                    )
            })
            .collect();
        let (head, arguments) = words.split_first()?;
        let resolution = registry.resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(*head, arguments).with_dialect(dialect),
            registry.own_surface_query(),
        );
        let facts = resolution.resolved()?.facts();
        crate::command_binding::OriginalCallbackPrefix::from_original_static_operand(
            self.original_words.get(written)?,
            &facts,
            written.checked_sub(1)?,
            dialect,
        )
    }

    fn authoring_profile(&self) -> Option<&tcl_dialect::DialectProfile> {
        self.original_bindings
            .and_then(|bindings| bindings.source_metadata_owner().source_analysis_input())
            .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
            .or_else(|| self.registry?.profile())
    }

    /// Pure source assistance; the policy never supplies a runtime lookup receipt.
    pub(super) fn name_policy(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        self.authoring_profile().and_then(|profile| {
            tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
        })
    }

    pub(super) fn original_name_key(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<super::scope::SignatureSourceNameKey> {
        let mut selected = self.original_words.iter().filter(|word| {
            word.span() == span
                || word
                    .tokens()
                    .first()
                    .is_some_and(|token| token.span == span)
        });
        let word = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        super::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            self.rules,
            self.name_policy()?,
        )
    }

    pub(super) fn formal_count(
        &self,
        span: tcl_lexer::Span,
    ) -> super::formal_count::SourceFormalCount {
        use super::formal_count::SourceFormalCount;
        let Some(registry) = self.registry else {
            return SourceFormalCount::Authored(tcl_dialect::ParameterGrammar::Tcl);
        };
        let dialect =
            crate::environment_ingress::authoring_invocation_dialect(registry, None, self.config);
        if dialect.authored_name_policy().is_none() {
            return SourceFormalCount::Authored(
                dialect
                    .parameter_grammar()
                    .unwrap_or(tcl_dialect::ParameterGrammar::Tcl),
            );
        }
        self.original_name_key(span)
            .as_ref()
            .map_or(SourceFormalCount::Unknown, |input| {
                SourceFormalCount::from_original_input(input, dialect)
            })
    }

    pub(super) fn original_command_words(
        &self,
        command: &crate::segmenter::SegmentedCommand,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        let plan = tcl_lexer::native_script_words_in(
            self.original_image.clone()?,
            command.span,
            self.config,
        )
        .ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return None;
        }
        let original = &plan.commands[0];
        if original.words.len() != command.argv.len()
            || !original
                .words
                .iter()
                .zip(&command.argv)
                .all(|(word, token)| word.tokens().first() == Some(token))
        {
            return None;
        }
        Some(original.words.clone())
    }

    pub(super) fn current_namespace(&self, compatibility: &str) -> Option<SignatureNamespaceScope> {
        self.namespace_scope.clone().or_else(|| {
            let root = SignatureNamespaceScope::root(self.name_policy());
            root.child(compatibility, self.name_policy())
        })
    }

    pub(super) fn procedure_name_in_context(
        &self,
        namespace: &SignatureNamespaceScope,
        written: &str,
        original: Option<&super::scope::SignatureSourceNameKey>,
    ) -> Option<(
        String,
        String,
        SignatureNamespaceScope,
        Option<SignatureSourceCommand>,
    )> {
        let Some(policy) = self.name_policy() else {
            let qualified = crate::naming::qualify(&namespace.display()?, written);
            let (holder, simple) = crate::naming::key_holder_and_tail(&qualified);
            return Some((
                qualified.clone(),
                simple.to_owned(),
                SignatureNamespaceScope::Symbolic(holder.to_owned()),
                None,
            ));
        };
        let source_name = match original {
            Some(key) => SignatureSourceCommand::procedure_from_key(namespace, key)?,
            None if self.original_image.is_none() => {
                SignatureSourceCommand::procedure_in_context(policy, namespace, written)?
            }
            None => return None,
        };
        let qualified = source_name
            .reported_full_name()
            .unwrap_or_else(|| written.to_owned());
        let body_scope = source_name.body_scope()?;
        let simple = source_name
            .simple_name()
            .unwrap_or_else(|| written.to_owned());
        Some((qualified, simple, body_scope, Some(source_name)))
    }

    pub(super) fn publication_name(
        &self,
        namespace: &SignatureNamespaceScope,
        written: &str,
        purpose: tcl_syntax::naming::NativeNamePurpose,
        span: tcl_lexer::Span,
    ) -> Option<(String, Option<SignatureSourceCommand>)> {
        let Some(policy) = self.name_policy() else {
            return Some((crate::naming::qualify(&namespace.display()?, written), None));
        };
        let original = self.original_name_key(span);
        let bytes = match &original {
            Some(key) => key.bytes(),
            None if self.original_image.is_none() => written.as_bytes(),
            None => return None,
        };
        let recipe = policy.recipe();
        let context = namespace.context()?;
        let slot = match purpose {
            tcl_syntax::naming::NativeNamePurpose::CommandPublication => {
                recipe.command_publication_slot(context, bytes)
            }
            tcl_syntax::naming::NativeNamePurpose::RenameDestination => {
                recipe.rename_destination_slot(context, bytes)
            }
            tcl_syntax::naming::NativeNamePurpose::AliasPublication => {
                recipe.alias_publication_slot(context, bytes)
            }
            tcl_syntax::naming::NativeNamePurpose::OoObjectPublication => {
                recipe.oo_object_publication_slot(context, bytes)
            }
            _ => return None,
        }
        .ok()?;
        let source_name = match purpose {
            tcl_syntax::naming::NativeNamePurpose::OoObjectPublication => match original.as_ref() {
                Some(key) => SignatureSourceCommand::object_from_key(namespace, key)?,
                None => SignatureSourceCommand::object_in_context(policy, namespace, written)?,
            },
            _ => SignatureSourceCommand::new(policy, slot.clone()),
        };
        let reported = source_name
            .reported_full_name()
            .unwrap_or_else(|| written.to_owned());
        Some((reported, Some(source_name)))
    }

    pub(super) fn namespace_context(
        &self,
        namespace: &str,
        written: &str,
        span: tcl_lexer::Span,
    ) -> Option<SignatureNamespaceScope> {
        let parent = self.current_namespace(namespace)?;
        match self.original_name_key(span) {
            Some(key) => parent.child_from_key(&key),
            None if self.original_image.is_none() => parent.child(written, self.name_policy()),
            None => None,
        }
    }

    pub(super) fn record_proc(&mut self, declaration: SignatureProc) {
        let name = declaration.qualified_name.clone();
        if self
            .result
            .procs
            .get(&name)
            .is_some_and(|previous| previous.source_name != declaration.source_name)
        {
            self.result.procs.remove(&name);
            self.ambiguous_proc_names.insert(name.clone());
        }
        if !self.ambiguous_proc_names.contains(&name) {
            self.result.procs.insert(name, declaration.clone());
        }
        self.result.procedure_declarations.push(declaration);
    }

    #[cfg(test)]
    pub(super) fn command_keys(namespace: &str, written: &str) -> Vec<String> {
        let local = crate::naming::qualify(namespace, written);
        let global = crate::naming::qualify("::", written);
        if local == global || written.starts_with("::") {
            vec![local]
        } else {
            vec![local, global]
        }
    }
}

/// Project source declarations with an explicitly authored policy. The returned
/// analytical key carries no entered command identity or execution authority.
#[cfg(test)]
pub(super) fn authored_publication_key(
    namespace: &str,
    written: &str,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    purpose: tcl_syntax::naming::NativeNamePurpose,
) -> Option<String> {
    use tcl_syntax::naming::{NativeNameContext, NativeNamePurpose};
    let Some(policy) = policy else {
        return Some(crate::naming::qualify(namespace, written));
    };
    let path =
        tcl_core_types::ByteNamespacePath::from_segments(crate::naming::key_segments(namespace));
    let context = NativeNameContext::with_jim_namespace(
        &path,
        namespace.strip_prefix("::").unwrap_or(namespace).as_bytes(),
    );
    let recipe = policy.recipe();
    let slot = match purpose {
        NativeNamePurpose::RenameDestination => {
            recipe.rename_destination_slot(context, written.as_bytes())
        }
        NativeNamePurpose::AliasPublication => {
            recipe.alias_publication_slot(context, written.as_bytes())
        }
        _ => return None,
    }
    .ok()?;
    String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(&slot)).ok()
}

/// Factory-skip heads that are **not** registered commands and so
/// cannot carry the registry's `NOT_PROC_FACTORY` trait: the `TclOO`
/// definition keywords `method` / `classmethod` and the
/// (unregistered) itcl class-definition heads.  `extract_signatures`
/// unions these with the registry-stamped heads for factory exclusion.
pub(super) const FACTORY_SKIP_NONCOMMAND_HEADS: &[&str] =
    &["method", "classmethod", "itcl::class", "::itcl::class"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noncommand_skip_heads_are_the_unregistered_four() {
        assert_eq!(FACTORY_SKIP_NONCOMMAND_HEADS.len(), 4);
        for head in ["method", "classmethod", "itcl::class", "::itcl::class"] {
            assert!(FACTORY_SKIP_NONCOMMAND_HEADS.contains(&head));
        }
    }

    #[test]
    fn default_ctx_is_empty() {
        let ctx = ScanCtx::default();
        assert_eq!(ctx.candidates.len(), 0);
        assert_eq!(ctx.proc_bodies.len(), 0);
        assert_eq!(ctx.result.procs.len(), 0);
        assert_eq!(
            ctx.result.command_invocations,
            [] as [crate::signature_scan::types::SignatureCommandInvocation; 0]
        );
    }
}
