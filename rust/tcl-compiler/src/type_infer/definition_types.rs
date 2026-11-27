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

//! How one statement types the variables it defines: stores, calls with write typing, and `foreach` element variables.

use super::container_elements::var_elements_effect_lattice;
use super::literals::literal_type;
use super::word_types::{lowered_expression_result_type, value_word_type};
use super::{WordTypingCtx, return_type_for_command};
use crate::ir::Statement;
use crate::types::{Elements, TypeKind, TypeLattice, TypeShape, shape_join};
use crate::var_resolve::VariableCellTable;
use tcl_registry::{TclType, VarWriteTyping};

/// How one statement types the variable(s) it defines.
pub(super) enum DefTyping {
    /// One lattice applied to every def of the statement.
    Uniform(TypeLattice),
    /// Positional element typing (`lassign`, `foreach` element vars): each
    /// def takes its own lattice; a def absent from the map widens to
    /// `Overdefined`.
    PerDef(VariableCellTable<TypeLattice>),
}

/// Infer the type produced by `stmt` under the current `types` map.
#[must_use]
pub(super) fn evaluate_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> DefTyping {
    match stmt {
        Statement::AssignConst { value, .. } => {
            DefTyping::Uniform(literal_type(value, ctx.numbers))
        }

        // The fused `set` keeps its original words beside the statement, not on it.
        Statement::AssignExpr {
            expr, expr_base, ..
        } => DefTyping::Uniform(lowered_expression_result_type(
            ctx,
            ctx.tokens.or_else(|| ctx.source.source_tokens()),
            expr,
            *expr_base,
        )),

        Statement::AssignValue { value, tokens, .. } => DefTyping::Uniform(value_word_type(
            ctx,
            value,
            tokens.as_ref().and_then(|tokens| tokens.words().get(2)),
        )),

        Statement::Incr { .. } => DefTyping::Uniform(TypeLattice::of(TclType::Int)),

        // A physical array-element write can have no scalar SSA definition.
        // Its retained memory write still contributes a value at later reads.
        Statement::Call { .. } => evaluate_call_type_def(stmt, ctx),

        // `ExprEval`, `Barrier`, and structured statements that survive as
        // statements (before CFG construction in some paths) all lack a
        // resolvable result type here — treat them conservatively as
        // overdefined.
        _ => DefTyping::Uniform(TypeLattice::overdefined()),
    }
}

fn normal_store_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> Option<DefTyping> {
    if let Some(tokens) = stmt.tokens()
        && let Some(binding) = tokens.source_binding.as_ref()
        && let Some(normal) =
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                ctx.registry,
                ctx.context,
                tokens,
            )
    {
        if let Some((value, word)) =
            normal.stored_value_operand(&binding.variable_context, ctx.registry)
        {
            return Some(DefTyping::Uniform(value_word_type(ctx, value, Some(word))));
        }
        if let Some(write) = normal.container_element_write(&binding.variable_context, ctx.registry)
        {
            let args = write
                .arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            return Some(DefTyping::Uniform(var_elements_effect_lattice(
                ctx,
                write.effect,
                &write.target,
                &args,
                write.argument_offset,
                Some(write.words),
            )));
        }
    }
    None
}

