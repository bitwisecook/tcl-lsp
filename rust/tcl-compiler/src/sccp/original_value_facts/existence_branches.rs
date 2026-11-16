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

//! Folding `info exists` and `array exists` conditions into constant branches, from the names a body may define or unset.

use super::super::ConstantBranch;
use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;
use crate::ssa::SsaFunction;
use rustc_hash::FxHashSet;
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

fn retain_source_call_name_coverage(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
    defined: &mut FxHashSet<String>,
    unset: &mut FxHashSet<String>,
) {
    let Some(names) = crate::registry_invocation::original_source_name_roles(registry, tokens)
    else {
        return;
    };
    let destination = if names
        .facts()
        .traits
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
    {
        unset
    } else {
        defined
    };
    for &(argument, role) in names.roles() {
        if role == tcl_registry::ArgRole::VarWrite
            && let Some(name) = names.original_variable_name_operand(argument)
        {
            destination.insert(name.as_str().to_owned());
            if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name.as_str()) {
                destination.insert(root.to_owned());
            }
        }
    }
}

/// Possible writes and destroys protect whole-body existence folds.
/// Original source-only roles retain unknown completion and frame effects.
pub(super) fn scan_defined_and_unset(
    cfg: &CfgFunction,
    metadata: ExistenceMetadataInputs<'_>,
) -> (
    FxHashSet<String>,
    FxHashSet<String>,
    crate::dynamic_names::DynamicNameBarrier,
) {
    use crate::dynamic_names::DynamicNameBarrier;
    let mut defined = FxHashSet::default();
    let mut unset = FxHashSet::default();
    let mut unknown = DynamicNameBarrier::default();
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(metadata.registry))
    else {
        return (defined, unset, DynamicNameBarrier::OPAQUE_SCRIPT);
    };
    if cfg.metadata_context.source_analysis_input() != context.source_analysis_input()
        && (!cfg.metadata_context.is_standalone() || context.source_analysis_input().is_some())
    {
        return (defined, unset, DynamicNameBarrier::OPAQUE_SCRIPT);
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { .. }
                | Statement::AssignExpr { .. }
                | Statement::AssignValue { .. }
                | Statement::Incr { .. } => {
                    let projection = crate::ir_helpers::structural_variable_write_projection(stmt);
                    unknown.writes |= projection.opaque_variable_frame;
                    for name in projection.literal_names {
                        unknown.writes |= name.contains('\0');
                        defined.insert(
                            tcl_syntax::naming::split_element_ref(&name)
                                .map_or(name.as_str(), |(root, _)| root)
                                .to_owned(),
                        );
                    }
                }
                Statement::Call {
                    command,
                    args,
                    defs,
                    ..
                } => {
                    retain_call_name_writes(
                        (command, args, defs),
                        stmt,
                        cfg,
                        metadata,
                        context,
                        (&mut defined, &mut unset, &mut unknown),
                    );
                }
                _ => {}
            }
        }
    }
    (defined, unset, unknown)
}

