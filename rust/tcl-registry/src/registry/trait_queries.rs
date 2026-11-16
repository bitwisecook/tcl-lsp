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

//! Membership and metadata queries over one command: traits, symbols, frame sensitivity, linkage and command-binding transitions.

use super::{CommandRegistry, ProcedureWords};
use crate::invocation_words::InvocationWord;
use crate::spec::BytePayloadSpec;
use crate::state_transition::StateTransitions;
use crate::traits::Traits;
use crate::{InvocationArguments, InvocationWords};
use std::collections::HashMap;
use tcl_dialect::model::SurfaceQuery;

impl CommandRegistry {
    /// Return the names of `command`'s subcommands whose traits include
    /// `t` — the subcommand-level counterpart of [`Self::commands_with_trait`].
    /// Empty when the command is unknown or has no matching subcommand.
    #[must_use]
    pub fn subcommands_with_trait(&self, command: &str, t: Traits) -> Vec<&str> {
        self.get(command).map_or_else(Vec::new, |spec| {
            spec.subcommands
                .iter()
                .filter(|s| s.traits.contains(t))
                .map(|s| s.name)
                .collect()
        })
    }

    /// The symbol-definer descriptor for `name` in `dialect`, if the command
    /// binds a navigable definition name (a `tcltest::test` case, …).
    ///
    /// A leading `::` falls back to the bare name, as with [`Self::get`].  The
    /// analyser and signature scanner consult this to record outline symbols
    /// generically — the argument index and outline category come from the
    /// [`crate::symbol_def::SymbolDef`], never from a command-name check.
    #[must_use]
    pub fn defines_symbol(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::symbol_def::SymbolDef> {
        self.get_for_surface(name, dialect)
            .and_then(|s| s.defines_symbol.as_ref())
    }

    /// Every command name that declares a [`crate::symbol_def::SymbolDef`] in
    /// any registered spec, for consumers (the signature scanner) that
    /// precompute a symbol-definer lookup set rather than querying per-call.
    #[must_use]
    pub fn commands_defining_symbols(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.defines_symbol.is_some())
                    .then_some(*name)
            })
            .collect()
    }

    /// Return all command specs whose traits include `t`.
    #[must_use]
    pub fn commands_with_trait(&self, t: Traits) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs.last().filter(|s| s.traits.contains(t)).map(|_| *name)
            })
            .collect()
    }

    /// What role `name` plays in a **cross-language RPC family**, when it plays
    /// one — [`CommandSpec::remote_method`], resolved through [`Self::get`] so
    /// `::ILX::call` answers exactly as the bare spelling does.
    ///
    /// This is also the dialect gate for the whole relation: the ILX commands
    /// are `SpecSurface::IRULES` specs, so a registry built for stock Tcl holds
    /// no command of the name and this answers `None` — an ordinary Tcl
    /// document whose own `proc ILX::call` shadows nothing gets no
    /// remote-method navigation.
    ///
    /// [`CommandSpec::remote_method`]: crate::spec::CommandSpec::remote_method
    #[must_use]
    pub fn remote_method(
        &self,
        name: &str,
    ) -> Option<&'static crate::remote_method::RemoteMethodRole> {
        self.get(name).and_then(|spec| spec.remote_method)
    }

    /// Every command in this registry that takes part in a cross-language RPC
    /// family, by name.
    ///
    /// The membership test a consumer needs *before* it walks a document: if
    /// this is empty (every non-iRules registry), there is no relation to look
    /// for and the walk is skipped entirely; if it is not, a document that
    /// mentions none of the names cannot contain a site — a rename or an
    /// `interp alias` still has to spell the target once to create the binding.
    /// Enumerating them here is what keeps that gate out of the consumers,
    /// which would otherwise have to name `ILX::call` to ask the question.
    #[must_use]
    pub fn remote_method_commands(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .by_name
            .values()
            .filter_map(|specs| specs.last())
            .filter(|spec| spec.remote_method.is_some())
            .map(|spec| spec.name)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `name` **writes or modifies** the variable named by its
    /// first argument (`set` / `append` / `lappend` / `incr` / `lset`):
    /// [`Traits::FIRST_ARG_VARNAME`] minus the destroy-only `unset`
    /// ([`Traits::DESTROYS_VARIABLE`]).
    ///
    /// The single membership query for the write-command consumers —
    /// loop-bound checks, dead-store cancellation, embedded-script def
    /// collection, catch-body out-vars — rather than each keeping its own
    /// hardcoded (and easily inconsistent) name set.
    #[must_use]
    pub fn writes_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && !s.traits.contains(Traits::DESTROYS_VARIABLE)
        })
    }

    /// Whether `name` **read-modify-writes** the variable named by its
    /// first argument (`append` / `lappend` / `incr` / `lset` — not the
    /// whole-value `set`): [`Traits::FIRST_ARG_VARNAME`] ∧
    /// [`Traits::READS_BEFORE_WRITE`].
    ///
    /// Drives the minifier's RMW target protection: a name-compaction
    /// must not rename a variable whose current value an RMW command is
    /// about to fold into, while a plain `set` target is rename-safe.
    #[must_use]
    pub fn rmw_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && s.traits.contains(Traits::READS_BEFORE_WRITE)
        })
    }

    /// Whether `name` is a core Tcl built-in carrying the
    /// [`Traits::BYTE_COMPILED`] trait — i.e. the minifier must not
    /// rewrite this command head to a `$var` alias.  Checks every
    /// registered spec for the name (not just the dialect-preferred
    /// one) so the core stamp is honoured even when a dialect layers
    /// an additional spec under the same name.
    #[must_use]
    pub fn is_byte_compiled(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::BYTE_COMPILED))
        })
    }

    /// Whether `name` is unsafe in sandboxed dialects — it allows
    /// context escalation (`uplevel`, `history`).  Drives the IRULE2003
    /// "unsafe iRules command" check.  Checks every spec registered
    /// under the name.
    #[must_use]
    pub fn is_unsafe(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.unsafe_command))
    }

    /// The [`crate::traits::UNIT_LINKAGE_TRAITS`] this concrete invocation
    /// carries — the registry's answer to "does this command widen the set
    /// of callers beyond the file it appears in?".
    ///
    /// `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
    /// `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) say the
    /// file publishes an API surface; `LOADS_EXTERNAL_UNIT` (`source`,
    /// `load`, `package require`, `auto_load`, `auto_import`, `namespace
    /// import`) says another unit's script runs in this interpreter and can
    /// call back in.  Any of the three sinks the "every caller of this
    /// file's procs is in this file" assumption that
    /// `tcl_compiler::unit_scope`'s interprocedural call-site seed rests on.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured (`package provide` is a boundary, `package names` is not)
    /// and `spec.traits | sub.traits` composes exactly as
    /// [`crate::spec::SubCommand::traits`] documents. An unknown command
    /// carries no linkage — a user proc named `source` is a user proc.
    #[must_use]
    pub fn unit_linkage(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        self.invocation_traits(name, args, dialect)
            .intersection(crate::traits::UNIT_LINKAGE_TRAITS)
    }

    /// Every trait this concrete invocation carries — `spec.traits |
    /// sub.traits`, composed exactly as [`crate::spec::SubCommand::traits`]
    /// documents.
    ///
    /// The invocation-level counterpart of `self.get(name).traits`: a
    /// subcommand's traits are *additive* over its parent's, so a consumer
    /// asking a trait question about a compound command must compose them.
    /// `namespace eval` / `namespace inscope` / `interp eval` carry the
    /// eval-family traits ([`Traits::EVALUATES_CODE`],
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`]) on the **subcommand**, not on
    /// the `namespace` / `interp` spec, so a parent-only trait test silently
    /// misses them.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured; pass `None` to skip dialect gating (the plain
    /// [`Self::get`] lookup) when the question is "what shape is this
    /// command" rather than "is it available here". An unknown command
    /// carries no traits.
    ///
    /// A pack's `stores` row states a write class as well
    /// ([`crate::value_transfer::DeclaredStores::write_class`]): a target
    /// declared `write_or_preserve` or `may_write` is a conditional write
    /// whether or not the pack spells the trait.
    #[must_use]
    pub fn invocation_traits(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        let Some(resolved) = self.resolve_call(name, args, dialect) else {
            return Traits::empty();
        };
        resolved.spec.traits
            | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits)
            | crate::value_transfer::resolve_semantics(resolved.spec, resolved.sub, resolved.form)
                .write_class()
    }

    /// The frame the invocation's scope-alias declaration links the locals
    /// its `VarWrite` operands name into — the global namespace for
    /// `global`, the current namespace for `variable` — where the resolved
    /// command declares one
    /// ([`crate::value_transfer::CommandSemantics::alias_frame`]).
    #[must_use]
    pub fn alias_frame(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::value_transfer::AliasFrame> {
        let resolved = self.resolve_call(name, args, dialect)?;
        crate::value_transfer::resolve_semantics(resolved.spec, resolved.sub, resolved.form)
            .alias_frame()
    }

    /// Whether `name` is **frame-sensitive**: its meaning depends on the
    /// frame or scope it executes in, so moving a call across a proc
    /// boundary (inlining it into a caller) changes behaviour.  The union
    /// of the block-unwinding ([`Traits::TERMINATES_BLOCK`]),
    /// control-transferring ([`Traits::TRANSFERS_CONTROL`]),
    /// scope-aliasing ([`Traits::CREATES_SCOPE_ALIAS`]), and
    /// barrier-creating ([`Traits::CREATES_BARRIER`]) traits.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under the
    /// name.  Drives the inline-proc code action's safety decline and the
    /// negative half of [`Self::is_splice_safe`].
    #[must_use]
    pub fn is_frame_sensitive(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| crate::traits::is_frame_sensitive(s.traits))
        })
    }

    /// Which substitutions `name` performs over its own argument text for a
    /// call with `args`, or `None` when it performs none at all.
    ///
    /// [`Traits::PERFORMS_SUBSTITUTION`] says *that* a command substitutes;
    /// this answers *which kinds* for the call in hand, so a consumer asking
    /// "does this argument read a variable?" never has to match option
    /// spellings itself. The answer is the projection of the call's option
    /// effects onto the substitution axis
    /// ([`CommandSpec::substitutions_performed`]), under this registry's
    /// profile: a command whose options move no substitution axis performs
    /// every kind on every call, and an unreadable call — a computed switch,
    /// a spelling the release lacks, the two families mixed — answers every
    /// kind too, because assuming a substitution does not happen is the
    /// answer that loses a real read.
    #[must_use]
    pub fn substitutions_performed(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::substitution::SubstitutionKinds> {
        self.get(name)?.substitutions_performed(
            InvocationArguments::literals(args),
            self.own_surface_query(),
        )
    }

    /// Every command name that is frame-sensitive ([`Self::is_frame_sensitive`]),
    /// for consumers that scan text for any member of the set (the
    /// inline-proc code action's body check) rather than querying one
    /// resolved head.
    #[must_use]
    pub fn frame_sensitive_commands(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| crate::traits::is_frame_sensitive(s.traits))
                    .then_some(*name)
            })
            .collect()
    }

    /// Whether `name` is **splice-safe**: a call to it can be lifted out of
    /// a wrapper proc and spliced into any caller's frame without changing
    /// observable behaviour.  A splice-safe command is fully lowered
    /// ([`Traits::FRAMELESS_RUNTIME`] — it never falls back to the
    /// interpreter, so it needs no frame) **and** frame-independent: not
    /// frame-sensitive ([`Self::is_frame_sensitive`]) and not operating on
    /// variables by name ([`Traits::FIRST_ARG_VARNAME`] writers,
    /// [`Traits::FRAME_HASH_BUILTIN`] hash-bucket binders, and
    /// [`Traits::DYNAMIC_EVAL_BODY`] body evaluators all observe the frame
    /// they run in).  Drives the inliner's verbatim-splice eligibility;
    /// membership is pinned by `splice_safe_membership` in
    /// `tests/registry_commands.rs`.
    #[must_use]
    pub fn is_splice_safe(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| crate::traits::is_splice_safe(spec.traits))
    }

    /// How *name* crosses stack frames, if it does — the registry's
    /// [`FrameEffectSpec`](crate::frame_effect::FrameEffectSpec).
    ///
    /// The single membership test for "is this a frame-crossing command?".
    /// A consumer reads the returned descriptor to find the level word and
    /// the affected arguments; it never names `upvar` / `uplevel` / `eval`
    /// itself.
    #[must_use]
    pub fn frame_effect(&self, name: &str) -> Option<crate::frame_effect::FrameEffectSpec> {
        self.get(name).and_then(|spec| spec.frame_effect)
    }

    /// The behavioural traits in force for one **concrete call** — the
    /// command's own set, unioned with those of the subcommand `args[0]`
    /// resolves to (exact or unique-prefix), when it has subcommands.
    ///
    /// The trait counterpart of [`Self::arg_indices_for_role`], and it takes
    /// the same `args` shape (subcommand first) for the same reason: a
    /// consumer asking "does *this* call declare a namespace / evaluate code"
    /// must not have to know whether the fact lives on the ensemble or on one
    /// of its subcommands. `namespace eval` carries
    /// [`Traits::DECLARES_NAMESPACE`] on the subcommand while `namespace`
    /// itself does not, and `namespace inscope` — identical argument layout,
    /// same analyser hook — does not carry it at all.
    #[must_use]
    pub fn call_traits(&self, name: &str, args: &[&str]) -> Traits {
        let Some(spec) = self.get(name) else {
            return Traits::empty();
        };
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        sub.map_or(spec.traits, |sub| spec.traits.union(sub.semantic_traits()))
    }

    /// The **command-table transitions** one invocation establishes — the
    /// one vocabulary for "this call changed the command table".
    ///
    /// Every consumer that models command-name bindings reads this: the
    /// flow-sensitive lattice, the lowerer's alias table, the analyser's
    /// rename / alias records, the realm scan, the taint walk. None of them
    /// dispatches on a coarse effect word and re-destructures the argument
    /// layout for itself any more — the layout lives with the registry
    /// resolver, once, and a dynamic operand becomes a typed
    /// [`TransitionSubject::Unknown`] or a domain widening rather than each
    /// consumer's own `is_dynamic_word` test.
    ///
    /// Selection is the ordinary
    /// [`Self::resolve_structured_invocation`] at **this registry's own
    /// point**: command-table mutations are executable behaviour, so
    /// they must use the profile-visible command surface. In particular,
    /// iRules intentionally has no `rename` or `interp`, even though the
    /// dialect-agnostic catalogue knows their Tcl specs. A profile-less
    /// registry (`build_default`) answers dialect-agnostically, exactly as
    /// [`Self::get`] does.
    ///
    /// Reusing that primitive means a **subcommand-shaped** mutator now
    /// resolves by the ordinary ensemble rule rather than by exact
    /// spelling, which is a deliberate behavioural change from the retired
    /// `command_table_effect`: `interp ali {} a {} b` really does create an
    /// alias in C Tcl (`interp` is an ensemble), and the retired resolver's
    /// exact-word lookup made it invisible to every binding consumer.
    /// `interp aliases` is its own subcommand and still states nothing.
    ///
    /// The explicitly global spellings answer
    /// identically to the bare ones — `::rename format ::origfmt`,
    /// `::interp alias {} myfmt {} ::origfmt` and `::proc ::greet {} {…}`
    /// all really do mutate the command table (tclsh 9.0.4 and 8.6.16,
    /// byte-identical; `namespace which -command ::rename` → `::rename`).
    ///
    /// [`TransitionSubject::Unknown`]: crate::TransitionSubject::Unknown
    #[must_use]
    pub fn command_binding_transitions(&self, words: InvocationWords<'_>) -> StateTransitions {
        let query = self.own_surface_query();
        self.resolve_structured_invocation(words, query)
            .resolved()
            .map_or_else(StateTransitions::default, |invocation| {
                invocation.state_transitions()
            })
    }

    /// Whether `name` (or the compound key `"cmd sub"`) produces a
    /// canonical Tcl list — a list whose elements are properly
    /// quoted so re-parsing by ``eval`` / ``uplevel`` /
    /// ``interp eval`` doesn't trigger unwanted substitution.
    ///
    /// The canonical-list-command set is
    /// derived from `return_type == TclType::List` on the command
    /// (or its subcommand entry), minus the explicit exclusion
    /// `concat` whose join-strip-grouping semantics can leave
    /// unquoted specials in the output.
    ///
    /// Drives the W101 (`eval` with string concatenation) safe-
    /// idiom suppression — `eval [list ...]`, `eval [linsert ...]`,
    /// `eval [split ...]`, etc. shouldn't fire because the inner
    /// command's output is a properly-quoted list.
    #[must_use]
    pub fn is_canonical_list_command(&self, name: &str) -> bool {
        // Exclusion: concat returns LIST but isn't canonical.
        if name == "concat" {
            return false;
        }
        // Compound form ``"cmd sub"`` — split into head + sub.
        if let Some((head, sub_name)) = name.split_once(' ') {
            if let Some(spec) = self.get(head)
                && let Some(sub) = spec.subcommand(sub_name)
            {
                return sub.return_type == Some(crate::types::TclType::List);
            }
            return false;
        }
        // Bare command name.
        self.get(name)
            .and_then(|spec| spec.return_type)
            .is_some_and(|t| t == crate::types::TclType::List)
    }

    /// Whether `name` runs one of its body arguments more than once —
    /// [`Traits::HAS_LOOP_BODY`].
    ///
    /// The single answer for a consumer that needs "is this a loop?" without
    /// naming `while` / `for` / `foreach`, so a pack-declared loop command
    /// joins loop analysis with no consumer edit. Nothing else in the model
    /// says it: `NEVER_INLINE_BODY` marks a different, non-coextensive set —
    /// `timerate` and `array for` are loops without it, and consumers read the
    /// command's bits unioned with the resolved subcommand's, so `dict for`
    /// carries it from `dict` itself.
    #[must_use]
    pub fn is_loop_command(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| spec.traits.contains(Traits::HAS_LOOP_BODY))
    }

    /// Positions selected by the definer's Native argv-shape grammar.
    /// An actual argument dialect must identify that grammar; bare catalogue
    /// roles are available separately through authored source queries.
    ///
    /// The indices count words after the head, including Jim's optional
    /// static-variable list. This shape does not validate formal contents,
    /// install a procedure, or admit its body for execution.
    #[must_use]
    pub fn procedure_definition_words(&self, head: &str, args: &[&str]) -> Option<ProcedureWords> {
        let arguments = InvocationArguments::literals(args).with_profile(self.profile());
        let selected = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(head), arguments),
                self.own_surface_query(),
            )
            .resolved()?;
        if !selected
            .semantics
            .traits
            .contains(Traits::DEFINES_PROCEDURE)
        {
            return None;
        }
        let crate::native_procedure::NativeProcedureDefinitionSelection::Valid(definition) =
            selected.semantics.procedure_definition?.select(arguments)
        else {
            return None;
        };
        Some(ProcedureWords {
            name: definition.name_at,
            params: definition.parameters_at,
            statics: definition.statics_at,
            body: definition.body_at,
        })
    }

    /// `{command: BytePayloadSpec}` for every registered `<proto>::payload`
    /// byte-array command — the getter is a binary source and `<cmd> replace`
    /// a byte sink for the S110 byte-array-corruption check.
    ///
    /// Only commands actually loaded into this registry are returned, so the
    /// set is implicitly gated by the active dialect: a plain-Tcl registry
    /// (no iRules pack loaded) yields an empty map — the payload layouts
    /// are intersected with the active dialect.
    #[must_use]
    pub fn byte_array_payload_layouts(&self) -> HashMap<&'static str, BytePayloadSpec> {
        let mut out = HashMap::new();
        for specs in self.by_name.values() {
            for spec in specs {
                if let Some(layout) = spec.byte_array_payload {
                    out.insert(spec.name, layout);
                }
            }
        }
        out
    }
}
