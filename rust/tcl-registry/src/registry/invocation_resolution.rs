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

//! Resolving a call to its command, subcommand and form: `ResolvedCall`, form picking and option terminators.

use super::CommandRegistry;
use crate::arity::Arity;
use crate::forms::CommandForm;
use crate::hooks::{AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::resolved_invocation::{
    InvocationResolutionUnresolved, ResolvedInvocation, ResolvedSubcommand,
    StructuredInvocationResolution, SubcommandResolution,
};
use crate::spec::{CommandSpec, SubCommand};
use crate::stamp_window::StampSelection;
use crate::types::VarWriteTyping;
use crate::{InvocationArguments, InvocationWords};
use tcl_dialect::model::{SurfaceQuery, surface_admits};

impl CommandRegistry {
    /// Resolve a concrete invocation to its target-neutral registry semantics.
    ///
    /// This is the common compiler entry point.  It retains the original head
    /// and argument spellings while selecting the registry command,
    /// subcommand, and form descriptors.  The returned projection contains no
    /// backend code-generation hook; target registries decide how to emit the
    /// already-resolved semantic operation later.
    ///
    /// Returns `None` when the command head is unknown or unavailable in
    /// `dialect`.
    #[must_use]
    pub fn resolve_invocation<'r, 'w>(
        &'r self,
        name: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_invocation(InvocationWords::literals(name, args), dialect)
            .resolved()
    }

    /// Readonly descriptor availability from this exact command store and query.
    /// The supplied descriptor has already been selected independently. Scoped
    /// schemas inherit their body owner's package floor through this same owner.
    #[must_use]
    pub fn source_descriptor_availability<'q>(
        &self,
        spec: &CommandSpec,
        query: Option<SurfaceQuery<'q>>,
    ) -> crate::resolved_invocation::InvocationAvailability<'q> {
        crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec(spec),
        }
    }

    /// Resolve source-aware invocation words to target-neutral registry
    /// semantics.
    ///
    /// Only a literal command head can select a registry command. Likewise,
    /// only a literal first argument can select a subcommand. Expanded and
    /// opaque arguments make form arity indeterminate, so no form is matched.
    /// A substituted non-expanded argument still contributes exactly one argv
    /// entry and may therefore participate in an arity-only form selection;
    /// its spelling is never inspected as a value.
    ///
    /// The compatibility [`Self::resolve_invocation`] adapter constructs an
    /// all-literal view without allocation.
    #[must_use]
    pub fn resolve_structured_invocation<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> StructuredInvocationResolution<'r, 'w> {
        let numbers = words
            .arguments()
            .dialect()
            .map(|dialect| dialect.numbers)
            .or_else(|| dialect.and_then(|query| self.control_numbers(Some(query)).syntax()))
            .or_else(|| self.profile().map(|profile| profile.grammar.numbers));
        let words = words.with_profile(self.profile());
        let dialect = dialect.or_else(|| {
            words
                .arguments()
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        let Some(name) = words.head_literal() else {
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::ComputedHead {
                    word_kind: words.head().kind(),
                },
            );
        };
        let spec = if dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, dialect)
        };
        let Some(spec) = spec else {
            if crate::trace_deferred_writes_any() {
                eprintln!(
                    "TCL_LSP_TRACE_METADATA_REGISTRY head={name:?} query={dialect:?} unresolved=UnknownLiteralHead"
                );
            }
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::UnknownLiteralHead { spelling: name },
            );
        };
        let availability = self.source_descriptor_availability(spec, dialect);
        StructuredInvocationResolution {
            invocation: Some(resolve_source_descriptor_invocation(
                spec,
                words,
                availability,
                numbers,
            )),
            unresolved: None,
        }
    }

    /// The one resolution the derived-query layer projects from
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *The
    /// derived-query layer*): `words` resolved under the surface query
    /// `ctx` fixes — its target profile's — so every query the resolution
    /// then answers (`clause_plan`, `option_effects`, `arg_roles`,
    /// `pattern_args`, `case_invocation`, `frame_effect`, `return_type`,
    /// `effects`, …) is asked under that one release. A context that names
    /// no profile resolves surface-blind.
    #[must_use]
    pub fn invocation<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        ctx: &crate::value_transfer::AnalysisContext,
    ) -> StructuredInvocationResolution<'r, 'w> {
        self.resolve_structured_invocation(words, ctx.surface_query())
    }

    /// Resolve a concrete call to its registry-described form.
    ///
    /// Given the command head `name` and the literal argument words
    /// `args`, returns a [`ResolvedCall`] describing which
    /// [`CommandSpec`] / [`SubCommand`] / [`CommandForm`] the call
    /// matches and which lowering / codegen hook applies. The
    /// returned reference borrows from the registry, so callers must
    /// not retain it across registry mutation.
    ///
    /// Returns `None` when the command is unknown to the registry.
    #[must_use]
    pub fn resolve_call<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'r>>,
    ) -> Option<ResolvedCall<'r>> {
        let selection = self.resolve_legacy_call_selection(name, args, dialect)?;
        let spec = selection.spec;
        let sub = selection.sub;
        let form = selection.form;

        // A stamp is read at the point the call is resolved at: a level whose
        // windows that point does not settle declines, and the call is
        // dispatched plain rather than by the level above's stamp.
        let point = dialect.as_ref();
        let command_codegen = spec.codegen_hook_selection(point);
        let command_inline = spec.inline_codegen_hook_selection(point);
        let mut resolved = ResolvedCall {
            spec,
            sub,
            form,
            availability: selection.availability,
            lowering_hook: spec.lowering_hook,
            codegen_hook: command_codegen.stamp(),
            inline_codegen_hook: command_inline.stamp(),
            analyser_hook: spec.analyser_hook,
        };

        if let Some(sub) = sub {
            resolved.lowering_hook = form
                .and_then(|f| f.lowering_hook)
                .or(sub.lowering_hook)
                .or(spec.lowering_hook);
            resolved.codegen_hook = StampSelection::stated(form.and_then(|f| f.codegen_hook))
                .or(sub.codegen_hook_selection(point))
                .or(command_codegen)
                .stamp();
            // Forms carry no inline hook — the inline emitters guard
            // their own applicability (arity / shape) at the dispatch
            // site, so subcommand-level wins over command-level.
            resolved.inline_codegen_hook = sub
                .inline_codegen_hook_selection(point)
                .or(command_inline)
                .stamp();
            // Forms carry no analyser hook either — the analyser
            // handlers keep their own shape guards, so the
            // subcommand-level stamp wins over the command-level one.
            resolved.analyser_hook = sub.analyser_hook.or(spec.analyser_hook);
            return Some(resolved);
        }

        if let Some(f) = form {
            resolved.lowering_hook = f.lowering_hook.or(spec.lowering_hook);
            resolved.codegen_hook = StampSelection::stated(f.codegen_hook)
                .or(command_codegen)
                .stamp();
            resolved.form = Some(f);
        }
        Some(resolved)
    }

    /// Select raw registry descriptors for the legacy call resolver.
    ///
    /// The exact subcommand lookup intentionally preserves the long-standing
    /// `resolve_call` compatibility behaviour.  The common
    /// [`ResolvedInvocation`] resolver instead uses
    /// [`CommandSpec::resolve_subcommand_word`] and records a typed outcome.
    fn resolve_legacy_call_selection<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'r>>,
    ) -> Option<InvocationSelection<'r>> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        let availability = crate::resolved_invocation::InvocationAvailability {
            query: dialect.or_else(|| self.own_surface_query()),
            package_version: self.package_floor_for_spec(spec),
        };

        if !spec.subcommands.is_empty()
            && let Some(first) = args.first()
            && let Some(sub) = spec.subcommand(first)
        {
            return Some(InvocationSelection {
                spec,
                sub: Some(sub),
                form: pick_form(
                    spec,
                    Some(sub),
                    InvocationArguments::literals(args),
                    availability,
                ),
                availability,
            });
        }

        Some(InvocationSelection {
            spec,
            sub: None,
            form: pick_form(
                spec,
                None,
                InvocationArguments::literals(args),
                availability,
            ),
            availability,
        })
    }

    /// Resolve the option-terminator profile for a command invocation.
    ///
    /// Matches the invocation's first argument against subcommands
    /// that declare an [`OptionSpec`](crate::hover::OptionSpec) with
    /// `name == "--"`, then falls back to form-level `--` declarations.
    /// Returns `None` when the command does not declare a `--`
    /// terminator at all (subcommand-scoped or form-scoped).
    ///
    /// Drives the W304 ("missing option terminator") diagnostic — the
    /// returned `scan_start` index, `subcommand` label, and
    /// `options` slice tell the caller where to start scanning for
    /// the first positional argument and which option specs to
    /// consult for value-consuming options (so they're not mistaken
    /// for positionals).  Returning a borrowed `&'static [OptionSpec]`
    /// rather than a freshly-allocated `HashSet` keeps the resolver
    /// allocation-free on the analyser hot path; per-command option
    /// counts are small (typically 1-3 value-consuming options per
    /// command), so a linear scan at the call site is cheaper than
    /// a `HashSet` build.
    ///
    #[must_use]
    pub fn resolve_option_terminator(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedTerminator> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        // Subcommand-scoped first. Resolve the subcommand word the same way
        // ensemble dispatch does — accepting a unique prefix abbreviation
        // (`string le` ⇒ `length`) — so an abbreviated subcommand keeps its
        // `--` terminator profile, matching `arg_indices_for_role`.
        if let Some(first) = args.first()
            && let Some(sub) = if dialect.is_none() {
                spec.resolve_subcommand(first)
            } else {
                spec.resolve_subcommand_for_dialect(first, dialect)
            }
            && sub.options.iter().any(|o| o.name == "--")
        {
            return Some(ResolvedTerminator {
                scan_start: 1,
                subcommand: Some(sub.name),
                options: sub.options,
                reserved_trailing_words: 0,
            });
        }

        // Form-level fallback — option specs live at the
        // `CommandSpec.options` level (a single set per spec), so we
        // consult that directly when no subcommand match was found.
        if spec.options.iter().any(|o| o.name == "--") {
            let effective_dialect = self.own_surface_query().or(dialect);
            let reserved_trailing_words =
                spec.case_list.map_or(spec.reserved_trailing_words, |case| {
                    case.option_scan_reserved_trailing_words(
                        args,
                        effective_dialect,
                        spec.reserved_trailing_words,
                    )
                });
            return Some(ResolvedTerminator {
                scan_start: 0,
                subcommand: None,
                options: spec.options,
                reserved_trailing_words,
            });
        }

        None
    }
}

