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

//! Element facts of lists and dictionaries: what a builder's words contribute and how an in-place write evolves them.

use super::WordTypingCtx;
use super::word_types::value_word_type;
use crate::analyses::{ConstValue, LatticeValue};
use crate::shimmer::hints::is_pure_intrep;
use crate::types::{
    Elements, MAX_EXACT_ELEMENTS, TypeLattice, TypeShape, join_elements, shape_join,
};
use tcl_registry::{ReturnElements, TclType, VarElementsEffect};

/// The shape a builder argument *word* contributes as a container element,
/// or `None` when no shape claim is defensible for downstream checks.
///
/// Faithful to the runtime's by-reference elements (see docs
/// type-tracking.md): a container shares its argument objects, so a
/// **committed** source keeps its intrep inside the container —
/// `[list [expr {2**20}] x]` genuinely holds an int, `[list [C new]]` an
/// object. A **pure** source (a literal word, an interpolation, a
/// still-pure variable, a string-returning command) contributes a pure
/// string whose downstream conversion validity depends on its *value* — a
/// fact the type lattice cannot carry per element — so those positions stay
/// agnostic rather than invite the FP classes either concrete claim brings
/// (FP-SH-17 pins `lassign {1 2 3}` targets silent in both arithmetic and
/// string comparisons). The purity split is [`is_pure_intrep`] — the same
/// predicate the shimmer suppression uses.
fn element_word_shape<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
    retained: Option<&crate::ir::WordExpr>,
) -> Option<TypeShape> {
    let stripped = word.trim();
    if stripped.starts_with("{*}") {
        return None;
    }
    if ctx.is_variable_word(stripped) {
        let t = ctx.written_variable_type(stripped, retained)?;
        let shape = t.single_shape()?.clone();
        let constant = retained.map_or_else(
            || ctx.constant_of_word(stripped),
            |word| {
                let read = ctx.source.read_word(word)?;
                ctx.values.get(&(read.symbol, read.version?))
            },
        );
        if is_pure_intrep(shape.coarse(), constant) {
            return None;
        }
        return Some(shape);
    }
    if stripped.starts_with('[') && stripped.ends_with(']') {
        let shape = value_word_type(ctx, stripped, retained)
            .single_shape()?
            .clone();
        // A string-returning command (`string trim`, `format`) yields a pure
        // result — no committed intrep enters the container.
        if is_pure_intrep(shape.coarse(), None) {
            return None;
        }
        return Some(shape);
    }
    // Bare literal or interpolation: a fresh pure string object of known
    // spelling but per-element-untracked value.
    None
}

/// Element facts for a builder's result, per the registry's
/// [`ReturnElements`] declaration. `None` when the fact does not apply to
/// this call shape (a multi-level `dict get`, an unknown container).
pub(super) fn return_elements_lattice<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    fact: ReturnElements,
    args: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Option<TypeLattice> {
    match fact {
        ReturnElements::ListOfArgs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            Some(TypeLattice::of_shape(TypeShape::List(
                build_exact_elements(
                    ctx,
                    words,
                    retained.and_then(|words| words.get(usize::from(from)..)),
                ),
            )))
        }
        ReturnElements::DictOfPairs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            // Values are the odd offsets of the key/value pairs.
            let value_words: Vec<&str> = words.iter().skip(1).step_by(2).copied().collect();
            let retained_values = retained
                .and_then(|words| words.get(usize::from(from)..))
                .map(|words| words.iter().skip(1).step_by(2).cloned().collect::<Vec<_>>());
            Some(TypeLattice::of_shape(TypeShape::Dict(uniform_elements_of(
                ctx,
                &value_words,
                retained_values.as_deref(),
            ))))
        }
        ReturnElements::ElementOf { container_arg } => {
            // Single-step retrieval only: exactly one index/key word after
            // the container (`lindex $l $i`, `dict get $d $k`).
            let container_idx = usize::from(container_arg);
            if args.len() != container_idx + 2 {
                return None;
            }
            let container = args.get(container_idx)?;
            if !ctx.is_variable_word(container) {
                return None;
            }
            let t = ctx.written_variable_type(
                container,
                retained.and_then(|words| words.get(container_idx)),
            )?;
            let elements = t.elements()?;
            // A constant integer index resolves an Exact position; any
            // other index falls back to the uniform bound.
            let index_word = args.get(container_idx + 1)?;
            let shape = constant_index(ctx, index_word)
                .and_then(|i| elements.shape_at(i).cloned())
                .or_else(|| elements.uniform_shape())?;
            Some(TypeLattice::of_shape(shape))
        }
        ReturnElements::SubListOf { container_arg } => {
            let container = args.get(usize::from(container_arg))?;
            if !ctx.is_variable_word(container) {
                return None;
            }
            let elements = ctx
                .written_variable_type(
                    container,
                    retained.and_then(|words| words.get(usize::from(container_arg))),
                )?
                .elements()?
                .uniform_shape()
                .map_or(Elements::Unknown, |u| Elements::Uniform(Box::new(u)));
            Some(TypeLattice::of_shape(TypeShape::List(elements)))
        }
    }
}