fn evaluate_call_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> DefTyping {
    let Statement::Call {
        command,
        canonical_command,
        args,
        defs,
        ..
    } = stmt
    else {
        return DefTyping::Uniform(TypeLattice::overdefined());
    };
    if stmt.tokens().is_some_and(|tokens| {
        matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        )
    }) {
        let Statement::Call { foreach_groups, .. } = stmt else {
            unreachable!()
        };
        let arguments = args.iter().map(String::as_str).collect::<Vec<_>>();
        return elements_of_def_typing(ctx, &arguments, defs, foreach_groups.as_deref(), 0);
    }
    if let Some(typing) = normal_store_type_def(stmt, ctx) {
        return typing;
    }
    // Resolve the source spelling through the lowerer's
    // `canonical_command` snapshot (an `interp alias` / `rename`
    // target) so a renamed or aliased builtin — `rename set myset` /
    // `interp alias {} myset {} set` — is typed by the *real* command's
    // registry spec, not left as an unknown `Call` (OVERDEFINED).
    let proven = stmt.tokens().map(|_| {
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            stmt,
        )
    });
    if matches!(proven, Some(None)) {
        return DefTyping::Uniform(TypeLattice::overdefined());
    }
    let proven = proven.flatten();
    let canon = proven.as_ref().map_or_else(
        || canonical_command.as_deref().unwrap_or(command),
        |invocation| invocation.facts.canonical_command.as_str(),
    );
    let presented;
    let args = if let Some(invocation) = &proven {
        let Some(arguments) = invocation
            .arguments
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
        else {
            return DefTyping::Uniform(TypeLattice::overdefined());
        };
        presented = arguments;
        presented.as_slice()
    } else {
        args.as_slice()
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    // A canonically-`set` store keeps its runtime `Call` shape for
    // codegen — an aliased / renamed command is not inline-foldable,
    // its binding may change by call time — but for the type lattice
    // its stored cell takes the *value word's* intrep verbatim, exactly
    // as the un-aliased [`Statement::AssignValue`] path does. Keyed off
    // the canonical command's `Set` lowering hook (the registry's own
    // "this is a value-passthrough store" fact), never the source
    // spelling. Effective argv fixes the setter value position, including
    // composed aliases. A dynamic array index has a physical store without
    // a scalar definition, so scalar def cardinality cannot gate this fact.
    if arg_refs.len() == 2
        && proven.as_ref().map_or_else(
            || {
                ctx.registry.get(canon).and_then(|spec| spec.lowering_hook)
                    == Some(tcl_registry::hooks::LoweringHookId::Set)
            },
            |invocation| {
                invocation.facts.operation
                    == tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Set,
                    )
            },
        )
    {
        return DefTyping::Uniform(value_word_type(
            ctx,
            arg_refs[1],
            proven
                .as_ref()
                .and_then(|invocation| invocation.effective.words.get(2)),
        ));
    }

    written_call_type(stmt, ctx, canon, &arg_refs, proven.as_ref())
}

