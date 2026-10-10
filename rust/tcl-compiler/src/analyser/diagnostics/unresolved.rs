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

//! Positioned unresolved-slot advice and package assistance diagnostics.
//!
//! W123 consumes the shared source owner's exact selected-slot presence. Its
//! suggestions may use catalogue metadata, but metadata never proves dispatch.
//! Default autoloading remains possible; custom or uncertain fallback handlers
//! suppress absence advice. Missing-package advice uses its separate surface.

use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_registry::model::{BindingKnowledge, BindingTarget};

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

/// Advisory suggestion candidates, independent of positioned command presence.
pub(crate) struct CommandExistenceOracle {
    known: KnownNameTiers,
}

struct KnownNameTiers {
    candidates: Vec<String>,
}

/// Group `(qualified_name, establishing_offset)` pairs by their
/// `::`-tail — shared by the proc and class def maps in
/// [`Analyser::build_w123_known_names`] and its siblings
/// (`var_command.rs`'s `build_w307_known_names` / interpolated-W123
/// resolution): a tail may match several qualified names
/// (the same simple name in different namespaces), each kept with its
/// own offset for a later per-call live check
/// ([`Analyser::fact_live_for_call`]). `pub(super)` (not private) so
/// those sibling passes reuse it rather than reimplementing the same
/// grouping loop.
pub(super) fn group_defs_by_tail<'a>(
    entries: impl Iterator<Item = (&'a String, u32)>,
) -> HashMap<String, Vec<(String, u32)>> {
    let mut map: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    for (qn, off) in entries {
        if let Some((_, tail)) = qn.rsplit_once("::")
            && !tail.is_empty()
        {
            map.entry(tail.to_string())
                .or_default()
                .push((qn.clone(), off));
        }
    }
    map
}

/// The sentence a `W123` ends with where its call widens: a call to a command
/// the module cannot name may reach the frame that calls it (`upvar 1`,
/// `uplevel 1`), so the flow graph widens the variables that frame holds at
/// the call — in a procedure's own frame, its locals — and a stub of `name`
/// stating its frame effect as a plain call
/// ([`tcl_registry::model::DeclaredCommand::plain_call_frame_effect`]) names
/// the command, so they are kept.
fn widening_hint(name: &str) -> String {
    format!(
        "The call widens the variables held at it, in a procedure's own frame its locals; a \
         `# tcl-lsp: stub {name} {{…}} -frame own` (or `-frame none`) declaration keeps them \
         when every argument is a value, name, pattern or channel and no flag but `-pure` or \
         `-unsafe` is set."
    )
}