fn retain_nominal_call_name_writes(
    command: &str,
    args: &[String],
    metadata: ExistenceMetadataInputs<'_>,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    (defined, unset, unknown): (
        &mut FxHashSet<String>,
        &mut FxHashSet<String>,
        &mut crate::dynamic_names::DynamicNameBarrier,
    ),
) {
    use crate::dynamic_names::DynamicNameBarrier;
    use tcl_registry::{ArgRole, Traits};
    // Explicit nominal argv has no original invocation or Native permission.
    let arguments: Vec<_> = args.iter().map(String::as_str).collect();
    let Some(schema) = metadata.registry.resolve_invocation(
        command,
        &arguments,
        Some(context.context().authoring_query()),
    ) else {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let facts = schema.facts();
    if !facts.arg_roles_complete {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
    let destroys = facts.traits.contains(Traits::DESTROYS_VARIABLE);
    let argument_offset = facts.argument_offset;
    for (ordinal, role) in facts.arg_roles {
        if role != ArgRole::VarWrite {
            continue;
        }
        let Some(name) = args.get(argument_offset + usize::from(ordinal)) else {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            continue;
        };
        if crate::dynamic_names::names_a_dynamic_variable(name) || name.contains('\0') {
            if destroys {
                unknown.destroys = true;
            } else {
                unknown.writes = true;
            }
        } else {
            let destination = if destroys { &mut *unset } else { &mut *defined };
            destination.insert(name.clone());
            if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name) {
                destination.insert(root.to_owned());
            }
        }
    }
}

fn retain_call_name_writes(
    (command, args, defs): (&str, &[String], &[String]),
    stmt: &Statement,
    cfg: &CfgFunction,
    metadata: ExistenceMetadataInputs<'_>,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    (defined, unset, unknown): (
        &mut FxHashSet<String>,
        &mut FxHashSet<String>,
        &mut crate::dynamic_names::DynamicNameBarrier,
    ),
) {
    use crate::dynamic_names::DynamicNameBarrier;
    use crate::registry_invocation::EffectiveInvocationWord;
    use tcl_registry::ArgRole;
    defined.extend(defs.iter().cloned());
    if let Some(tokens) = stmt.tokens() {
        let Some(names) =
            crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                metadata.registry,
                Some(context),
                tokens,
            )
        else {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            retain_source_call_name_coverage(metadata.registry, tokens, defined, unset);
            return;
        };
        if names.unknown_residual() {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            retain_source_call_name_coverage(metadata.registry, tokens, &mut *defined, &mut *unset);
        }
        for (role, word, _, destroys) in names.phased_operands() {
            if role != ArgRole::VarWrite {
                continue;
            }
            let destination = if destroys { &mut *unset } else { &mut *defined };
            match word {
                EffectiveInvocationWord::Literal(name) => {
                    destination.insert(name.clone());
                    if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name) {
                        destination.insert(root.to_owned());
                    }
                    if name.contains('\0') {
                        if destroys {
                            unknown.destroys = true;
                        } else {
                            unknown.writes = true;
                        }
                    }
                }
                EffectiveInvocationWord::ArrayElementName { root } => {
                    destination.insert(root.clone());
                }
                _ => {
                    if destroys {
                        unknown.destroys = true;
                    } else {
                        unknown.writes = true;
                    }
                }
            }
        }
        for (role, destroys) in names.phased_unresolved_roles() {
            if role == ArgRole::VarWrite {
                if destroys {
                    unknown.destroys = true;
                } else {
                    unknown.writes = true;
                }
            }
        }
    } else if cfg.metadata_context.is_standalone() && context.source_analysis_input().is_none() {
        retain_nominal_call_name_writes(
            command,
            args,
            metadata,
            context,
            (defined, unset, unknown),
        );
    } else {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
}

/// The entry facts one function frame contributes to the existence fold
/// ([`existence_constant_branches`]), sourced from the typed IR for whichever
/// kind of body it is — a [`crate::ir::Procedure`], a
/// [`crate::ir::MethodDef`], or neither (the top level, a lambda).
///
/// Bundled rather than passed positionally so a new fact reaches both
/// consumers of the fold — the analyser's I230 and the optimiser's O101 — by
/// construction: the two build the same struct from the same IR, so they
/// cannot drift on, say, method parameters.
#[derive(Clone, Copy, Default)]
pub struct ExistenceFrame<'a> {
    /// The body's formal parameter names: bound on entry as scalars, so
    /// they exist for `info exists` and never for `array exists`.
    /// Empty for the top level and for any body with no parameter list.
    pub params: &'a [String],
    /// Names auto-bound to out-of-frame *object* storage on entry — a
    /// `TclOO` method body's [`crate::ir::MethodDef::instance_vars`].
    /// `None` for every body kind that has none.
    pub object_state: Option<&'a HashSet<String>>,
    /// Whether this body is the document's **initial global frame** (the
    /// compilation unit's top level).  Only there does the frame share the
    /// interpreter's own globals, so only there must the fold abstain on the
    /// registry's special variables — a procedure-local `argv` is an ordinary
    /// fresh Tcl name and keeps folding.
    pub initial_global: bool,
}

/// Command store and exact availability for variable-presence source queries.
#[derive(Clone, Copy)]
pub struct ExistenceMetadataInputs<'a> {
    /// Actual graph command store.
    pub registry: &'a CommandRegistry,
    /// Supplied availability; absence and foreign stores refuse folding.
    pub context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
}