/// Resolved option-terminator profile for a command invocation.
///
/// Returned by [`CommandRegistry::resolve_option_terminator`].  Drives
/// the W304 ("missing option terminator") diagnostic.  Carries
/// borrowed references into the registry's static spec table; do
/// not retain across registry mutation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminator {
    /// Index in the `args` slice where positional-argument scanning
    /// begins.  `0` for form-level matches; `1` for subcommand-scoped
    /// matches (the first arg is the subcommand keyword).
    pub scan_start: usize,
    /// Subcommand keyword that owns the `--` declaration, if the
    /// match was subcommand-scoped.  `None` for form-level matches.
    pub subcommand: Option<&'static str>,
    /// Borrowed slice of every option declared on the matched
    /// command (or subcommand).  Callers consult [`crate::hover::OptionSpec::takes_value`]
    /// on each entry to determine whether an option name consumes a
    /// following value argument — done at the call site to avoid the
    /// per-resolve `HashSet` allocation a precomputed name set would
    /// require.  Per-command counts are small; a linear scan is
    /// cheaper than a heap-allocated set on the analyser hot path.
    pub options: &'static [crate::hover::OptionSpec],
    /// Trailing words (after the command name) that are never scanned as
    /// option candidates — see [`crate::spec::CommandSpec::reserved_trailing_words`].
    /// `0` for every subcommand-scoped match (no subcommand currently
    /// needs the reservation) and for any form without one declared.
    pub reserved_trailing_words: usize,
}