impl Analyser {
    /// End each `W123` whose call widens with the sentence that says so
    /// ([`widening_hint`]), decided by the fact the widening reads: the flow
    /// graph of `cu` puts the marker for a call to code the module cannot see
    /// ([`crate::ir::SyntheticMarker::UnseenCall`]) over the call, at the top
    /// level, in a `namespace eval` body and in a procedure's own frame
    /// alike. A report whose call no marker covers — one the graph does not
    /// lower where it is written, as an `uplevel #0` body's — ends as it was.
    pub(super) fn settle_w123_widening(&mut self, cu: &crate::compilation_unit::CompilationUnit) {
        if self.result.unresolved_command_sites.is_empty() {
            return;
        }
        let markers: Vec<tcl_lexer::Span> = std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values())
            .chain(cu.body_units.values())
            .flat_map(|unit| unit.cfg.blocks.values())
            .flat_map(|block| &block.statements)
            .filter(|statement| crate::ssa::is_unseen_call_marker(statement))
            .map(crate::ir::Statement::span)
            .collect();
        let widening: Vec<(tcl_lexer::Span, String)> = self
            .result
            .unresolved_command_sites
            .iter()
            .filter(|(site, _)| {
                markers
                    .iter()
                    .any(|marker| marker.start() <= site.start() && site.end() <= marker.end())
            })
            .cloned()
            .collect();
        for diagnostic in &mut self.result.diagnostics {
            if diagnostic.code != DiagCode::W123 {
                continue;
            }
            let Some((_, name)) = widening.iter().find(|(site, _)| *site == diagnostic.span) else {
                continue;
            };
            let joint = if diagnostic.message.ends_with('?') {
                " "
            } else {
                ". "
            };
            diagnostic.message.push_str(joint);
            diagnostic.message.push_str(&widening_hint(name));
        }
    }

    /// W123 advises that the selected callable slot is unresolved at this
    /// source point. It does not predict failure of an autoload handler.
    pub fn emit_unresolved_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.unresolved_commands_emitted {
            return;
        }
        self.unresolved_commands_emitted = true;
        // The W123 *diagnostic* honours `disabled_diagnostics`, but the
        // unresolved-command *call sites* are recorded regardless (below), so a
        // cross-file consumer can run its arity check independently of the W123
        // toggle. Each invocation retains its own positioned lookup evidence.
        let emit_w123 = !self.disabled_diagnostics.contains("W123");

        let oracle = self.command_existence_oracle(registry);
        self.emit_w123_for_invocations(&oracle, emit_w123, registry);
    }

    /// Build suggestions only; presence comes from the shared positioned owner.
    pub(crate) fn command_existence_oracle(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> CommandExistenceOracle {
        CommandExistenceOracle {
            known: self.build_w123_known_names(registry),
        }
    }

    /// Names declared by inline or sidecar stubs.  The analysis setup
    /// assembles this shared, provenance-aware declaration surface, so this
    /// must not rescan just the source document and omit sidecar bundles.
    fn stub_command_names(&self) -> HashSet<String> {
        self.result
            .stub_commands
            .iter()
            .map(|stub| stub.name.clone())
            .collect()
    }

    /// Catalogue and lexical names are candidates for a reviewed spelling fix.
    /// They provide no command-presence, namespace or implementation authority.
    fn build_w123_known_names(&self, _registry: &tcl_registry::CommandRegistry) -> KnownNameTiers {
        let context = self.analysis_context();
        let registry = context.commands();
        let mut candidates = registry
            .command_names_in_any_dialect()
            .filter(|name| context.context().resolve_spec(registry, name).is_some())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        candidates.extend(self.result.all_procs.keys().cloned());
        candidates.extend(self.result.all_classes.keys().cloned());
        candidates.extend(self.result.command_aliases.keys().cloned());
        candidates.extend(self.renamed_commands.keys().cloned());
        candidates.extend(self.stub_command_names());
        candidates.extend(self.extra_commands.iter().cloned());
        candidates.sort_unstable();
        candidates.dedup();
        KnownNameTiers { candidates }
    }

    /// Diagnostic-only presence. Qualified names, implementation effects and
    /// lookup order come from the exact retained execution or future entry.
    #[must_use]
    pub(crate) fn command_binding_knowledge(
        &self,
        _oracle: &CommandExistenceOracle,
        name: &str,
        range: tcl_lexer::Span,
        lookup: crate::signature_scan::types::SignatureCommandLookup,
        is_mathfunc_call: bool,
        _resolution_candidates: &[String],
    ) -> BindingKnowledge {
        use crate::command_binding::SourceCommandSlotPresence as Presence;
        if name.starts_with('$') || name.starts_with('[') {
            return BindingKnowledge::Unknown;
        }
        // Explicit assistance declarations can suppress this advisory without
        // establishing a callable implementation or donating a registry key.
        if self.extra_commands.contains(name)
            || self
                .result
                .stub_commands
                .iter()
                .any(|stub| stub.name == name)
            || self.is_scoped_command_resolved(name, range)
        {
            return BindingKnowledge::Unknown;
        }
        let Some(offset) = lookup.offset(range) else {
            return BindingKnowledge::Unknown;
        };
        let presence = if is_mathfunc_call {
            self.registry
                .as_deref()
                .map_or(Presence::Unknown, |registry| {
                    self.head_identities
                        .diagnostic_math_function_presence_at(registry, name, offset)
                })
        } else {
            match lookup {
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                    self.head_identities.diagnostic_slot_presence_at(offset)
                }
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName { .. } => self
                    .head_identities
                    .diagnostic_command_slot_presence_at(name, offset),
                crate::signature_scan::types::SignatureCommandLookup::DeferredReference
                | crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                    ..
                } => Presence::Unknown,
            }
        };
        match presence {
            Presence::Present => BindingKnowledge::Must(BindingTarget::document(name)),
            Presence::Absent => BindingKnowledge::Absent,
            Presence::MayPresent | Presence::Unknown => BindingKnowledge::Unknown,
        }
    }

    /// Walk every recorded command invocation, record the ones the oracle
    /// proves `Absent` as call sites, and (when `emit_w123`) push a W123
    /// with a "did you mean…?" suggestion.  Restores
    /// `command_invocations` on exit.
    fn emit_w123_for_invocations(
        &mut self,
        oracle: &CommandExistenceOracle,
        emit_w123: bool,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let known = &oracle.known;
        // Pre-compute the deduplicated ``Vec<&str>`` over the
        // candidate set once, instead of rebuilding it per
        // unresolved invocation.  ``candidates`` may carry
        // duplicates because each contributor (registry / proc
        // tails / class tails / aliases / ensemble cmds /
        // stubs / unknown-proc dispatch_targets) is unioned
        // independently — dedupe via a ``HashSet`` filter
        // while preserving stable iteration order.
        let mut seen_candidate_strs: FxHashSet<&str> = FxHashSet::default();
        let candidate_strs: Vec<&str> = known
            .candidates
            .iter()
            .map(String::as_str)
            .filter(|candidate| seen_candidate_strs.insert(*candidate))
            .collect();

        // Drain so the iteration loop can mutate
        // ``self.result.diagnostics`` freely; restore at the end
        // (matches the snapshot/restore round-trip contract).
        let invocations = std::mem::take(&mut self.result.command_invocations);
        #[cfg(debug_assertions)]
        self.trace_unresolved_math_invocations(oracle, &invocations);
        for inv in &invocations {
            let name = &inv.name;
            // An existence probe (`namespace which -command NAME`, exact
            // `info commands NAME`) asserts nothing about the name's
            // existence — reference identity and existence are orthogonal, so
            // the record never feeds W123.
            if inv.existence_probe {
                continue;
            }
            // Only diagnostic-purpose selected-slot absence feeds W123.
            if self.command_binding_knowledge(
                oracle,
                name,
                inv.range,
                inv.lookup,
                inv.is_mathfunc_call,
                &inv.resolution_candidates,
            ) != BindingKnowledge::Absent
            {
                continue;
            }

            // A missing selected slot can still be serviced by autoloading.
            // Cross-file assistance may supply a declaration for this site.
            self.result
                .unresolved_command_sites
                .push((inv.range, name.clone()));
            if !emit_w123 {
                continue;
            }

            let subject = self.unresolved_source_subject(inv, registry);
            let original_name = subject.as_ref().and_then(|subject| match subject {
                super::super::DiagnosticSubject::UnresolvedCommand(subject) => {
                    Some(subject.reporting_name())
                }
                super::super::DiagnosticSubject::UnresolvedMathFunction(subject) => {
                    Some(subject.reporting_name())
                }
                _ => None,
            });
            // "Did you mean…?" suggestion via edit distance (max 1
            // suggestion, budget scaled to the name's length so a short
            // typo can't match an unrelated short command).
            // ``candidate_strs`` was
            // deduplicated above so every name in it is unique;
            // copying the slice per invocation is cheap (Vec of
            // ``&str`` references).  The name itself is excluded — a
            // renamed-away builtin is still in the registry candidate set,
            // and suggesting the very name that no longer resolves would be
            // a self-referential fix.
            let suggestions = original_name
                .map(|name| {
                    crate::text::suggest_similar(
                        name,
                        candidate_strs
                            .iter()
                            .copied()
                            .filter(|candidate| *candidate != name),
                        1,
                        crate::text::scaled_max_distance(name),
                    )
                })
                .unwrap_or_default();
            let mut message = format!("Unresolved command '{name}' at this source point");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
                if let Some(fix) = subject.as_ref().and_then(|subject| {
                    original_command_suggestion_fix(subject, best, self.lexer_config())
                }) {
                    fixes.push(fix);
                }
            }
            let mut diagnostic = crate::analyser::types::Diagnostic::new(
                DiagCode::W123,
                inv.range,
                message,
                Severity::Hint,
            )
            .with_fixes(fixes);
            if let Some(subject) = subject {
                diagnostic = diagnostic.with_subject(subject);
            }
            self.result.diagnostics.push(diagnostic);
        }
        self.result.command_invocations = invocations;
    }

    #[cfg(debug_assertions)]
    fn trace_unresolved_math_invocations(
        &self,
        oracle: &CommandExistenceOracle,
        invocations: &[crate::signature_scan::types::SignatureCommandInvocation],
    ) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_MATH_SUBJECT").is_some() {
            for invocation in invocations {
                eprintln!(
                    "ORIGINAL_MATH_SUBJECT stage=diagnostic-invocation name={:?} range={:?} math={} knowledge={:?}",
                    invocation.name,
                    invocation.range,
                    invocation.is_mathfunc_call,
                    self.command_binding_knowledge(
                        oracle,
                        &invocation.name,
                        invocation.range,
                        invocation.lookup,
                        invocation.is_mathfunc_call,
                        &invocation.resolution_candidates
                    )
                );
            }
        }
    }

    fn unresolved_source_subject(
        &self,
        invocation: &crate::signature_scan::types::SignatureCommandInvocation,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::DiagnosticSubject> {
        use super::super::{DiagnosticSubject, SourceUnresolvedMathFunctionSubject};
        let offset = invocation.lookup.offset(invocation.range)?;
        if invocation.is_mathfunc_call {
            let image = self.head_identities.original_source_image()?;
            let config = self.lexer_config();
            let mut occurrences = self
                .head_identities
                .source_bindings_ref()
                .original_math_functions_in_source(registry, image, config, invocation.range)
                .into_iter()
                .filter(|occurrence| occurrence.span() == invocation.range);
            let occurrence = occurrences.next()?;
            if !occurrences.all(|other| other == occurrence) {
                return None;
            }
            let subject = SourceUnresolvedMathFunctionSubject::from_original_occurrence(
                occurrence,
                invocation,
                image,
                config,
                registry,
                self.head_identities.diagnostic_math_function_presence_at(
                    registry,
                    &invocation.name,
                    offset,
                ),
            )?;
            return Some(DiagnosticSubject::UnresolvedMathFunction(
                std::sync::Arc::new(subject),
            ));
        }
        let subject = super::super::SourceUnresolvedCommandSubject::from_original_invocation(
            self.head_identities.invocation_at_source("", offset),
            invocation,
            self.lexer_config(),
            self.word_rules(),
            self.declaration_name_policy()?,
        )?;
        Some(DiagnosticSubject::UnresolvedCommand(std::sync::Arc::new(
            subject,
        )))
    }

    /// Whether the bare command head `name`, invoked at `range`, resolves
    /// against a scoped command environment active at that position.
    ///
    /// A head is resolved when its call site falls inside a recorded
    /// [`ScopedBodyRegion`](super::super::types::ScopedBodyRegion) whose
    /// environment either lists `name` as one of its commands, exposes sibling
    /// definitions of that name (a previously-defined `report::defstyle`
    /// style), or accepts unknown heads outright.  Purely registry-data driven
    /// — the scoped command set lives on the definer's spec, never here.
    #[must_use]
    fn is_scoped_command_resolved(&self, name: &str, range: tcl_lexer::Span) -> bool {
        let offset = range.start();
        self.result.scoped_command_regions.iter().any(|region| {
            if !region.contains(offset) {
                return false;
            }
            let env = region.env;
            env.is_command(name)
                || env.allow_unknown_commands
                || (env.include_sibling_definitions
                    && self
                        .result
                        .scoped_sibling_defs
                        .get(env.name)
                        .is_some_and(|names| names.contains(name)))
        })
    }

    /// Original source selections for package advice, using immutable whole
    /// vectors and current full context rather than invocation reporting names.
    fn original_package_advice_invocations(
        &self,
    ) -> Vec<super::super::diagnostic_registry::OriginalDiagnosticInvocation> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        use crate::signature_scan::types::SignatureCommandLookup;
        let context = self.analysis_context();
        let mut offsets = HashSet::new();
        self.result
            .command_invocations
            .iter()
            .filter(|invocation| {
                !invocation.existence_probe
                    && !invocation.is_mathfunc_call
                    && invocation.lookup == SignatureCommandLookup::InvocationHead
            })
            .filter_map(|invocation| invocation.lookup.offset(invocation.range))
            .filter(|offset| offsets.insert(*offset))
            .filter_map(|offset| {
                crate::registry_invocation::source_structure::source_registry_words_at(
                    &self.source,
                    &self.result,
                    offset,
                )
            })
            .filter_map(|words| {
                super::super::diagnostic_registry::OriginalDiagnosticInvocation::new(
                    words,
                    std::sync::Arc::clone(&context),
                )
            })
            .collect()
    }

    fn source_package_references(
        &self,
        invocations: &[super::super::diagnostic_registry::OriginalDiagnosticInvocation],
    ) -> Vec<(
        crate::registry_invocation::source_structure::OriginalSourcePackageReference,
        bool,
    )> {
        invocations
            .iter()
            .filter_map(|original| {
                let reference = original.words().package_reference(original.context())?;
                let head = original.head();
                let token_start = head.tokens().first()?.span.start();
                let unconditional =
                    self.result.package_requires.iter().any(|require| {
                        !require.conditional && require.range.start() == token_start
                    });
                Some((reference, unconditional))
            })
            .collect()
    }

    /// Missing package source advice for an authentically selected descriptor.
    /// It never claims installation, loading or eventual callable presence.
    pub fn emit_missing_package_require_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("W120") || self.result.has_dynamic_providers {
            return;
        }
        let generation = self.analysis_context();
        if generation
            .context()
            .resolve_spec(registry, "package")
            .is_none()
        {
            return;
        }
        let invocations = self.original_package_advice_invocations();
        let references = self.source_package_references(&invocations);
        let insert_offset =
            crate::registry_invocation::source_structure::original_package_require_insert_offset(
                &self.source,
                &self.result,
            );
        let mut best = HashMap::new();
        for original in &invocations {
            let Some(package) = original
                .with_schema(super::super::diagnostic_registry::source_descriptors)
                .and_then(|descriptors| descriptors.command.required_package)
            else {
                continue;
            };
            if !package.is_ascii()
                || generation.context().ambient_package(package)
                || references
                    .iter()
                    .any(|(reference, _)| reference.matches_ascii(package))
            {
                continue;
            }
            best.entry(original.command()).and_modify(|current: &mut &super::super::diagnostic_registry::OriginalDiagnosticInvocation| {
                if original.head().span().start() < current.head().span().start() { *current = original; }
            }).or_insert(original);
        }
        let mut rows = best.into_values().collect::<Vec<_>>();
        rows.sort_by_key(|original| original.head().span().start());
        for original in rows {
            let Some(subject) = original.subject(
                super::super::RegistrySourceDiagnosticKind::PackageRequirement,
                None,
            ) else {
                continue;
            };
            let super::super::DiagnosticSubject::RegistrySource(selected) = &subject else {
                continue;
            };
            let Some(package) = selected.required_package() else {
                continue;
            };
            let Some(name) = self.source.get(original.head().span().as_range()) else {
                continue;
            };
            let mut diagnostic = super::types::Diagnostic::new(
                DiagCode::W120,
                original.head().span(),
                format!("\"{name}\" requires `package require {package}`"),
                Severity::Warning,
            )
            .with_subject(subject);
            // A metadata package label cannot manufacture a Native source
            // spelling. Only the retained original naming recipe supplies one.
            if let Some((atom, insert_offset)) = diagnostic
                .required_package_key()
                .and_then(|key| key.manifest_atom(self.lexer_config()))
                .zip(insert_offset)
            {
                diagnostic.fixes.push(super::types::CodeFix {
                    span: tcl_lexer::Span::new(insert_offset, insert_offset),
                    new_text: format!(
                        "{}package require {atom}\n",
                        package_insert_separator(&self.source, insert_offset)
                    ),
                    description: format!("Add 'package require {package}'"),
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                });
            }
            self.result.diagnostics.push(diagnostic);
        }
    }

    /// Source reading order for a genuine selected package command relative
    /// to an authenticated unconditional requirement. Deferred body advice
    /// retains its own source applicability; no executed ordering is asserted.
    pub fn emit_package_require_ordering_hints(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use tcl_registry::source_navigation::SourcePackageReferenceKind as Kind;
        if self.disabled_diagnostics.contains("H301") || self.result.has_dynamic_providers {
            return;
        }
        let generation = self.analysis_context();
        if generation
            .context()
            .resolve_spec(registry, "package")
            .is_none()
        {
            return;
        }
        let invocations = self.original_package_advice_invocations();
        let references = self.source_package_references(&invocations);
        let mut earliest = HashMap::new();
        for original in &invocations {
            let Some(package) = original
                .with_schema(super::super::diagnostic_registry::source_descriptors)
                .and_then(|descriptors| descriptors.command.required_package)
            else {
                continue;
            };
            if !package.is_ascii()
                || generation.context().ambient_package(package)
                || references.iter().any(|(reference, _)| {
                    reference.kind() == Kind::Provide && reference.matches_ascii(package)
                })
            {
                continue;
            }
            let require_start = references
                .iter()
                .filter(|(reference, unconditional)| {
                    *unconditional
                        && reference.kind() == Kind::Require
                        && reference.matches_ascii(package)
                })
                .filter_map(|(reference, _)| {
                    reference
                        .words()
                        .head_source()?
                        .word()
                        .map(|word| word.span().start())
                })
                .min();
            if require_start.is_none_or(|start| original.head().span().start() >= start) {
                continue;
            }
            earliest.entry(package).and_modify(|current: &mut &super::super::diagnostic_registry::OriginalDiagnosticInvocation| {
                if original.head().span().start() < current.head().span().start() { *current = original; }
            }).or_insert(original);
        }
        let mut rows = earliest.into_iter().collect::<Vec<_>>();
        rows.sort_by_key(|(package, original)| (original.head().span().start(), *package));
        for (package, original) in rows {
            let Some(subject) = original.subject(
                super::super::RegistrySourceDiagnosticKind::PackageOrdering,
                None,
            ) else {
                continue;
            };
            let Some(name) = self.source.get(original.head().span().as_range()) else {
                continue;
            };
            self.result.diagnostics.push(
                super::types::Diagnostic::new(
                    DiagCode::H301,
                    original.head().span(),
                    format!(
                        "\"{name}\" is used above the `package require {package}` that provides it"
                    ),
                    Severity::Hint,
                )
                .with_subject(subject),
            );
        }
    }
}

