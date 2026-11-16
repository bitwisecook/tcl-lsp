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

//! Argument roles of a call: role assignments, type hints, format and pattern arguments, presentation, trailing roles and command prefixes.

use super::CommandRegistry;
use super::invocation_resolution::pick_form;
use crate::abbrev::PrefixMatching;
use crate::arg_role::{AppendedArity, ArgRole};
use crate::body_kind::BodyKind;
use crate::invocation_words::{CommandPrefixArguments, InvocationWord};
use crate::resolved_invocation::SubcommandResolution;
use crate::spec::{CommandSpec, SubCommand};
use crate::{InvocationArguments, InvocationWords};
use std::hash::Hash;
use tcl_dialect::model::SurfaceQuery;

/// Append `(index, role)` for the `args` indices consumed by value-taking
/// options whose value role (primary or secondary) `wanted` admits.
///
/// Walks `args` from `scan_start` (1 to skip a subcommand word, else 0),
/// resolving option names, aliases, and unique abbreviations through the
/// declared table — so `-1`, `$x`, `[cmd]`, and ambiguous abbreviations are
/// treated as positionals — and advancing past each recognised option and the
/// value word(s) it consumes ([`OptionSpec::value_indices`], which honours
/// arity and the `--` terminator). The emitted indices are absolute into `args`,
/// exactly like the positional roles, so consumers map them via `argv[idx + 1]`
/// unchanged. A two-way binding (`role: VarWrite, also_role: VarRead`) emits its
/// index under both roles — the multi-role convention.
pub(super) fn push_option_value_roles(
    out: &mut Vec<(usize, ArgRole)>,
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    wanted: &impl Fn(ArgRole) -> bool,
    prefix_matching: PrefixMatching,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            for role in [opt.value_role(), opt.value_also_role()]
                .into_iter()
                .flatten()
                .filter(|&role| wanted(role))
            {
                out.extend(vals.iter().map(|&value| (value, role)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// Collect option values whose role is [`ArgRole::CommandPrefix`], paired with
/// the option's [`AppendedArity`] — the option-side companion to
/// [`push_option_value_roles`], used by [`CommandRegistry::command_prefixes`].
fn push_command_prefix_options(
    out: &mut Vec<(usize, AppendedArity)>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(ArgRole::CommandPrefix) {
                let arity = opt.value_appended_arity();
                out.extend(vals.iter().map(|&v| (v, arity)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// The command-prefix positions of one call to `spec` (post-head
/// coordinates, the selecting word first) and the arity each receives
/// appended: `sub`'s own table when the call selected one — a subcommand, or
/// an instance method — else the command's (a resolver, else the static
/// table), plus every command-prefix option value. The one rule behind
/// [`CommandRegistry::command_prefixes`] and
/// [`crate::ResolvedInvocation::arg_roles`]; the caller selects `sub`.
pub(crate) fn command_prefixes_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: CommandPrefixArguments<'_>,
) -> Vec<(usize, AppendedArity)> {
    if let Some(sub) = sub {
        return sub_command_prefixes(sub, args.slice_from(1))
            .into_iter()
            .map(|(index, arity)| (index + 1, arity))
            .collect();
    }
    let Some(n) = args.words().exact_argv_len() else {
        return Vec::new();
    };
    let mut out: Vec<(usize, AppendedArity)> = Vec::new();
    if let Some(resolver) = spec.command_prefix_resolver {
        out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
    } else {
        out.extend(spec.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
    }
    if let Some(spellings) = args.spellings() {
        push_command_prefix_options(&mut out, spec.options, spellings, 0);
    }
    out.retain(|&(idx, _)| idx < n);
    out
}

/// The command-prefix positions of a subcommand's (or an instance method's)
/// own words — `args` are the words after its selecting word, and so are the
/// indices returned.
pub(super) fn sub_command_prefixes(
    sub: &SubCommand,
    args: CommandPrefixArguments<'_>,
) -> Vec<(usize, AppendedArity)> {
    let Some(n) = args.words().exact_argv_len() else {
        return Vec::new();
    };
    let mut out: Vec<(usize, AppendedArity)> = Vec::new();
    if let Some(resolver) = sub.command_prefix_resolver {
        out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
    } else {
        out.extend(sub.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
    }
    if let Some(spellings) = args.spellings() {
        push_command_prefix_options(&mut out, sub.options, spellings, 0);
    }
    out.retain(|&(idx, _)| idx < n);
    out
}

/// Put a role table in its one order — by position, and the roles one
/// position carries in [`ArgRole::ALL`] order — with each pair once.
pub(crate) fn sort_role_table(roles: &mut Vec<(usize, ArgRole)>) {
    roles.sort_by_key(|&(index, role)| {
        (
            index,
            ArgRole::ALL
                .iter()
                .position(|&known| known == role)
                .unwrap_or(usize::MAX),
        )
    });
    roles.dedup();
}

/// The pattern-bearing arguments of one call to `spec` — the one rule behind
/// [`CommandRegistry::pattern_args_for_dialect`] and
/// [`crate::ResolvedInvocation::pattern_args`]. `options` supplies the
/// option table available at `dialect`, read only by an option-selected
/// layout; `sub` is the caller's selection for `args[0]`, whose pattern
/// language overrides the command's; a static layout's positions are the
/// caller's `ArgRole::Pattern` answer.
pub(crate) fn pattern_args_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: &[&str],
    options: impl FnOnce() -> Vec<&'static crate::hover::OptionSpec>,
    dialect: Option<SurfaceQuery<'_>>,
    static_pattern_indices: impl FnOnce() -> Vec<usize>,
) -> Vec<crate::patterns::PatternArg> {
    if spec.pattern_arg_resolver.is_some() || spec.option_selects_pattern_language() {
        let options = options();
        if let Some(resolve) = spec.pattern_arg_resolver {
            return resolve(
                args,
                crate::patterns::PatternArgResolverContext {
                    options: &options,
                    reserved_trailing_words: spec.reserved_trailing_words,
                },
            );
        }
        let effects =
            spec.option_effects_over(&options, InvocationArguments::literals(args), dialect);
        return crate::patterns::option_selected_pattern_args(
            &effects,
            spec.reserved_trailing_words,
            args.len(),
        );
    }
    let Some(kind) = sub.and_then(|sub| sub.pattern_type).or(spec.pattern_type) else {
        return Vec::new();
    };
    static_pattern_indices()
        .into_iter()
        .filter_map(|index| u8::try_from(index).ok())
        .map(|index| crate::patterns::PatternArg { index, kind })
        .collect()
}

/// Whether source words prove the option layout a resolver-derived role of
/// `spec` (or its selected `sub`) depends on — the one rule behind the
/// registry's source-aware role, pattern and format queries and the
/// resolution's derived queries. The option tables are the caller's,
/// filtered to its release.
pub(crate) fn layout_is_proven_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: InvocationArguments<'_>,
    spec_options: impl FnOnce() -> Vec<&'static crate::hover::OptionSpec>,
    sub_options: impl FnOnce(&SubCommand) -> Vec<&'static crate::hover::OptionSpec>,
) -> bool {
    if !args.has_exact_argv_len() {
        return false;
    }
    // Static role positions and generic option-value roles remain stable
    // once expansion is excluded. Only a resolver can reinterpret a
    // source word as a positional operand, so a `clock format $time
    // -format %Y` time value does not suppress the independently-owned
    // `-format` value role.
    let resolver_depends_on_options = sub.map_or(
        (spec.arg_role_count_resolver.is_none()
            && spec.arg_role_layout_resolver.is_none()
            && spec.arg_role_resolver.is_some())
            || spec.pattern_arg_resolver.is_some()
            || spec.option_selects_pattern_language(),
        |sub| {
            sub.arg_role_count_resolver.is_none()
                && sub.arg_role_layout_resolver.is_none()
                && sub.arg_role_resolver.is_some()
        },
    );
    if !resolver_depends_on_options {
        return true;
    }
    let (options, option_args, prefix_matching, reserved_trailing_words) = match sub {
        Some(sub) => (sub_options(sub), args.slice_from(1), sub.prefix_matching, 0),
        None => (
            spec_options(),
            args,
            spec.prefix_matching,
            spec.reserved_trailing_words,
        ),
    };
    options.is_empty()
        || source_option_layout_is_proven(
            &options,
            option_args,
            prefix_matching,
            reserved_trailing_words,
        )
}

/// The options of `sub` (of `spec`) available at `dialect`, in declaration
/// order — the profile-less option table the layout proof reads for a
/// selected subcommand.
pub(crate) fn sub_options_at(
    spec: &CommandSpec,
    sub: &SubCommand,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<&'static crate::hover::OptionSpec> {
    sub.options
        .iter()
        .filter(|option| option.supports_dialect(dialect, sub.surface.or(spec.surface)))
        .collect()
}

/// Whether source words prove a command's leading option layout.
///
/// A literal option consumes its descriptor-declared values before scanning
/// continues. A dynamic word while that scan is live might evaluate to a
/// value-taking option, so every resolver-derived positional role must
/// abstain. A literal invalid or ambiguous dash word is likewise not a
/// positional operand: treating it as one would describe an invocation Tcl
/// rejects. The profile-filtered descriptor table, its abbreviation policy,
/// and the command's reserved trailing operands are inputs, so every pattern
/// and format consumer makes this decision from the same owner data.
fn source_option_layout_is_proven(
    options: &[&crate::hover::OptionSpec],
    args: InvocationArguments<'_>,
    prefix_matching: PrefixMatching,
    reserved_trailing_words: usize,
) -> bool {
    crate::spec::leading_option_word_count_for_arguments(
        options,
        args,
        prefix_matching,
        reserved_trailing_words,
    )
    .is_some()
}

impl CommandRegistry {
    /// The declared [`ArgTypeHint`] for the 0-based argument `index` (after
    /// the command name) of a call to `name` with `args`.
    ///
    /// Resolves through the subcommand when the command is an ensemble and
    /// `args[0]` names one, so `dict for {k v} $d body` reports the `Dict`
    /// hint the `for` subcommand declares on its own argument 1.
    ///
    /// [`ArgTypeHint`]: crate::hooks::ArgTypeHint
    #[must_use]
    pub fn arg_type_hint(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> Option<&'static crate::hooks::ArgTypeHint> {
        let spec = self.get(name)?;
        let find = |types: &'static [(u8, crate::hooks::ArgTypeHint)], idx: usize| {
            u8::try_from(idx)
                .ok()
                .and_then(|i| types.iter().find(|(at, _)| *at == i).map(|(_, h)| h))
        };
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
            && index >= 1
        {
            return find(sub.arg_types, index - 1);
        }
        find(spec.arg_types, index)
    }

    /// Selected argument representation hint without fabricating dynamic values.
    /// Literal subcommands select their authored positional table. Unresolved
    /// leading option operands retain uncertainty about the positional offset.
    #[must_use]
    pub fn arg_type_hint_words(
        &self,
        words: crate::InvocationWords<'_>,
        index: usize,
    ) -> Option<&'static crate::hooks::ArgTypeHint> {
        let arguments = words.arguments();
        let query = arguments
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query);
        let spec = self.get_for_surface(words.head_literal()?, query)?;
        let find = |types: &'static [(u8, crate::hooks::ArgTypeHint)], index| {
            let index = u8::try_from(index).ok()?;
            types
                .iter()
                .find(|(at, _)| *at == index)
                .map(|(_, hint)| hint)
        };
        if spec.subcommands.is_empty() {
            let types = pick_form(
                spec,
                None,
                arguments,
                crate::resolved_invocation::InvocationAvailability {
                    query,
                    package_version: self.package_floor_for_spec(spec),
                },
            )
            .and_then(|form| form.arg_types)
            .unwrap_or(spec.arg_types);
            return find(types, index);
        }
        let sub = spec.resolve_subcommand(arguments.literal_at(0)?)?;
        let availability = crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec(spec),
        };
        let form = pick_form(spec, Some(sub), arguments, availability);
        let types = form
            .and_then(|form| form.arg_types)
            .unwrap_or(sub.arg_types);
        let relative = index.checked_sub(1)?;
        let options =
            crate::resolved_invocation::invocation_options(spec, Some(sub), form, availability);
        let skipped = options.leading_word_count(arguments.slice_from(1))?;
        find(types, relative.checked_sub(skipped)?)
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`.
    ///
    /// Four role sources feed this, in the order the registry contract
    /// documents: the [`CommandSpec::clause_grammar`] walk's flat roles (the
    /// clause structure — keywords, conditions, scripts), a dynamic
    /// `arg_role_resolver` or else the static `arg_roles` table, and — for
    /// the unbounded regular tails a fixed table cannot express — the
    /// [`RepeatedArgLayout`]s of [`CommandSpec::repeated_args`].  The
    /// grammar and the repeated layouts are *additive*: a spec may pin its
    /// leading words with `arg_roles` (`namespace upvar`'s leading namespace
    /// word) and still declare the repeating pair tail, and `catch` states
    /// its clause structure in a grammar while its `arg_roles` keep the
    /// result words' `VarWrite`.
    ///
    /// [`RepeatedArgLayout`]: crate::repeated::RepeatedArgLayout
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        // `CommandPrefix` positions (with their appended arities) are owned by
        // [`Self::command_prefixes`]; delegate so highlighting, param-trait
        // inference, and the call-reference extractor all read one source.
        if role == ArgRole::CommandPrefix {
            return self
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
        }
        self.arg_role_assignments(name, args, &[role])
            .into_iter()
            .map(|(index, _)| index)
            .collect()
    }

    /// The `(index, role)` pairs a call gives any of `wanted`, from the
    /// sources [`Self::arg_indices_for_role`] documents, through the one rule
    /// [`arg_roles_in`] states. One walk answers every wanted role, so a
    /// consumer that needs several (a definer's name, parameter list and
    /// body) asks once.
    ///
    /// [`ArgRole::CommandPrefix`] is not answered here; see
    /// [`Self::command_prefixes`].
    fn arg_role_assignments(
        &self,
        name: &str,
        args: &[&str],
        wanted: &[ArgRole],
    ) -> Vec<(usize, ArgRole)> {
        // The words carry no engine of their own, so the registry's query
        // selects (see `arg_role_assignments_words_for_dialect`).
        self.arg_role_assignments_words(name, InvocationArguments::literals(args), wanted)
            .unwrap_or_default()
    }

    /// Project selected roles once, retaining the execution dialect and
    /// command/subcommand/form precedence of structured resolution.
    /// Unknown source positions return `None`; computed ordinary values never
    /// become literal selectors. This is descriptor metadata, not handler
    /// binding or execution evidence. Command-prefix appended arity remains
    /// owned by [`Self::arg_indices_for_role_words`].
    #[must_use]
    pub fn arg_role_assignments_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        wanted: &[ArgRole],
    ) -> Option<Vec<(usize, ArgRole)>> {
        self.arg_role_assignments_words_for_dialect(name, args, wanted, self.own_surface_query())
    }

    fn arg_role_assignments_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        wanted: &[ArgRole],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        // A caller's own engine selects its query. Otherwise the registry's
        // query selects, with the packages its profile loaded: the engine
        // projection of that profile carries none, so it would refuse a
        // package's own statements (`iapp::conf`, a `SpecTcl` block).
        let query = args
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or(dialect);
        let args = args.with_profile(self.profile());
        let words = InvocationWords::from_arguments(InvocationWord::Literal(name), args);
        let resolved = self
            .resolve_structured_invocation(words, query)
            .resolved()?;
        Self::selected_argument_role_assignments(wanted, &resolved)
    }

    /// The clause plan of a call to `name` with `args` — the command's (or,
    /// when `args[0]` names one, the subcommand's) clause grammar walked under
    /// this registry's own profile, in the same post-head coordinates as
    /// [`Self::arg_indices_for_role`]. `None` when neither declares a grammar
    /// or it is unavailable at the profile.
    #[must_use]
    pub fn clause_plan(&self, name: &str, args: &[&str]) -> Option<crate::ClausePlan> {
        let spec = self.get(name)?;
        let dialect = self.own_surface_query();
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
        {
            return sub
                .clause_plan(&args[1..], dialect)
                .map(|plan| plan.offset_by(1));
        }
        spec.clause_plan(args, dialect)
    }

    /// The structural defect a consumer reports for a call to `name` in place
    /// of the generic arity check — [`CommandSpec::clause_shape_defect`] under
    /// this registry's own profile (`if`'s E004).
    #[must_use]
    pub fn clause_shape_defect(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::ClauseShapeError> {
        self.get(name)?
            .clause_shape_defect(args, self.own_surface_query())
    }

    /// Resolve the argument indices whose **brace-quoted** word this command
    /// nonetheless evaluates in the *calling* frame — `expr {$a} + 1`,
    /// `if {$c} …` — so a `$name` written inside one is a genuine read at
    /// the call site.
    ///
    /// Tcl substitutes nothing inside `{…}`; whether the callee then
    /// evaluates that text, and in whose frame, is the per-role fact
    /// [`ArgRole::braced_word_evaluated_in_frame`] owns. This is the one
    /// place that asks it over a call's argument words, so the SSA use
    /// classifier and the shimmer detectors share a single answer instead of
    /// each rebuilding the set.
    #[must_use]
    pub fn arg_indices_evaluated_in_frame(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let roles: Vec<_> = ArgRole::ALL
            .iter()
            .copied()
            .filter(|role| role.braced_word_evaluated_in_frame())
            .collect();
        let mut out: Vec<_> = self
            .arg_role_assignments(name, args, &roles)
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Resolve argument-role positions from source-aware invocation words.
    ///
    /// `None` means Tcl expansion, a computed subcommand, or an option-driven
    /// layout prevents a sound source-position projection. Dynamic ordinary
    /// operands remain usable: they occupy exactly one argv position and are
    /// represented by an inert placeholder while the registry-owned layout is
    /// resolved.
    #[must_use]
    pub fn arg_indices_for_role_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        role: ArgRole,
    ) -> Option<Vec<usize>> {
        if role == ArgRole::CommandPrefix {
            // Preserve the callback owner's appended-arity contract after the
            // same selected descriptor has proved the source layout.
            self.arg_role_assignments_words(name, args, &[])?;
            let spellings: Vec<_> = (0..args.len())
                .map(|index| args.literal_at(index).unwrap_or(""))
                .collect();
            let words: Vec<_> = (0..args.len())
                .filter_map(|index| args.get(index))
                .collect();
            return Some(
                self.command_prefixes_structured(name, &spellings, &words)
                    .into_iter()
                    .map(|(index, _)| index)
                    .collect(),
            );
        }
        self.arg_role_assignments_words(name, args, &[role])
            .map(|roles| roles.into_iter().map(|(index, _)| index).collect())
    }

    /// Whether any registry-owned layout for `name` can assign `role`.
    ///
    /// Dynamic role hooks are conservatively capable of every role; this is
    /// used only when source expansion makes their concrete answer unknowable.
    #[must_use]
    pub fn may_have_arg_role(&self, name: &str, role: ArgRole) -> bool {
        let Some(spec) = self.get(name) else {
            return false;
        };
        Self::spec_may_have_arg_role(spec, role)
    }

    /// Pure role capabilities of an independently selected command schema.
    /// Callers retain availability and context selection; this query does not
    /// select a spelling again or establish execution of a script operand.
    #[must_use]
    pub fn spec_may_have_arg_role(spec: &CommandSpec, role: ArgRole) -> bool {
        let options_have = |options: &[crate::hover::OptionSpec]| {
            options.iter().any(|option| {
                option.value_role() == Some(role) || option.value_also_role() == Some(role)
            })
        };
        spec.arg_role_resolver_roles.contains(&role)
            || spec.arg_roles.iter().any(|(_, found)| *found == role)
            || spec
                .clause_grammar
                .is_some_and(|grammar| grammar.may_assign(role))
            || spec.repeated_args.iter().any(|layout| layout.role == role)
            || options_have(spec.options)
            || spec.command_forms.iter().any(|form| {
                form.arg_roles.iter().any(|(_, found)| *found == role) || options_have(form.options)
            })
            || spec.subcommands.iter().any(|sub| {
                sub.arg_role_resolver_roles.contains(&role)
                    || sub.arg_roles.iter().any(|(_, found)| *found == role)
                    || sub
                        .clause_grammar
                        .is_some_and(|grammar| grammar.may_assign(role))
                    || sub.repeated_args.iter().any(|layout| layout.role == role)
                    || options_have(sub.options)
            })
    }

    /// Resolve a subcommand only when its source word has a known literal
    /// value. A dynamic selector cannot be projected into a selected
    /// descriptor's argument layout.
    fn source_selected_subcommand<'a>(
        spec: &'a CommandSpec,
        args: InvocationArguments<'_>,
    ) -> Option<&'a SubCommand> {
        (!spec.subcommands.is_empty() && spec.constructor_prefix_words().is_none())
            .then(|| {
                args.literal_at(0)
                    .and_then(|word| spec.resolve_subcommand(word))
            })
            .flatten()
    }

    /// The format-string words of **one concrete call**: which argument
    /// positions carry a conversion / field string, and which mini-language
    /// each is written in.
    ///
    /// The single registry answer, in place of matching command spellings —
    /// `match head { "format" => …, "scan" =>
    /// …, "clock" => …, "binary" => …, "regsub" => … }` — in both the
    /// semantic-token walk and the inlay-hint collector, each with its own
    /// copy of the argument layout. It combines the two facts
    /// the registry already models:
    ///
    /// * **Where** — the [`ArgRole::FormatString`] / [`ArgRole::ScanFormat`]
    ///   positions of the call, resolved through
    ///   [`Self::arg_indices_for_role`], so a fixed index (`format`), a
    ///   resolver-computed one (`scan`, `regsub` past its switches), a
    ///   subcommand-relative one (`binary format`), and an **option value**
    ///   (`clock format … -format FMT`) all answer the same way.
    /// * **Which language** — [`CommandSpec::format_string_type`], overridden
    ///   by [`SubCommand::format_string_type`] when a subcommand dispatches
    ///   (`clock format` ⇒ `Clock`, `binary scan` ⇒ `Binary`).
    ///
    /// Because the head is resolved through [`Self::get`], the explicitly
    /// global spellings (`::format`, `::clock`, …) answer identically to the
    /// bare ones — the false negative the spelling tests had. A command with
    /// no declared family answers empty, so a same-named user proc, an
    /// unknown command, or a dynamic head is never misread.
    ///
    /// `args` is the post-head argument list, subcommand first, the same
    /// shape [`Self::arg_indices_for_role`] takes; returned indices are into
    /// that list.
    #[must_use]
    pub fn format_string_args(&self, name: &str, args: &[&str]) -> Vec<FormatStringArg> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        // A dispatching subcommand's family wins over the parent's.
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let Some(kind) = sub
            .and_then(|s| s.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        let mut out: Vec<FormatStringArg> = Vec::new();
        for (role, scan) in [(ArgRole::FormatString, false), (ArgRole::ScanFormat, true)] {
            out.extend(
                self.arg_indices_for_role(name, args, role)
                    .into_iter()
                    .map(|index| FormatStringArg { index, kind, scan }),
            );
        }
        out.sort_by_key(|f| f.index);
        out
    }

    /// Source-aware counterpart to [`Self::format_string_args`].
    ///
    /// Unlike the compatibility string query, this method retains whether
    /// each source word is literal, dynamic, expanded, or opaque. The format
    /// family still comes entirely from the registry, but a dynamic leading
    /// word is never reinterpreted as a positional pattern or replacement
    /// before the profile-filtered option grammar has resolved it. This is
    /// the owner query for editors and other source-aware consumers.
    #[must_use]
    pub fn format_string_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<FormatStringArg> {
        self.format_string_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::format_string_args_words`].
    #[must_use]
    pub fn format_string_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<FormatStringArg> {
        let effective_dialect = self.own_surface_query().or(dialect);
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        // A dynamic subcommand cannot select a concrete format family.
        if !spec.subcommands.is_empty() && !args.is_empty() && args.literal_at(0).is_none() {
            return Vec::new();
        }
        let sub = Self::source_selected_subcommand(spec, args);
        let Some(kind) = sub
            .and_then(|sub| sub.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        let Some(roles) = self.arg_role_assignments_words_for_dialect(
            name,
            args,
            &[ArgRole::FormatString, ArgRole::ScanFormat],
            effective_dialect,
        ) else {
            return Vec::new();
        };
        let mut out = roles
            .into_iter()
            .map(|(index, role)| FormatStringArg {
                index,
                kind,
                scan: role == ArgRole::ScanFormat,
            })
            .collect::<Vec<_>>();
        out.sort_by_key(|format| format.index);
        out
    }

    /// The pattern-bearing arguments of one concrete invocation.
    ///
    /// This pairs each declared position with its embedded language. A static
    /// command derives the positions from [`ArgRole::Pattern`]; an
    /// option-selected command (`lsearch -regexp`) answers the projection of
    /// its option effects onto the pattern-language axis, and a layout no
    /// axis can express keeps the registry resolver escape hatch. Consumers
    /// therefore never need command-name or `-regexp` branches of their own.
    #[must_use]
    pub fn pattern_args(&self, name: &str, args: &[&str]) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args`].
    ///
    /// A profile-bound registry always uses its attached profile.  A
    /// profile-less registry uses `dialect`, which is how editor consumers
    /// carrying one shared registry keep option-selected pattern layouts in
    /// sync with the current document's Tcl release.
    #[must_use]
    pub fn pattern_args_for_dialect(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let effective_dialect = self.own_surface_query().or(dialect);
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        pattern_args_in(
            spec,
            Self::source_selected_subcommand(spec, InvocationArguments::literals(args)),
            args,
            || {
                self.profile().map_or_else(
                    || spec.option_specs(effective_dialect),
                    |profile| {
                        crate::profile_queries::ProfileQueries::available_option_specs(
                            profile, spec,
                        )
                    },
                )
            },
            effective_dialect,
            || self.arg_indices_for_role(name, args, ArgRole::Pattern),
        )
    }

    /// Source-aware counterpart to [`Self::pattern_args`].
    ///
    /// A pattern descriptor may use a dedicated pattern resolver, a dynamic
    /// argument-role resolver, or static roles. Every option-dependent form
    /// goes through the same source-layout proof before the compatibility
    /// resolver sees a spelling projection, so `regexp $mode …`,
    /// `glob $mode …`, and `string match $mode …` all abstain consistently.
    #[must_use]
    pub fn pattern_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args_words`].
    #[must_use]
    pub fn pattern_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let query = self.own_surface_query().or(dialect);
        let words = InvocationWords::from_arguments(
            InvocationWord::Literal(name),
            args.with_profile(self.profile()),
        );
        self.resolve_structured_invocation(words, query)
            .resolved()
            .and_then(|resolved| resolved.authored_source_pattern_arguments())
            .unwrap_or_default()
    }

    /// How a formatter should **present** argument `index` of a call to
    /// `name` — the layout fact that refines the argument's semantic
    /// [`ArgRole`].
    ///
    /// Returns the declared override, or [`ArgPresentation::BlockScript`]
    /// (the default) when the spec says nothing. `for`'s `start` and `next`
    /// scripts answer [`ArgPresentation::InlineScript`], which is how the
    /// formatting engine keeps them on the header line without a
    /// `name == "for"` branch.
    ///
    /// Resolves through [`Self::get`], so the explicitly-global spelling
    /// `::for` answers identically to the bare `for` — the false negative
    /// the formatter's literal name comparison had. A command the registry
    /// does not know (a user proc, a dynamic head) answers the default,
    /// which is what leaves such a call formatted like any other command.
    ///
    /// `args` is the post-name argument list, in the same shape
    /// [`Self::arg_indices_for_role`] takes (subcommand first), so a
    /// subcommand-dispatching call resolves against the subcommand's own
    /// table with the same `+1` offset.
    #[must_use]
    pub fn arg_presentation(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> crate::presentation::ArgPresentation {
        use crate::presentation::ArgPresentation;
        let Some(spec) = self.get(name) else {
            return ArgPresentation::default();
        };
        // A dispatching subcommand owns the positions after its own word.
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
            && index > 0
            && let Ok(sub_index) = u8::try_from(index - 1)
        {
            return sub
                .arg_presentation
                .iter()
                .find(|(i, _)| *i == sub_index)
                .map_or_else(ArgPresentation::default, |(_, p)| *p);
        }
        let Ok(idx) = u8::try_from(index) else {
            return ArgPresentation::default();
        };
        spec.arg_presentation
            .iter()
            .find(|(i, _)| *i == idx)
            .map_or_else(ArgPresentation::default, |(_, p)| *p)
    }

    /// The declared roles of the argument positions a call has **not**
    /// filled — the optional trailing words a caller could still append.
    ///
    /// [`Self::arg_indices_for_role`] answers "what role does each supplied
    /// argument play"; this answers the complementary question a
    /// splice-a-trailing-word quick-fix needs: "what would the *next* words
    /// mean if I added them".  Positions are returned in ascending order,
    /// starting at `args.len()` and stopping at the spec's arity ceiling, and
    /// only while the spec actually declares a role for the position — an
    /// unlimited-arity command with no declared tail role yields nothing
    /// rather than an unbounded run of `Value` slots.
    ///
    /// A subcommand-dispatching call resolves against the subcommand's own
    /// role table (offset by the subcommand word), exactly as
    /// [`Self::arg_indices_for_role`] does, so `chan gets`-shaped commands
    /// answer for the right spec.  Dynamic
    /// [`arg_role_resolver`](CommandSpec::arg_role_resolver) tables are
    /// consulted with the supplied argument list, which is what a resolver
    /// keyed on the *written* words can answer; positions the resolver does
    /// not name are simply absent from the result.
    #[must_use]
    pub fn unfilled_trailing_roles(&self, name: &str, args: &[&str]) -> Vec<(usize, ArgRole)> {
        self.unfilled_trailing_roles_words(
            name,
            InvocationArguments::literals(args).with_profile(self.profile()),
        )
    }

    /// Prospective trailing roles under the retained argument grammar. Added
    /// words have unknown values and ordinary cardinality; original dynamic
    /// words and expansions never become compatibility literal placeholders.
    #[must_use]
    pub fn unfilled_trailing_roles_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<(usize, ArgRole)> {
        let args = args.with_profile(self.profile());
        if args.exact_argv_len() != Some(args.len()) {
            return Vec::new();
        }
        let query = args
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or_else(|| self.own_surface_query());
        let resolution = self.resolve_structured_invocation(
            InvocationWords::from_arguments(InvocationWord::Literal(name), args),
            query,
        );
        let Some(resolved) = resolution.resolved() else {
            return Vec::new();
        };
        if !matches!(
            resolved.subcommand,
            SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
                | SubcommandResolution::NotApplicable
        ) {
            return Vec::new();
        }
        let semantics = &resolved.semantics;
        let offset = semantics.argument_offset;
        let Some(counted) = resolved.argument_count_for_arity() else {
            return Vec::new();
        };
        let ignored = args.len().saturating_sub(usize::from(counted));
        let ceiling = usize::from(semantics.arity.max).saturating_add(ignored);
        if semantics.arg_role_layout_resolver.is_some() {
            return self.prospective_layout_roles(name, args, ceiling, query);
        }
        let declared = if let Some(resolve) = semantics.arg_role_count_resolver {
            resolve(args.len().saturating_sub(offset))
        } else if let Some(resolve) = semantics.arg_role_resolver {
            let Some(values) = args.slice_from(offset).literal_values() else {
                return Vec::new();
            };
            resolve(&values)
        } else {
            semantics.arg_roles.to_vec()
        };
        (args.len()..ceiling)
            .map_while(|position| {
                declared.iter().find_map(|&(index, role)| {
                    (usize::from(index) + offset == position).then_some((position, role))
                })
            })
            .collect()
    }

    fn prospective_layout_roles(
        &self,
        name: &str,
        original: InvocationArguments<'_>,
        ceiling: usize,
        query: Option<SurfaceQuery>,
    ) -> Vec<(usize, ArgRole)> {
        let Some(mut arguments) = (0..original.len())
            .map(|index| original.get(index))
            .collect::<Option<Vec<_>>>()
        else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for position in original.len()..ceiling.min(usize::from(u8::MAX) + 1) {
            arguments.push(InvocationWord::Dynamic);
            let words = InvocationArguments::structured(&arguments);
            let words = original
                .dialect()
                .map_or(words, |dialect| words.with_dialect(dialect));
            let resolution = self.resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), words),
                query,
            );
            let Some(resolved) = resolution.resolved() else {
                break;
            };
            let (roles, complete) = resolved.argument_roles();
            if !complete {
                break;
            }
            let offset = resolved.semantics.argument_offset;
            let Some(role) = roles.iter().find_map(|&(index, role)| {
                (usize::from(index) + offset == position).then_some(role)
            }) else {
                break;
            };
            result.push((position, role));
        }
        result
    }

    /// [`Self::arg_indices_for_role`]`(name, args, `[`ArgRole::Body`]`)`,
    /// filtered to the indices whose [`BodyKind`] is
    /// [`Plain`](BodyKind::Plain) — a body that runs in the caller's own
    /// frame (`if`/`while`/`for`/`foreach`/`switch`/`try`/`catch`/`eval`, …).
    ///
    /// `Structural` bodies (`proc`, `oo::class create`, `oo::define`,
    /// `snit::method`, `uplevel`, `namespace eval`, …) execute in a
    /// definition or different-frame dispatch context, so are excluded —
    /// a command found inside one is *not* still running in the enclosing
    /// call's scope.  This is the generic test for "is a dispatch nested in
    /// this body argument still the same lexical/dispatch context as the
    /// caller": intra-class `my`/`next`/`nextto` `TclOO` dispatch and `$obj`
    /// method dispatch recursion both need this to decide which nested
    /// script regions still belong to the same enclosing method.
    ///
    /// Respects a subcommand's own `body_kind` override (e.g. a compound
    /// command whose subcommand's body is structural even though the
    /// top-level spec defaults to `Plain`), the same resolution order
    /// [`Self::arg_indices_for_role`] uses for the subcommand's other roles.
    #[must_use]
    pub fn plain_body_arg_indices(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind != BodyKind::Plain {
            return Vec::new();
        }
        self.arg_indices_for_role(name, args, ArgRole::Body)
    }

    /// Resolve [`ArgRole::CommandPrefix`] argument positions and their
    /// [`AppendedArity`] for a concrete call.
    ///
    /// The single source of truth for command-prefix callbacks — unions the
    /// three declaration mechanisms (static [`CommandSpec::command_prefixes`]
    /// table, [`CommandSpec::command_prefix_resolver`], and `command_prefix`
    /// option values), for the top-level command or its resolved subcommand.
    /// Mirrors [`Self::arg_indices_for_role`]'s subcommand offset / `--`
    /// handling. Consumers: highlighting, find-references / call-hierarchy /
    /// call-graph recording, and the callback-arity check.
    ///
    /// For subcommand-based commands pass the subcommand as `args[0]`.
    #[must_use]
    pub fn command_prefixes(&self, name: &str, args: &[&str]) -> Vec<(usize, AppendedArity)> {
        self.command_prefixes_with_arguments(name, CommandPrefixArguments::literals(args))
    }

    /// Source-aware command-prefix resolution.
    ///
    /// `spellings` supports existing position-only declarations; `words`
    /// carries the proof boundary for literal-sensitive arity resolvers. The
    /// slices must describe the same post-head source words.
    #[must_use]
    pub fn command_prefixes_structured<'w>(
        &self,
        name: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.command_prefixes_with_arguments(name, arguments)
    }

    fn command_prefixes_with_arguments(
        &self,
        name: &str,
        args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        command_prefixes_in(
            spec,
            Self::source_selected_subcommand(spec, args.words()),
            args,
        )
    }
}

/// One format-string word of a concrete call — the answer
/// [`CommandRegistry::format_string_args`] returns.
///
/// The two facts a consumer needs are deliberately separate: `kind` names the
/// mini-language (a `clock` field string and a `format` %-string share
/// neither syntax nor version gates), while `scan` says which *direction* it
/// is written in (`format` and `scan` share the `Sprintf` family but not its
/// conversion set — `%b` is 8.6+ in both, `%p` is a `format`-only Tcl 9
/// addition). Collapsing them would make a consumer guess one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatStringArg {
    /// 0-based index into the post-head argument list.
    pub index: usize,
    /// Which mini-language the word is written in.
    pub kind: crate::patterns::FormatType,
    /// The word is a **scan**-direction spec ([`ArgRole::ScanFormat`]) rather
    /// than a format-direction one ([`ArgRole::FormatString`]).
    pub scan: bool,
}