/// Outcome of [`CommandRegistry::resolve_call`].
///
/// Carries borrowed references into the registry's spec table; the
/// resolved call describes the matched command spec, optionally a
/// matched subcommand and form, and the effective lowering / codegen
/// hook identifiers (form-level wins over subcommand-level wins
/// over command-level).
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCall<'r> {
    /// The matched top-level command spec.
    pub spec: &'r CommandSpec,
    /// The matched subcommand, if the call has one.
    pub sub: Option<&'r SubCommand>,
    /// The matched form descriptor, if any.
    pub form: Option<&'r CommandForm>,
    /// Option availability retained by the shared selection owner.
    pub availability: crate::resolved_invocation::InvocationAvailability<'r>,
    /// Effective lowering hook identifier.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective codegen hook identifier.
    pub codegen_hook: Option<CodegenHookId>,
    /// Effective inline (value-position / catch-body) codegen hook
    /// identifier (subcommand-level wins over command-level).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,
    /// Effective analyser handler-family hook identifier
    /// (subcommand-level wins over command-level).
    pub analyser_hook: Option<AnalyserHookId>,
}

/// Raw descriptor selection shared by the legacy and target-neutral
/// invocation-resolution facades.
#[derive(Debug, Clone, Copy)]
struct InvocationSelection<'r> {
    pub(super) spec: &'r CommandSpec,
    pub(super) sub: Option<&'r SubCommand>,
    pub(super) form: Option<&'r CommandForm>,
    availability: crate::resolved_invocation::InvocationAvailability<'r>,
}