fn written_call_type<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
    canon: &str,
    arg_refs: &[&str],
    proven: Option<&crate::registry_invocation::ResolvedStatementInvocation>,
) -> DefTyping {
    let Statement::Call {
        defs,
        foreach_groups,
        ..
    } = stmt
    else {
        return DefTyping::Uniform(TypeLattice::overdefined());
    };
    let resolved = tcl_registry::model::resolve_invocation_in_context(
        ctx.registry,
        ctx.context
            .map(crate::registry_invocation::InvocationMetadataContext::context),
        canon,
        arg_refs,
    );

    // An in-place element write (`lappend VAR v…`, `dict set VAR … v`)
    // evolves the target's container elements — the registry's
    // `VarElementsEffect` fact, generalising the old object-only
    // element-class harvesting to every element shape.
    if let Some(effect) = proven.map_or_else(
        || {
            resolved
                .as_ref()
                .and_then(|resolved| resolved.semantics.var_elements_effect)
        },
        |invocation| invocation.facts.var_elements_effect,
    ) {
        let base = proven.map_or_else(
            || {
                usize::from(
                    resolved
                        .as_ref()
                        .is_some_and(|resolved| resolved.subcommand.is_resolved()),
                )
            },
            |invocation| invocation.facts.argument_offset,
        );
        if let Some(target) = arg_refs.get(base) {
            return DefTyping::Uniform(var_elements_effect_lattice(
                ctx,
                effect,
                target,
                arg_refs,
                base,
                proven.and_then(|invocation| invocation.effective.words.get(1..)),
            ));
        }
    }

    // How a command types the variable(s) it *writes* is a distinct
    // fact from the value it *returns*.  A destructuring writer
    // (`scan`, `regexp`, `binary scan`) returns a match/convert count
    // while writing element-wise pieces; `gets` returns the character
    // count while writing a text line; `lassign` writes its
    // container's *elements* positionally.  The registry declares this
    // per command / subcommand (`VarWriteTyping`), so the compiler
    // never keys on the command name.  Resolved through `canon`, not
    // the source spelling, so an aliased / renamed destructuring
    // writer (`rename lassign mylassign`) still resolves to the real
    // command's `VarWriteTyping` (FP-SH-15).
    let typing = proven.map_or_else(
        || {
            resolved
                .as_ref()
                .map_or(VarWriteTyping::ReturnValue, |resolved| {
                    resolved.semantics.var_write_typing
                })
        },
        |invocation| invocation.facts.var_write_typing,
    );
    match typing {
        // The default typing stores the command's *return value* in the
        // target — meaningful only for a single-target writer (`append`,
        // `lappend`). A call that writes *several* variables under the
        // default (no override) is not a single-value writer: the
        // synthetic `catch {body} resultVar optionsVar` / `try …` calls
        // carry the body's writes plus the result / options vars as defs
        // while `catch` / `try` return an Int status code, none of which
        // is that status. Broadcasting the return type onto all of them
        // would mistype every such variable, so stay conservative — the
        // old `defs.len() > 1` fallback, now scoped to the default arm
        // rather than a blanket heuristic (a registry `Destructured` /
        // `Fixed` override still applies at any def count).
        VarWriteTyping::ReturnValue if defs.len() > 1 => {
            DefTyping::Uniform(TypeLattice::overdefined())
        }
        VarWriteTyping::ReturnValue => DefTyping::Uniform(proven.map_or_else(
            || {
                return_type_for_command(
                    ctx.registry,
                    canon,
                    arg_refs,
                    ctx.known_classes,
                    ctx.namespace,
                )
            },
            |invocation| {
                invocation
                    .facts
                    .return_type
                    .map_or_else(TypeLattice::overdefined, TypeLattice::of)
            },
        )),
        VarWriteTyping::Fixed(t) => DefTyping::Uniform(TypeLattice::of(t)),
        VarWriteTyping::Destructured => DefTyping::Uniform(TypeLattice::overdefined()),
        VarWriteTyping::ElementsOf { container_arg } => elements_of_def_typing(
            ctx,
            arg_refs,
            defs,
            foreach_groups.as_deref(),
            container_arg,
        ),
    }
}

/// Positional element typing for a [`VarWriteTyping::ElementsOf`] writer.
///
/// Two shapes reach here:
/// - **`lassign $l a b …`** (no `foreach_groups`): target `i` takes element
///   `i` of the container. A tracked `Exact` container types each target
///   from its position — a position past the container's arity is the empty
///   string `lassign` pads with (`String`), an untracked position widens to
///   `Overdefined`.
/// - **`foreach` / `lmap` / `dict for` headers** (`foreach_groups`
///   present): `defs` is the flattened per-group element vars and `args`
///   holds one container word per group. A single-var list group's element
///   var is typed `String` (list elements stringify — the established
///   convention the shimmer suite pins) unless the container tracks a
///   uniform element shape; multi-var groups type var `j` from the join of
///   the `Exact` positions congruent to `j` mod the group size, else
///   `Overdefined` (the old multi-def behaviour). A dict group types its
///   value var from the dict's tracked value shape when one exists.
pub(super) fn elements_of_def_typing<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    args: &[&str],
    defs: &[String],
    foreach_groups: Option<&[usize]>,
    container_arg: u8,
) -> DefTyping {
    let mut per_def = VariableCellTable::default();

    if let Some(groups) = foreach_groups {
        let mut def_cursor = 0usize;
        for (group, &nvars) in groups.iter().enumerate() {
            let group_defs = defs.get(def_cursor..def_cursor + nvars).unwrap_or(&[]);
            def_cursor += nvars;
            let container_shape = args.get(group).and_then(|w| container_word_shape(ctx, w));
            // A substituted container of no known type (`[list $a $b]` over
            // untyped words) gives its elements no known intrep either.
            let unknown_substitution = container_shape.is_none()
                && args.get(group).is_some_and(|word| {
                    let word = word.trim();
                    word.starts_with('[')
                        && value_word_type(ctx, word, None).kind() == TypeKind::Overdefined
                });
            for (j, def) in group_defs.iter().enumerate() {
                per_def.insert(
                    ctx.variable_key(def).unwrap_or_else(|| def.clone().into()),
                    if unknown_substitution {
                        TypeLattice::overdefined()
                    } else {
                        foreach_var_lattice(container_shape.as_ref(), nvars, j)
                    },
                );
            }
        }
        return DefTyping::PerDef(per_def);
    }

    // lassign: targets follow the container word.
    let container = args.get(usize::from(container_arg));
    let elements = container
        .filter(|w| ctx.is_variable_word(w))
        .and_then(|w| ctx.written_variable_type(w, None))
        .and_then(|t| t.elements().cloned());
    for (i, def) in defs.iter().enumerate() {
        let lattice = match &elements {
            Some(Elements::Exact(shapes)) => {
                if i >= shapes.len() {
                    // Past the container's arity: `lassign` pads with the
                    // empty string.
                    TypeLattice::of(TclType::String)
                } else {
                    shapes[i]
                        .clone()
                        .map_or_else(TypeLattice::overdefined, TypeLattice::of_shape)
                }
            }
            _ => TypeLattice::overdefined(),
        };
        per_def.insert(
            ctx.variable_key(def).unwrap_or_else(|| def.clone().into()),
            lattice,
        );
    }
    DefTyping::PerDef(per_def)
}