/// Existence folds from exact invocation and contents-presence proofs.
/// Both optimiser rewrites and diagnostics consume this positioned entry point.
/// Missing nested dispatch or uncertain presence declines without a name scan.
#[must_use]
pub fn existence_constant_branches_with_ssa(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
) -> Vec<ConstantBranch> {
    let metadata = ExistenceMetadataInputs {
        registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    existence_constant_branches_with_ssa_with_metadata_context(cfg, frame, metadata, config, ssa)
}

fn known_existence_query(
    var: &str,
    kind: crate::existence_query::ExistenceKind,
    frame: ExistenceFrame<'_>,
    (defined, unset): (&FxHashSet<String>, &FxHashSet<String>),
    aliased: &HashSet<String>,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
) -> Option<bool> {
    let unknown_writes = dynamic_names.writes;
    Some(if let Some(base) = array_element_base(var) {
        // An array-element guard on a never-touched array is provably
        // false: no element of `a` can exist when nothing
        // in this barrier-free body ever created `a` — tclsh 9.0.4 /
        // 8.6.16: `proc f {} { info exists Params(key) }` → 0.  The
        // decision is about the *array* name alone, so a dynamic element
        // key (`Params($k)`) folds just as well, and the base takes the
        // same abstentions as a simple name: scope-alias / instance-state
        // (`aliased`), a dynamic write that may have created any name,
        // and any touch of the base — a `set a(x) …` element write, an
        // `array set` / `upvar`-style whole-array def, either spelling.
        // A parameter base abstains outright: the parameter itself is a
        // scalar, and the fold stays strictly one-sided here rather than
        // reason about unset-and-remake shapes.
        if aliased.contains(base)
            || frame.params.iter().any(|p| p == base)
            || unknown_writes
            || defined
                .iter()
                .any(|d| d == base || d.strip_prefix(base).is_some_and(|r| r.starts_with('(')))
        {
            return None;
        }
        false
    } else {
        // Namespaced globals may be populated outside the function's
        // view — only fold simple local names.
        if var.is_empty() || !var.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return None;
        }
        // A scope-alias local's existence tracks the linked variable —
        // never fold it (see the `aliased` collection above).
        if aliased.contains(var) {
            return None;
        }
        if frame.params.iter().any(|p| p.as_str() == var) {
            // A literal `unset x` already blocks this; a computed
            // `unset $n` can name the parameter just as well
            // (tclsh 9.0.4 / 8.6.14: `proc f {p n} {unset $n; info exists p}`
            // → `0` for `f hello p`), so the barrier blocks it too.
            if unset.contains(var) || dynamic_names.destroys {
                return None;
            }
            // Which constant depends on the spelling.  A
            // parameter is bound as a *scalar* on entry — Tcl has no
            // pass-an-array-by-value — so `array exists PARAM` is
            // provably **false** where `info exists PARAM` is true.
            // Nothing in a barrier-free body can turn the parameter into
            // an array without first removing the scalar binding, and a
            // literal `unset` / a dynamic destroy already abstained above
            // (`set p(k) …` and `array set p …` on a live scalar are
            // runtime errors, not conversions).
            //
            // tclsh-proof (8.6.16 / 9.0.4):
            //   proc f {a} { if {[array exists a]} { puts yes } else { puts no } }
            //   f 1                                        ;# → no
            //   proc g {a} { array set a {x 1} }
            //   g 1  ;# → can't set "a(x)": variable isn't array
            matches!(kind, crate::existence_query::ExistenceKind::AnyVariable)
        } else if !defined.contains(var) {
            // `set $switch {}` may have defined exactly this name — the
            // argparse idiom.
            if unknown_writes {
                return None;
            }
            false
        } else {
            return None;
        }
    })
}