/// `Exact` per-position elements for a builder's argument words, widening to
/// a uniform bound (or no facts) past [`MAX_EXACT_ELEMENTS`] or on an
/// `{*}`-expansion word (whose element count is unknown).
fn build_exact_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    if words.iter().any(|w| w.trim().starts_with("{*}")) {
        return uniform_elements_of(ctx, words, retained);
    }
    if words.len() > MAX_EXACT_ELEMENTS {
        return uniform_elements_of(ctx, words, retained);
    }
    Elements::Exact(
        words
            .iter()
            .enumerate()
            .map(|(index, w)| {
                element_word_shape(ctx, w, retained.and_then(|words| words.get(index)))
            })
            .collect(),
    )
}

/// The uniform element bound of a word set: the single-shape join of every
/// word's shape, or no facts when any word is shapeless / unjoinable.
fn uniform_elements_of<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    let mut acc: Option<TypeShape> = None;
    for (index, word) in words.iter().enumerate() {
        // `{*}$expansion` contributes its *list's* element shapes, which are
        // unknown here — no uniform claim survives.
        let word = word.trim();
        let shape = if let Some(expanded) = word.strip_prefix("{*}") {
            let Some(TypeShape::List(elements)) = (if ctx.is_variable_word(expanded) {
                ctx.written_variable_type(expanded, None)
                    .and_then(|t| t.single_shape().cloned())
            } else {
                None
            }) else {
                return Elements::Unknown;
            };
            match elements.uniform_shape() {
                Some(u) => u,
                None => return Elements::Unknown,
            }
        } else {
            match element_word_shape(ctx, word, retained.and_then(|words| words.get(index))) {
                Some(s) => s,
                None => return Elements::Unknown,
            }
        };
        acc = Some(match acc {
            None => shape,
            Some(prev) => match shape_join(&prev, &shape) {
                Some(joined) => joined,
                None => return Elements::Unknown,
            },
        });
    }
    match acc {
        Some(shape) => Elements::Uniform(Box::new(shape)),
        None => Elements::Unknown,
    }
}

/// The constant element index a retrieval word denotes: a literal integer
/// (`lindex $l 0`) or a `$var` whose SCCP constant is an integer.
fn constant_index<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
) -> Option<usize> {
    let word = word.trim();
    if let Ok(i) = word.parse::<usize>() {
        return Some(i);
    }
    if ctx.is_variable_word(word)
        && let Some(LatticeValue::Const(ConstValue::Int(i))) = ctx.constant_of_word(word)
    {
        return usize::try_from(*i).ok();
    }
    None
}

/// The evolved container shape after an in-place element write —
/// `lappend var v…` / `dict set var … v` — per the registry's
/// [`VarElementsEffect`]. Generalises the old object-only element-class
/// harvesting: every element shape flows, so `lappend l [C new]` still
/// yields `List<OBJECT(C)*>` and `lappend l [list 1]` now yields real
/// element facts too.
pub(super) fn var_elements_effect_lattice<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    effect: VarElementsEffect,
    target: &str,
    args: &[&str],
    base: usize,
    retained: Option<&[crate::ir::WordExpr]>,
) -> TypeLattice {
    let (container_ctor, value_words): (fn(Elements) -> TypeShape, &[&str]) = match effect {
        VarElementsEffect::SetsArrayElementsFromList { .. } => return TypeLattice::overdefined(),
        VarElementsEffect::AppendsListElements { values_from } => (
            TypeShape::List,
            args.get(base + usize::from(values_from)..).unwrap_or(&[]),
        ),
        VarElementsEffect::SetsDictValue => (
            TypeShape::Dict,
            args.len()
                .checked_sub(1)
                .map_or(&[][..], |last| &args[last..]),
        ),
        VarElementsEffect::ListifiesDictValue => (TypeShape::Dict, &[]),
        VarElementsEffect::ExtendsDictValuesByName { values_from } => (
            TypeShape::Dict,
            args.get(base + usize::from(values_from)..).unwrap_or(&[]),
        ),
    };

    let retained_values =
        retained.and_then(|words| words.get(args.len().saturating_sub(value_words.len())..));
    let prior = prior_container_elements(ctx, target);
    let evolved = match effect {
        VarElementsEffect::SetsArrayElementsFromList { .. } => return TypeLattice::overdefined(),
        VarElementsEffect::AppendsListElements { .. } => {
            append_list_elements(ctx, prior, value_words, retained_values)
        }
        // Dict values: join the new value shape into the prior uniform
        // bound — per-key tracking is out of scope (type-tracking.md).
        // Only the single-key `dict set var key value` carries the leaf
        // word's shape; a nested path stores a *dict* under the first key
        // (`dict get $d outer` is a dict — tclsh-verified), so the
        // contribution is a Dict wrapper with unknown structure.
        VarElementsEffect::SetsDictValue => {
            let single_key = args.len().saturating_sub(base) == 3;
            let incoming = if single_key {
                uniform_elements_of(ctx, value_words, retained_values)
            } else {
                Elements::Uniform(Box::new(TypeShape::Dict(Elements::Unknown)))
            };
            match prior {
                Some(prior) => join_elements(&prior, &incoming),
                None => incoming,
            }
        }
        // `dict append` concatenates: value intreps do not survive, but an
        // object's dispatch identity (the objref text) does — only
        // object-class shapes flow into the bound (the collection-of-objects
        // pattern); anything else contributes no
        // element fact.
        VarElementsEffect::ExtendsDictValuesByName { .. } => {
            let incoming = uniform_elements_of(ctx, value_words, retained_values);
            let object_only = match &incoming {
                Elements::Uniform(shape) if matches!(**shape, TypeShape::Object(_)) => {
                    Some(incoming.clone())
                }
                _ => None,
            };
            match (prior, object_only) {
                (Some(prior), Some(inc)) => join_elements(&prior, &inc),
                (Some(prior), None) => prior,
                (None, Some(inc)) => inc,
                (None, None) => Elements::Unknown,
            }
        }
        // `dict lappend`: the key's value becomes a list (a prior scalar
        // becomes element 0), so the wrapper is the fact and the element
        // shapes stay unknown.
        VarElementsEffect::ListifiesDictValue => {
            let incoming = Elements::Uniform(Box::new(TypeShape::List(Elements::Unknown)));
            match prior {
                Some(prior) => join_elements(&prior, &incoming),
                None => incoming,
            }
        }
    };
    TypeLattice::of_shape(container_ctor(evolved))
}