impl ResolvedCall<'_> {
    /// Effective arity for this resolved call: the form arity if a
    /// form matched, otherwise the subcommand arity, otherwise the
    /// top-level [`CommandSpec`] arity.
    #[must_use]
    pub fn arity(&self) -> Arity {
        if let Some(f) = self.form {
            return f.arity;
        }
        if let Some(s) = self.sub {
            return s.arity;
        }
        self.spec.arity
    }

    /// Effective native arity using the actual retained argument dialect.
    /// A selected procedure-definition contract may differ from the legacy
    /// catalogue synopsis, for example Jim's optional statics operand.
    #[must_use]
    pub fn arity_for_arguments(&self, arguments: crate::InvocationArguments<'_>) -> Arity {
        self.spec
            .procedure_definition
            .filter(|_| self.sub.is_none())
            .map_or_else(
                || {
                    crate::native_list_assignment::operation_arity(
                        crate::resolved_invocation::resolved_operation(
                            self.spec,
                            self.sub,
                            self.form,
                            self.availability.query.as_ref(),
                        ),
                        arguments.dialect(),
                    )
                    .unwrap_or_else(|| self.arity())
                },
                |descriptor| descriptor.arity(arguments.dialect()),
            )
    }

    /// Count selected signature operands through the common option-layout owner.
    #[must_use]
    pub fn argument_count_for_arity(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<u16> {
        let options = crate::resolved_invocation::invocation_options(
            self.spec,
            self.sub,
            self.form,
            self.availability,
        );
        crate::resolved_invocation::count_invocation_arguments(
            self.arity_for_arguments(arguments),
            arguments,
            usize::from(self.sub.is_some()),
            options,
        )
    }

    /// Effective [`VarWriteTyping`] for this resolved call: the matched
    /// subcommand's when one matched (`binary scan` destructures where the
    /// bare `binary` does not), otherwise the top-level [`CommandSpec`]'s.
    ///
    /// The compiler's type-inference pass consults this to type the
    /// variables a command writes as a side effect, rather than assuming
    /// they receive the command's return value.
    #[must_use]
    pub fn var_write_typing(&self) -> VarWriteTyping {
        self.sub
            .map_or(self.spec.var_write_typing, |s| s.var_write_typing)
    }

    /// The result↔element-structure fact for this call — the subcommand's
    /// when one matched, else the command's. See
    /// [`crate::types::ReturnElements`].
    #[must_use]
    pub fn return_elements(&self) -> Option<crate::types::ReturnElements> {
        self.sub
            .map_or(self.spec.return_elements, |s| s.return_elements)
    }

    /// The in-place element evolution of the written variable for this call —
    /// the subcommand's when one matched, else the command's. See
    /// [`crate::types::VarElementsEffect`].
    #[must_use]
    pub fn var_elements_effect(&self) -> Option<crate::types::VarElementsEffect> {
        self.sub
            .map_or(self.spec.var_elements_effect, |s| s.var_elements_effect)
    }

    /// The clause plan of this call: the matched subcommand's grammar walked
    /// over the words after the subcommand word, else the command's over
    /// `args`, in the post-head coordinates of `args` — the plan
    /// [`CommandRegistry::clause_plan`] answers for the same descriptors.
    /// `args` are source spellings (a braced `{-}` is the fall-through
    /// marker). `None` when neither declares a grammar or it is unavailable
    /// at `dialect`.
    #[must_use]
    pub fn clause_plan(
        &self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::ClausePlan> {
        match self.sub {
            Some(sub) => sub
                .clause_plan(args.get(1..).unwrap_or_default(), dialect)
                .map(|plan| plan.offset_by(1)),
            None => self.spec.clause_plan(args, dialect),
        }
    }
}

pub(super) fn successful_handler_contracts(
    spec: &CommandSpec,
) -> impl Iterator<Item = crate::native_compilation::SuccessfulHandlerSpec> + '_ {
    spec.successful_handler
        .into_iter()
        .chain(
            spec.command_forms
                .iter()
                .filter_map(|form| form.successful_handler),
        )
        .chain(spec.subcommands.iter().flat_map(|member| {
            member.successful_handler.into_iter().chain(
                member
                    .subcommand_forms
                    .iter()
                    .filter_map(|form| form.successful_handler),
            )
        }))
}