/// The container shape a `foreach` group's list word denotes, when tracked.
fn container_word_shape<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
) -> Option<TypeShape> {
    let word = word.trim();
    if ctx.is_variable_word(word) {
        return ctx
            .written_variable_type(word, None)
            .and_then(|t| t.single_shape().cloned());
    }
    if word.starts_with('[') && word.ends_with(']') {
        return value_word_type(ctx, word, None).single_shape().cloned();
    }
    None
}

/// The lattice for one `foreach` element variable — var `j` of an
/// `nvars`-wide group iterating `container_shape`.
fn foreach_var_lattice(container_shape: Option<&TypeShape>, nvars: usize, j: usize) -> TypeLattice {
    match container_shape {
        Some(TypeShape::List(elements)) => {
            if nvars == 1 {
                match elements.uniform_shape() {
                    Some(u) => TypeLattice::of_shape(u),
                    // List elements stringify — the established convention.
                    None => TypeLattice::of(TclType::String),
                }
            } else if let Elements::Exact(shapes) = elements {
                // Var `j` sees positions j, j+nvars, j+2·nvars, …
                let mut acc: Option<TypeShape> = None;
                for shape in shapes.iter().skip(j).step_by(nvars) {
                    let Some(shape) = shape else {
                        return TypeLattice::overdefined();
                    };
                    acc = Some(match acc {
                        None => shape.clone(),
                        Some(prev) => match shape_join(&prev, shape) {
                            Some(joined) => joined,
                            None => return TypeLattice::overdefined(),
                        },
                    });
                }
                acc.map_or_else(
                    // No positions at this stride: the var gets the empty
                    // string `foreach` pads with.
                    || TypeLattice::of(TclType::String),
                    TypeLattice::of_shape,
                )
            } else {
                TypeLattice::overdefined()
            }
        }
        // A dict group (`dict for {k v} $d`): the key stringifies, the value
        // takes the dict's tracked value shape. Without a tracked value
        // shape both stay conservative (the old multi-def behaviour).
        Some(TypeShape::Dict(elements)) => {
            if nvars == 2 && j == 1 {
                elements
                    .uniform_shape()
                    .map_or_else(TypeLattice::overdefined, TypeLattice::of_shape)
            } else if nvars == 2 && j == 0 && elements.uniform_shape().is_some() {
                TypeLattice::of(TclType::String)
            } else {
                TypeLattice::overdefined()
            }
        }
        // Untracked container: a single-var group keeps the established
        // `String` element convention; wider groups stay conservative.
        _ => {
            if nvars == 1 {
                TypeLattice::of(TclType::String)
            } else {
                TypeLattice::overdefined()
            }
        }
    }
}