fn package_insert_separator(source: &str, offset: u32) -> &'static str {
    if usize::try_from(offset).ok() == Some(source.len())
        && !source.is_empty()
        && !source.ends_with('\n')
    {
        "\n"
    } else {
        ""
    }
}

/// A reviewed metadata suggestion gains only faithful source spelling, never
/// command presence. Native lexical ownership supplies the whole replacement
/// extent and string recipe; expression identifiers remain a separate grammar.
fn original_command_suggestion_fix(
    subject: &super::super::DiagnosticSubject,
    candidate: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<super::types::CodeFix> {
    // naming.diagnostic.original-command-suggestion-source
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-suggestion-source.md
    let super::super::DiagnosticSubject::UnresolvedCommand(subject) = subject else {
        return None;
    };
    let original = subject.name_input().original_word();
    (candidate.is_ascii() && original.config() == config).then_some(())?;
    let quoted = tcl_syntax::backslash::native_literal_source_word(
        candidate.as_bytes(),
        original.image().channel(),
        config,
        subject.name_input().policy().string_protocol(),
    )?;
    let safe_atom = !candidate.is_empty()
        && !candidate.bytes().any(|byte| {
            byte.is_ascii_whitespace()
                || byte.is_ascii_control()
                || matches!(
                    byte,
                    b';' | b'$' | b'[' | b']' | b'{' | b'}' | b'"' | b'\\' | b'#'
                )
        });
    Some(super::types::CodeFix {
        span: original.span(),
        new_text: if safe_atom {
            candidate.to_owned()
        } else {
            quoted
        },
        description: format!("Replace with '{candidate}'"),
        safety: crate::irules_checks::FixSafety::RequiresReview,
    })
}

#[cfg(test)]
mod require_ordering_tests {
    use crate::analyser::state::Analyser;

    /// `(code, message)` for the two package-requirement codes only — H301
    /// and the W120 it must stay disjoint from.
    fn diags(source: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "H301" | "W120"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn count(source: &str, code: &str) -> usize {
        diags(source).iter().filter(|(c, _)| c == code).count()
    }

    fn slot_diags(source: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() == "W123")
            .map(|diagnostic| (diagnostic.code.to_string(), diagnostic.message.clone()))
            .collect()
    }

    fn slot_count(source: &str, code: &str) -> usize {
        slot_diags(source)
            .iter()
            .filter(|(candidate, _)| candidate == code)
            .count()
    }

    #[test]
    fn unresolved_slot_advice_uses_current_namespace_and_exported_import() {
        assert_eq!(
            slot_count(
                "namespace eval unrelated {proc helper {} {return wrong}}; helper",
                "W123"
            ),
            1
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return right}; namespace export helper}; namespace import ::n::*; helper",
                "W123"
            ),
            0
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return wrong}}; namespace import ::n::*; helper",
                "W123"
            ),
            1
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return right}}; namespace eval call {namespace path ::n; helper}",
                "W123"
            ),
            0
        );
    }

    #[test]
    fn unresolved_slot_advice_respects_temporal_deletion_and_custom_fallback() {
        assert_eq!(slot_count("proc p {} {}; p; rename p {}; p", "W123"), 1);
        assert_eq!(
            slot_count("proc unknown args {return handled}; missing", "W123"),
            0
        );
        assert_eq!(slot_count("rename unknown {}; missing", "W123"), 1);
        let diagnostics = slot_diags("missing");
        assert!(
            diagnostics
                .iter()
                .any(|(code, text)| code == "W123" && text.contains("at this source point")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn consumed_command_names_use_the_post_argument_lookup_point() {
        use crate::signature_scan::types::SignatureCommandLookup;
        for source in [
            "rename missing moved",
            "rename missing {}",
            "info body missing",
        ] {
            assert_eq!(slot_count(source, "W123"), 1, "{source}");
        }
        assert_eq!(slot_count("proc p {} {}; rename p moved", "W123"), 0);
        assert_eq!(slot_count("proc p {} {}; rename p $destination", "W123"), 0);
        assert_eq!(
            slot_count("proc p {} {}; rename p [rename p {}]", "W123"),
            1
        );
        for source in [
            "info commands missing*",
            "namespace which -command missing",
            "interp alias {} later {} missing",
            "lsort -command missing {}",
        ] {
            assert_eq!(slot_count(source, "W123"), 0, "{source}");
        }
        let source = "proc p {} {}; rename p moved";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let occurrence = result
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.name == "p"
                    && matches!(
                        invocation.lookup,
                        SignatureCommandLookup::ConsumedName { .. }
                    )
            })
            .unwrap();
        assert_eq!(
            occurrence.lookup,
            SignatureCommandLookup::ConsumedName {
                invocation_offset: u32::try_from(source.find("rename").unwrap()).unwrap(),
            }
        );
        assert!(occurrence.resolved_command_reference.is_some());
        assert!(!occurrence.lookup.is_execution_site());
    }

    #[test]
    fn possible_consumed_names_preserve_navigation_without_absence_or_rename_authority() {
        use crate::signature_scan::types::SignatureCommandLookup;
        let source = "if {$unknown} {rename info original; proc info args {return ordinary}}; info body missing";
        let result = Analyser::new().analyse(source, "tcl8.6");
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        assert_eq!(slot_count(source, "W123"), 0);
        let reference = result
            .command_invocations
            .iter()
            .find(|reference| reference.name == "missing")
            .expect("possible native role retains navigation");
        assert!(matches!(
            reference.lookup,
            SignatureCommandLookup::PossibleConsumedName { .. }
        ));
        assert!(!reference.rename_safe);
        assert!(!reference.lookup.is_execution_site());
        assert_eq!(slot_count("info body missing", "W123"), 1);
    }

    #[test]
    fn a_command_above_its_require_is_hinted() {
        let src = "csv::join {a b}\npackage require csv\n";
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
        // The requirement is present, so the missing-require warning must not
        // also fire: the two are disjoint by construction.
        assert_eq!(count(src, "W120"), 0, "{:?}", diags(src));
    }

    #[test]
    fn a_command_below_its_require_is_silent() {
        let src = "package require csv\ncsv::join {a b}\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// The semantic view is position-insensitive and stays that way: a call
    /// inside a proc body runs after the file has been sourced, so requiring
    /// at the bottom is correct Tcl. It still *reads* as out of order, which
    /// is the whole point of a hint — but W120 must not fire, because
    /// nothing is missing.
    #[test]
    fn a_deferred_call_above_its_require_is_a_hint_and_not_a_warning() {
        let src = "proc later {} { csv::join {a b} }\npackage require csv\n";
        assert_eq!(count(src, "W120"), 0, "{:?}", diags(src));
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
    }

    #[test]
    fn a_missing_require_stays_w120_and_is_not_also_hinted() {
        let src = "csv::join {a b}\n";
        assert_eq!(count(src, "W120"), 1, "{:?}", diags(src));
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// One ordering mistake, one hint — not one per command.
    #[test]
    fn many_commands_above_one_require_hint_once() {
        let src = "csv::join {a b}\ncsv::split x\ncsv::report x\npackage require csv\n";
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
    }

    /// The package's own implementation file requires nothing of itself.
    #[test]
    fn a_providing_file_is_silent() {
        let src = "csv::join {a b}\npackage provide csv 1.0\npackage require csv\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// Inside a guarded branch there is no unconditional "before" to be
    /// after, so the ordering claim cannot be made.
    #[test]
    fn a_conditional_require_anchors_nothing() {
        let src = "csv::join {a b}\nif {$x} { package require csv }\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    #[test]
    fn the_hint_is_off_when_disabled() {
        let src = "csv::join {a b}\npackage require csv\n";
        let out = Analyser::with_disabled_diagnostics(["H301".to_owned()].into_iter().collect())
            .analyse(src, "tcl8.6");
        assert!(
            !out.diagnostics.iter().any(|d| d.code.as_str() == "H301"),
            "{:?}",
            out.diagnostics
        );
    }
    #[test]
    fn original_package_advice_distinguishes_queries_and_genuine_source_provisions() {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        assert_eq!(count("package provide csv\ncsv::join {a b}", "W120"), 1);
        assert_eq!(count("package provide csv 1\ncsv::join {a b}", "W120"), 0);
        assert_eq!(
            count(
                "namespace eval csv {proc join args {return local}}\ncsv::join {a b}",
                "W120"
            ),
            0
        );
        let source = "csv::join {a b}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == tcl_core_types::DiagCode::W120)
            .unwrap();
        let subject = diagnostic.registry_source().unwrap();
        assert_eq!(subject.required_package(), Some("csv"));
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::PackageRequirement
        );
        assert_eq!(diagnostic.required_package_key().unwrap().bytes(), b"csv");
        assert!(subject.words().matches_source(
            &tcl_lexer::SourceImage::document(source),
            result.body_lexer_config.unwrap()
        ));
    }

    #[test]
    fn package_source_advice_does_not_reparse_mutated_reporting_invocations_or_packages() {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        for source in ["csv::join {a b}", "csv::join {a b}\npackage require csv"] {
            let mut analyser = Analyser::new();
            let original = analyser.analyse(source, "tcl8.6");
            let expected = original
                .diagnostics
                .iter()
                .filter(|d| matches!(d.code.as_str(), "W120" | "H301"))
                .map(|d| (d.code, d.span))
                .collect::<Vec<_>>();
            assert_eq!(expected.len(), 1);
            analyser = analyser.with_resolved_input(original.resolved_input.clone().unwrap());
            analyser.result = original;
            for invocation in &mut analyser.result.command_invocations {
                invocation.name = "puts".to_owned();
            }
            for requirement in &mut analyser.result.package_requires {
                requirement.name = "wrong".to_owned();
            }
            analyser
                .result
                .diagnostics
                .retain(|d| !matches!(d.code.as_str(), "W120" | "H301"));
            let context = analyser.analysis_context();
            analyser.emit_missing_package_require_diagnostics(context.commands());
            analyser.emit_package_require_ordering_hints(context.commands());
            let actual = analyser
                .result
                .diagnostics
                .iter()
                .filter(|d| matches!(d.code.as_str(), "W120" | "H301"))
                .map(|d| (d.code, d.span))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    fn reviewed_command_suggestions_keep_original_word_extent_and_full_context() {
        // naming.diagnostic.original-command-suggestion-source
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-suggestion-source.md
        let source = "{puta} hi";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == tcl_core_types::DiagCode::W123)
            .unwrap();
        let subject = diagnostic.subject().unwrap();
        let fix = super::original_command_suggestion_fix(
            subject,
            "two words",
            result.body_lexer_config.unwrap(),
        )
        .unwrap();
        assert_eq!(&source[fix.span.as_range()], "{puta}");
        let image = tcl_lexer::SourceImage::document(&fix.new_text);
        let end = u32::try_from(image.len()).unwrap();
        let parsed = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, end),
            result.body_lexer_config.unwrap(),
        )
        .unwrap();
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].words.len(), 1);
        assert!(
            super::original_command_suggestion_fix(
                subject,
                "\u{1f642}",
                result.body_lexer_config.unwrap()
            )
            .is_none()
        );
        let mut analyser = Analyser::new();
        analyser.analyse("missing", "tcl8.4");
        let registry = analyser.analysis_context();
        let candidates = analyser
            .build_w123_known_names(registry.commands())
            .candidates;
        assert!(candidates.iter().any(|name| name == "puts"));
        assert!(!candidates.iter().any(|name| name == "dict"));
    }
}