/// Existence query under the supplied actual availability generation.
#[must_use]
pub fn existence_constant_branches_with_ssa_with_metadata_context(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    metadata: ExistenceMetadataInputs<'_>,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
) -> Vec<ConstantBranch> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return Vec::new();
    };
    let Some(points) = &ssa.point_contexts else {
        return existence_constant_branches_with_metadata_context(
            cfg,
            frame,
            metadata,
            crate::dynamic_names::dynamic_name_barrier(cfg, registry, config),
            config,
        );
    };
    cfg.blocks
        .iter()
        .filter_map(|(&id, block)| {
            let Terminator::Branch {
                condition,
                true_target,
                false_target,
                span: Some(span),
                condition_base: Some(base),
            } = block.terminator.as_ref()?
            else {
                return None;
            };
            let tokens = points.source_tokens_at(id, usize::MAX)?;
            let (query, context) = crate::existence_query::in_expr_at_with_metadata_context(
                condition,
                *base,
                tokens,
                registry,
                config,
                Some(context),
            )?;
            let place =
                crate::var_resolve::resolve_literal_place(&query.var, &context, false, registry);
            let exists = match context.contents_presence(&place) {
                crate::var_resolve::ContentsPresence::Undefined => false,
                crate::var_resolve::ContentsPresence::Defined => match query.kind {
                    crate::existence_query::ExistenceKind::AnyVariable => true,
                    crate::existence_query::ExistenceKind::Array => {
                        // Literal scalar contents and untouched own formal cells
                        // prove scalar storage. Other defined cells may be arrays.
                        let formal = frame.params.contains(&query.var)
                            && context.contents_origin(&place)
                                == crate::var_resolve::ContentsOrigin::Incoming
                            && place.cell.as_ref().is_some_and(|cell| {
                                matches!(
                                    &cell.owner, crate::place::CellOwner::Activation(owner)
                                        if context.activation.as_ref() == Some(owner)
                                )
                            });
                        if !formal && context.literal_contents_at(&place, registry).is_none() {
                            return None;
                        }
                        false
                    }
                },
                crate::var_resolve::ContentsPresence::Unknown
                | crate::var_resolve::ContentsPresence::DefinedOrUndefined => return None,
            };
            let value = exists ^ query.negated;
            let (taken, not_taken) = if value {
                (*true_target, *false_target)
            } else {
                (*false_target, *true_target)
            };
            Some(ConstantBranch {
                kind: super::super::BranchFactKind::Applied,
                block: block.name.clone(),
                span: Some(*span),
                condition: crate::expr_ast::expr_text(condition),
                value,
                taken_target: cfg.block_name(taken).to_owned(),
                not_taken_target: cfg.block_name(not_taken).to_owned(),
            })
        })
        .collect()
}

/// The array base name of an existence query written as an element guard —
/// `Some("Params")` for `Params(key)` (any element spelling, including a
/// dynamic `Params($k)`), `None` for every other shape.  Only a simple local
/// base qualifies: a namespaced array (`::env(PATH)`) may be populated
/// outside the function's view.
fn array_element_base(var: &str) -> Option<&str> {
    let (base, _) = tcl_syntax::naming::split_element_ref(var)?;
    if base.is_empty() {
        return None;
    }
    base.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        .then_some(base)
}