/// Select the [`CommandForm`] whose arity, dialect, and optional literal
/// argument-prefix selector match the invocation.
///
/// **No lifecycle filter, because there is nothing to filter on.**
/// [`CommandForm`] — the *semantic* form, which routes lowering / codegen
/// hooks and per-form descriptors — carries no [`crate::lifecycle::Lifecycle`]
/// field; only its documentation counterpart [`crate::hover::FormSpec`] does,
/// and that is what [`CommandSpec::primary_synopsis`] and
/// [`CommandSpec::optional_trailing_arg_names`] gate. If a semantic form ever
/// gains a lifecycle, this is where the same `package_version: Option<&str>`
/// parameter belongs — the retained availability already carries the selected package floor.
pub(super) fn pick_form<'r>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    arguments: InvocationArguments<'_>,
    availability: crate::resolved_invocation::InvocationAvailability<'_>,
) -> Option<&'r CommandForm> {
    let forms = sub.map_or(spec.command_forms, |sub| sub.subcommand_forms);
    let argument_offset = usize::from(sub.is_some());
    let argument_count = arguments.exact_argv_len()?.checked_sub(argument_offset)?;
    let dialect = availability.query;
    let accepts = |form: &CommandForm| {
        crate::resolved_invocation::count_invocation_arguments(
            form.arity,
            arguments,
            argument_offset,
            crate::resolved_invocation::invocation_options(spec, sub, Some(form), availability),
        )
        .is_some_and(|count| form.arity.accepts(count))
    };
    let admits_dialect = |form: &CommandForm| !matches!(form.surface, Some(d) if dialect.is_some() && !surface_admits(d, dialect.as_ref()));
    // Resolve each selector word as its own Tcl keyword table. This matters
    // for nested operations: `tag c ...` is ambiguous between `cell` and
    // `configure` before a later word can be inspected, while `tag cell h`
    // first resolves `cell` exactly and then uniquely prefixes `has`.
    //
    // Keep walking after a selector completes: a sibling may extend it
    // (`tag` beside `tag bind`), and the longest statically matched selector
    // wins. A dynamic word while such an extension remains viable abstains;
    // it may turn out to be the longer selector at runtime.
    let mut active: Vec<&CommandForm> = forms
        .iter()
        .filter(|form| admits_dialect(form) && form.literal_argument_prefix.is_some())
        .collect();
    let mut selected = None;
    for selector_index in 0..argument_count {
        active.retain(|form| {
            form.literal_argument_prefix
                .is_some_and(|selector| selector.words.len() > selector_index)
        });
        if active.is_empty() {
            break;
        }
        let spelling = arguments.literal_at(argument_offset + selector_index)?;
        let mut has_exact_word = false;
        let mut abbreviated_word = None;
        let mut abbreviated_ambiguous = false;

        for form in &active {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            let canonical = selector.words[selector_index];
            if spelling == canonical {
                has_exact_word = true;
            } else if selector.prefix_matching.accepts_prefixes()
                && !spelling.is_empty()
                && canonical.starts_with(spelling)
            {
                if abbreviated_word.is_some_and(|word| word != canonical) {
                    abbreviated_ambiguous = true;
                } else {
                    abbreviated_word = Some(canonical);
                }
            }
        }

        let canonical = if has_exact_word {
            spelling
        } else if abbreviated_ambiguous {
            return None;
        } else if let Some(abbreviated) = abbreviated_word {
            abbreviated
        } else {
            break;
        };
        active.retain(|form| {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            selector.words[selector_index] == canonical
                && (spelling == canonical || selector.prefix_matching.accepts_prefixes())
        });
        if let Some(form) = active.iter().copied().find(|form| {
            accepts(form)
                && form
                    .literal_argument_prefix
                    .is_some_and(|selector| selector.words.len() == selector_index + 1)
        }) {
            selected = Some(form);
        }
    }
    if selected.is_some() {
        return selected;
    }

    let selector_overlaps_arity = forms.iter().any(|form| {
        admits_dialect(form) && accepts(form) && form.literal_argument_prefix.is_some()
    });
    if selector_overlaps_arity {
        return None;
    }
    forms.iter().find(|form| {
        admits_dialect(form) && accepts(form) && form.literal_argument_prefix.is_none()
    })
}