/// The target variable's element facts *before* this write: its tracked
/// container elements, or — for a pure list-shaped constant (`set l {a b};
/// lappend l c`) — the parsed literal's per-position pure-string elements.
/// `None` when nothing is known (including an uninitialised accumulator).
fn prior_container_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    target: &str,
) -> Option<Elements> {
    if let Some(t) = ctx.named_cell_type(target) {
        if t.kind() == crate::types::TypeKind::Overdefined {
            return Some(Elements::Unknown);
        }
        if let Some(e) = t.elements() {
            return Some(e.clone());
        }
        // A pure string constant parses to a known element count of pure
        // strings — `lappend` on `{a b}` starts from two `String` elements.
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config);
        if t.tcl_type() == Some(TclType::String)
            && let Some(LatticeValue::Const(ConstValue::String(text))) =
                ctx.named_cell_constant(target)
            && let Ok(parsed) = rules.split_list(text)
        {
            if parsed.len() > MAX_EXACT_ELEMENTS {
                return Some(Elements::Uniform(Box::new(TypeShape::String)));
            }
            return Some(Elements::Exact(
                parsed.iter().map(|_| Some(TypeShape::String)).collect(),
            ));
        }
    }
    None
}

/// Append `value_words` as new elements after `prior`, keeping `Exact`
/// arity when both sides pin one (the phi joins collapse mismatched
/// loop-carried arities — see [`join_elements`] — so the fixpoint
/// terminates).
fn append_list_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    prior: Option<Elements>,
    value_words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    let appended = build_exact_elements(ctx, value_words, retained);
    match (prior, appended) {
        (None, appended) => appended,
        (Some(Elements::Exact(existing)), Elements::Exact(new)) => {
            let mut merged = existing.into_vec();
            merged.extend(new.into_vec());
            if merged.len() > MAX_EXACT_ELEMENTS {
                let shapes: Vec<TypeShape> = merged.into_iter().flatten().collect();
                return uniform_bound_of_shapes(&shapes);
            }
            Elements::Exact(merged.into_boxed_slice())
        }
        (Some(prior), appended) => {
            // No exact arity on one side: fall back to the joint uniform
            // bound when both sides admit one.
            match (prior.uniform_shape(), appended.uniform_shape()) {
                (Some(a), Some(b)) => match shape_join(&a, &b) {
                    Some(joined) => Elements::Uniform(Box::new(joined)),
                    None => Elements::Unknown,
                },
                _ => Elements::Unknown,
            }
        }
    }
}

/// The uniform bound over a shape list, or no facts when empty / unjoinable.
fn uniform_bound_of_shapes(shapes: &[TypeShape]) -> Elements {
    let mut acc: Option<TypeShape> = None;
    for shape in shapes {
        acc = Some(match acc {
            None => shape.clone(),
            Some(prev) => match shape_join(&prev, shape) {
                Some(joined) => joined,
                None => return Elements::Unknown,
            },
        });
    }
    match acc {
        Some(shape) => Elements::Uniform(Box::new(shape)),
        None => Elements::Unknown,
    }
}