/// Fold `[info exists X]` / `[array exists X]`
/// if-conditions into [`ConstantBranch`] entries for the
/// false-positive-free cases — a parameter always exists, as a **scalar**
/// (`info exists` → `true`, `array exists` → `false`); a never-defined
/// non-parameter never exists (`false`); an element guard `X(elem)` on an
/// array this body never touches never exists (`false` — the guard is decided
/// on the *array* name, with the same abstentions as a simple name, so the
/// element key may even be dynamic).
/// `![info exists X]` flips the value.
///
/// SCCP itself can't fold these (the predicate is an opaque
/// `ExprNode::Command`, and SCCP has neither parameter nor existence
/// facts), so this runs as a post-pass with the frame's own facts.  The
/// result feeds both the analyser's I230 (constant condition) and the
/// optimiser's O101 (constant-branch fold / DCE).  Only simple local
/// names are folded, and only in functions free of opaque barriers (an
/// unknown command could `unset` or `upvar`-define the variable).
/// Scope-alias locals (`global` / `variable` / `upvar` / `namespace
/// upvar` bindings) are never folded — their existence tracks the
/// linked out-of-frame variable.  In the **initial global frame**
/// ([`ExistenceFrame::initial_global`]) the registry's special variables join
/// them for the same reason: that frame is the interpreter's own global
/// namespace, whose startup bindings and runtime-materialised entries the
/// body's assignment scan cannot see.
///
/// `dynamic_names` carries the function's
/// [dynamic-name barrier](crate::dynamic_names) and gates each direction
/// independently:
///
/// - a **dynamic write** (`set $switch {}`) can define *any* name, so the
///   "never defined here, therefore absent" fold is no longer provable;
/// - a **dynamic destroy** (`unset $n`) can remove *any* name, so even the
///   "it's a parameter, therefore present" fold is no longer provable.
///
/// Both abstain by declining the fold, which silences I230 and leaves O101
/// with nothing to fold — say less rather than say something wrong.
///
/// [`ExistenceFrame::object_state`] carries the frame's *auto-bound*
/// out-of-frame names — a `TclOO` method body's
/// [`crate::ir::MethodDef::instance_vars`].
/// A class-level `variable x` declaration binds `x` in **every**
/// method's frame with no `variable` statement in the body itself, so
/// [`crate::optimiser::elimination::scan_scope_aliases`] (which only sees the
/// body's own commands) cannot find it — the name looks like a never-defined
/// local, which the fold would otherwise call "always absent".  It is not:
/// existence is per-instance runtime state, set by whichever method or
/// constructor assigned it first.  tclsh 9.0.4 and 8.6.14 agree:
///
/// ```tcl
/// oo::class create C { variable x; constructor {} { set x 1 }
///                      method m {} { info exists x } }   ;# [C new] m → 1
/// oo::class create D { variable x
///                      method m {} { info exists x } }   ;# [D new] m → 0
/// oo::class create F { variable x; method setit {} { set x 42 }
///                      method m {} { info exists x } }
/// set f [F new]; $f m   ;# → 0
/// $f setit; $f m        ;# → 1
/// ```
///
/// The declaration alone does not create the variable, but *any earlier call
/// on the same instance* may have — a dynamic fact no per-method analysis can
/// decide, so these names join `aliased` and never fold either way.
///
/// A method parameter that *collides* with an instance-variable name is the
/// exception, and still folds `true`: the parameter shadows the class-level
/// declaration completely, and writes through it never reach object state
/// (again identical on 9.0.4 and 8.6.14).
///
/// ```tcl
/// oo::class create A { variable x; constructor {} { set x 42 }
///                      method m {x} { set r [info exists x]; set x 9; return $r }
///                      method peek {} { return $x } }
/// set a [A new]; $a m hello   ;# → 1
/// $a peek                     ;# → 42, untouched by the method's `set x 9`
/// ```
///
/// This also holds for a *defaulted* parameter called with no argument
/// (`method m {{x def}} …` → `info exists x` is 1 and `$x` is `def`), and
/// when the instance variable was never assigned at all.
#[must_use]
pub fn existence_constant_branches(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    registry: &tcl_registry::CommandRegistry,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    config: tcl_lexer::LexerConfig,
) -> Vec<ConstantBranch> {
    let metadata = ExistenceMetadataInputs {
        registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    existence_constant_branches_with_metadata_context(cfg, frame, metadata, dynamic_names, config)
}

/// Existence query under the supplied actual availability generation.
#[must_use]
pub fn existence_constant_branches_with_metadata_context(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    metadata: ExistenceMetadataInputs<'_>,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    config: tcl_lexer::LexerConfig,
) -> Vec<ConstantBranch> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if cfg.blocks.values().any(|b| {
        b.statements.iter().any(|s| {
            matches!(
                s,
                Statement::Barrier { .. }
                    | Statement::NativeCall { .. }
                    | Statement::UpFrame { .. }
            )
        })
    }) {
        return out;
    }
    let (defined, unset, call_names) = scan_defined_and_unset(cfg, metadata);
    let dynamic_names = dynamic_names.union(call_names);
    // Locals bound to out-of-frame storage (`global` / `variable` / `upvar` /
    // `namespace upvar`): whether such a name exists depends on the *linked*
    // variable, which this function cannot see, so its existence query must
    // never fold either way.  `global` / `variable` / `upvar` escape via
    // `defined` already (their `Call::defs` carry the alias local), but
    // `namespace upvar` lowers with empty defs — tclsh 8.6:
    // `namespace eval ns {variable s ok}; proc t {} {namespace upvar ns s a;
    // info exists a}; t` → 1 (and → 0 when `ns::s` is unset), the exact
    // `::safe::CheckInterp` guard shape (safe.tcl:109).  The scanner also
    // returns `trace` targets, which only widens the skip — conservative,
    // never a false fold.
    let mut aliased = crate::optimiser::elimination::scan_scope_aliases_with_metadata_context(
        cfg,
        registry,
        Some(context),
    );
    // Object state is aliased the same way, minus a visible binding command:
    // `TclOO` links every class-level `variable` declaration into each method
    // frame at entry, so the body's own command scan cannot see it.
    //
    // A formal parameter of the same name is the one exception: it shadows the
    // class-level declaration outright, so the name is an ordinary local that
    // always exists and must keep folding `true`.  tclsh 9.0.4 / 8.6.14 agree
    // — the parameter wins completely, and writes to it do **not** reach
    // object state:
    //
    //   oo::class create A { variable x; constructor {} { set x 42 }
    //                        method m {x} { set r [info exists x]  ;# → 1
    //                                       set x 9; return $r }
    //                        method peek {} { return $x } }
    //   set a [A new]; $a m hello   ;# → 1  ($x inside m is "hello")
    //   $a peek                     ;# → 42, unchanged by `set x 9`
    //
    // Only the *object-state* half yields to parameters.  The
    // command-derived aliases keep full precedence: an explicit `global` /
    // `variable` / `upvar` / `namespace upvar` / `my variable` on a name that
    // is already a parameter is a runtime error on both runtimes (`variable
    // "x" already exists`), so it never legally co-occurs, while a variable
    // *trace* on a parameter does — and a trace callback can unset its own
    // target, which is exactly why `scan_scope_aliases` includes trace
    // targets and why they must go on abstaining.
    if let Some(instance_vars) = frame.object_state {
        aliased.extend(
            instance_vars
                .iter()
                .filter(|name| !frame.params.iter().any(|p| p == *name))
                .cloned(),
        );
    }
    // The document's initial global frame *is* the interpreter's global
    // namespace, so every name the special-variable registry recognises there
    // is out-of-frame runtime state exactly like object state above.
    // Some are bound before user code (`argv`, `env`, `tcl_platform`,
    // `auto_path`), some are materialised by a later runtime event this body
    // cannot see (`errorInfo` after a `catch`, `auto_index` after an
    // auto-load), and some by a read trace (`tcl_precision` on Tcl 8.x) — none
    // is provably absent merely because the body never assigned it, and
    // tclsh 8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 / 9.1b0 all answer
    // `info exists argv` → 1 at the top level.  Folding them "always absent"
    // produced a false I230 and, worse, an O101 rewrite of
    // `if {[info exists argv]} …` to `if {0} …`.
    //
    // The set is dialect-versioned registry data, so a release that drops a
    // variable (`tcl_precision` in Tcl 9) or a dialect that never had one
    // (iRules has no `argv`) keeps folding it.  Inside a procedure the name is
    // an ordinary local and still folds; an explicit `global argv` there is
    // already covered by the scope-alias skip above.
    if frame.initial_global {
        aliased.extend(
            tcl_registry::special_vars::special_vars_for_dialect(Some(
                context.context().authoring_query(),
            ))
            .map(|spec| spec.name.to_owned()),
        );
    }
    for block in cfg.blocks.values() {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span: Some(span),
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(crate::existence_query::ExistenceQuery { var, negated, kind }) =
            crate::existence_query::in_expr_with_metadata_context(
                condition,
                registry,
                config,
                Some(context),
            )
        else {
            continue;
        };
        let Some(exists) = known_existence_query(
            &var,
            kind,
            frame,
            (&defined, &unset),
            &aliased,
            dynamic_names,
        ) else {
            continue;
        };
        let value = exists ^ negated;
        let (true_name, false_name) = (
            cfg.block_name(*true_target).to_owned(),
            cfg.block_name(*false_target).to_owned(),
        );
        let (taken, not_taken) = if value {
            (true_name, false_name)
        } else {
            (false_name, true_name)
        };
        out.push(ConstantBranch {
            kind: super::super::BranchFactKind::Applied,
            block: block.name.clone(),
            span: Some(*span),
            condition: crate::expr_ast::expr_text(condition),
            value,
            taken_target: taken,
            not_taken_target: not_taken,
        });
    }
    out
}