/// Resolve readonly invocation metadata from an independently selected descriptor.
/// The caller owns descriptor selection and actual availability. This does not
/// select a command by name or grant a native handler, entered frame or effect.
/// Registered and scoped source descriptors share this selector/form owner.
#[must_use]
pub fn resolve_source_descriptor_invocation<'r, 'w>(
    spec: &'r CommandSpec,
    words: InvocationWords<'w>,
    availability: crate::resolved_invocation::InvocationAvailability<'w>,
    numbers: Option<tcl_dialect::NumberSyntax>,
) -> ResolvedInvocation<'r, 'w> {
    let arguments = words.arguments();
    let (subcommand, sub) = resolve_semantic_subcommand(
        spec,
        arguments,
        availability.query,
        availability.package_version,
        numbers,
    );
    let form = match (sub, spec.subcommands.is_empty(), arguments.is_empty()) {
        (Some(sub), _, _) => pick_form(spec, Some(sub), arguments, availability),
        (None, true, _) | (None, false, true) => pick_form(spec, None, arguments, availability),
        (None, false, false) if subcommand == SubcommandResolution::NotApplicable => {
            pick_form(spec, None, arguments, availability)
        }
        (None, false, false) => None,
    };
    ResolvedInvocation::new(words, spec, sub, form, subcommand, availability, numbers)
}

/// Resolve a subcommand for the common semantic path.
///
/// The registry's keyword table determines prefix legality.  The legacy
/// [`ResolvedCall`] path intentionally remains exact-only because its callers
/// preserve historical hook-routing behaviour.
pub(super) fn resolve_semantic_subcommand<'r, 'w>(
    spec: &'r CommandSpec,
    arguments: InvocationArguments<'w>,
    dialect: Option<SurfaceQuery<'_>>,
    package_version: Option<&str>,
    numbers: Option<tcl_dialect::NumberSyntax>,
) -> (SubcommandResolution<'w>, Option<&'r SubCommand>) {
    if spec.subcommands.is_empty()
        || arguments.is_empty()
        || spec.constructor_prefix_words().is_some()
    {
        return (SubcommandResolution::NotApplicable, None);
    }

    let Some(spelling) = arguments.literal_at(0) else {
        let word_kind = arguments.get(0).map_or(
            crate::InvocationWordKind::Opaque,
            crate::InvocationWord::kind,
        );
        return (SubcommandResolution::Indeterminate { word_kind }, None);
    };
    if spec
        .default_form_first_word
        .is_some_and(|shape| shape.matches_syntax(spelling, numbers))
    {
        return (SubcommandResolution::NotApplicable, None);
    }
    let matched = spec.resolve_subcommand_word(spelling, dialect, package_version, None);
    match matched {
        crate::abbrev::KeywordMatch::Unique(canonical_name) => {
            // `resolve_subcommand_word` builds its table from `spec` itself,
            // so its canonical result is necessarily one of these entries.
            let sub = spec
                .subcommand(canonical_name)
                .expect("registry subcommand table and descriptor slice agree");
            let resolved = ResolvedSubcommand {
                spelling,
                canonical_name: sub.name,
            };
            let outcome = if spelling == sub.name {
                SubcommandResolution::Exact(resolved)
            } else {
                SubcommandResolution::UniquePrefix(resolved)
            };
            (outcome, Some(sub))
        }
        crate::abbrev::KeywordMatch::Ambiguous(_) => {
            (SubcommandResolution::Ambiguous { spelling }, None)
        }
        crate::abbrev::KeywordMatch::Unknown => (SubcommandResolution::Unknown { spelling }, None),
    }
}
