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

//! What a branch condition proves about places on its true and false edges.

use super::TclValue;
use super::arithmetic::split_tcl_list;
use super::fold_ops::{FoldValue, Operand, classify_operand};
use super::fold_policy::FoldPolicy;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use tcl_registry::value_transfer::{DomainFact, ExactValue, Existence, FactDomain, FactView};

// Edge refinement: what a branch condition's outcome proves

/// One place's fact on one edge of a branch: what the condition's outcome
/// along the edge proves about it, in one domain
/// (`docs/design/compiler/value-transfers.md` § *Predicate refinement*).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceFact {
    /// The place as the condition names it: the variable an operand reads,
    /// or the word an existence query asks about.
    pub(crate) place: String,
    /// The domain the fact narrows.
    pub(crate) domain: FactDomain,
    /// The fact in the domain's view: an exact value or a finite set of
    /// them for the value domain, a domain fact for every other.
    pub(crate) fact: FactView,
    /// Original query cell, when retained scoped metadata supplies it.
    pub(crate) original_cell: Option<crate::var_resolve::VariableCellKey>,
}

impl PlaceFact {
    /// `place` holds exactly `value`.
    pub(super) fn exact(place: &str, value: &str) -> Self {
        Self {
            place: place.to_owned(),
            domain: FactDomain::ExactValue,
            fact: FactView::Exact(ExactValue::from_literal(value), None),
            original_cell: None,
        }
    }

    /// `place` holds `fact` in `domain`.
    pub(super) fn domain(place: &str, domain: FactDomain, fact: DomainFact) -> Self {
        Self {
            place: place.to_owned(),
            domain,
            fact: FactView::Domain(fact),
            original_cell: None,
        }
    }

    /// `place`'s existence is `fact`.
    pub(super) fn existence(place: &str, fact: Existence) -> Self {
        Self::domain(place, FactDomain::Existence, DomainFact::Existence(fact))
    }
}

/// The facts one condition states on its true and on its false edge.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EdgeFacts {
    /// What the condition holding proves.
    pub(crate) on_true: Vec<PlaceFact>,
    /// What the condition failing proves.
    pub(crate) on_false: Vec<PlaceFact>,
}

impl EdgeFacts {
    /// `facts` on the true edge alone.
    fn when_true(facts: Vec<PlaceFact>) -> Self {
        Self {
            on_true: facts,
            on_false: Vec::new(),
        }
    }

    /// `facts` on the false edge alone.
    fn when_false(facts: Vec<PlaceFact>) -> Self {
        Self {
            on_true: Vec::new(),
            on_false: facts,
        }
    }
}

/// What the condition-tree transfer reads beside the tree: the registry an
/// existence query or a `string is` test resolves against, the document's
/// lexer configuration its substitutions are split under, the fold policy
/// whose numeral grammar decides whether `==` compares numbers, and whether
/// a command head still denotes the registry's command.
#[derive(Clone, Copy)]
pub(crate) struct ConditionReading<'a> {
    /// The registry the condition's commands resolve against.
    pub(crate) registry: &'a tcl_registry::CommandRegistry,
    /// The document's lexer configuration.
    pub(crate) config: tcl_lexer::LexerConfig,
    /// The target's fold policy.
    pub(crate) policy: FoldPolicy,
    /// Whether the module leaves a command head denoting the registry's
    /// command, which a `string is` test asks of `string`.
    pub(crate) trusted: &'a dyn Fn(&str) -> bool,
    /// Actual CFG metadata and original expression geometry for existence calls.
    pub(crate) existence: crate::existence_query::QueryReading<'a>,
}

/// The `Selection` transfer of a branch condition over its tree: per edge,
/// what the condition's outcome proves about the places it reads
/// (`docs/design/compiler/value-transfers.md` § *Predicate refinement*, the
/// per-shape table). `eq` proves its literal on the true edge and `ne` on
/// the false one; `==` and `!=` do the same where the literal is no number
/// under the target's numeral grammar, which makes the comparison a string
/// one, and otherwise prove a number — the point when it is an integer —
/// and never the string; `<`, `<=`, `>` and `>=` against an integer prove
/// the half-line an integer value lies on, on each edge; `in` and `ni`
/// prove the list's finite set; a
/// `string is` test proves the type its members share, an existence query
/// the place's existence; `!` swaps the edges, `C1 && C2` proves both
/// true-edge answers on its true edge and `C1 || C2` both false-edge
/// answers on its false edge — `C1`'s only when `C2`, which runs after it,
/// changes no place ([`changes_no_place`]). The variable must be one plain
/// local operand and the other side a literal: `$x` alone, `$x eq $y`, an
/// element and a qualified name prove nothing.
pub(crate) fn condition_edge_facts(node: &ExprNode, reading: ConditionReading<'_>) -> EdgeFacts {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not | UnaryOp::WordNot,
            operand,
        } => {
            let EdgeFacts { on_true, on_false } = condition_edge_facts(operand, reading);
            EdgeFacts {
                on_true: on_false,
                on_false: on_true,
            }
        }
        // The right operand runs after the left one, so the left operand's
        // facts reach the edge only when the right one changes no place.
        ExprNode::Binary {
            op: BinOp::And,
            left,
            right,
        } => {
            let mut on_true = condition_edge_facts(right, reading).on_true;
            if changes_no_place(right, reading) {
                on_true.extend(condition_edge_facts(left, reading).on_true);
            }
            EdgeFacts::when_true(on_true)
        }
        ExprNode::Binary {
            op: BinOp::Or,
            left,
            right,
        } => {
            let mut on_false = condition_edge_facts(right, reading).on_false;
            if changes_no_place(right, reading) {
                on_false.extend(condition_edge_facts(left, reading).on_false);
            }
            EdgeFacts::when_false(on_false)
        }
        ExprNode::Binary { op, left, right } => comparison_facts(*op, left, right, reading),
        ExprNode::Command { .. } => reading.existence.in_expr_with_context(node).map_or_else(
            || EdgeFacts::when_true(string_is_fact(node, reading).into_iter().collect()),
            |(query, context)| {
                let mut facts = query_facts(&query.var, query.kind);
                if let Some(context) = context {
                    for fact in facts.on_true.iter_mut().chain(&mut facts.on_false) {
                        let place = crate::var_resolve::resolve_literal_place(
                            &fact.place,
                            &context,
                            false,
                            reading.registry,
                        );
                        if !place.dynamic
                            && !place.observed
                            && !context.implicit_read_changes_value(&place, reading.registry)
                        {
                            fact.original_cell = crate::var_resolve::canonical_place_key(&place);
                        }
                    }
                }
                facts
            },
        ),
        _ => EdgeFacts::default(),
    }
}

/// Original cells known bound at an exact read inside a lazy condition.
/// Prefix effects retire earlier predicates; the existing edge transfer owns
/// which facts the newly evaluated predicate contributes.
pub(crate) fn condition_read_guards(
    node: &ExprNode,
    expression_base: u32,
    reading: ConditionReading<'_>,
) -> Vec<(tcl_lexer::Span, crate::var_resolve::VariableCellKey)> {
    let mut guards = Vec::new();
    let mut pending = vec![(node, Vec::new())];
    while let Some((node, incoming)) = pending.pop() {
        match node {
            ExprNode::Binary { op, left, right } => {
                let mut continuing = if changes_no_place(left, reading) {
                    incoming.clone()
                } else {
                    Vec::new()
                };
                let facts = condition_edge_facts(left, reading);
                let selected = match op {
                    BinOp::And | BinOp::WordAnd => facts.on_true,
                    BinOp::Or | BinOp::WordOr => facts.on_false,
                    _ => Vec::new(),
                };
                continuing.extend(bound_original_cells(selected));
                pending.push((right, continuing));
                pending.push((left, incoming));
            }
            ExprNode::Unary { operand, .. } => pending.push((operand, incoming)),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                let continuing = if changes_no_place(condition, reading) {
                    incoming.clone()
                } else {
                    Vec::new()
                };
                let facts = condition_edge_facts(condition, reading);
                let mut on_true = continuing.clone();
                on_true.extend(bound_original_cells(facts.on_true));
                let mut on_false = continuing;
                on_false.extend(bound_original_cells(facts.on_false));
                pending.push((false_branch, on_false));
                pending.push((true_branch, on_true));
                pending.push((condition, incoming));
            }
            ExprNode::Var { .. } if changes_no_place(node, reading) => {
                if let Some(span) = node.variable_source_span(expression_base, reading.config) {
                    guards.extend(incoming.into_iter().map(|cell| (span, cell)));
                }
            }
            // Nested scripts/words and dispatched math functions need their
            // own original read and evaluation-order owners.
            _ => {}
        }
    }
    guards
}

fn bound_original_cells(
    facts: Vec<PlaceFact>,
) -> impl Iterator<Item = crate::var_resolve::VariableCellKey> {
    facts.into_iter().filter_map(|fact| {
        matches!(
            fact.fact,
            FactView::Domain(DomainFact::Existence(Existence::Bound(_)))
        )
        .then_some(fact.original_cell)
        .flatten()
    })
}

/// Whether evaluating `node` changes no place: every command it substitutes
/// is an existence query over a name that runs no command, and no operand
/// substitutes a command of its own. A math function may be a procedure,
/// and an unparsed operand may be anything, so neither is.
fn changes_no_place(node: &ExprNode, reading: ConditionReading<'_>) -> bool {
    match node {
        ExprNode::Command { .. } => reading.existence.query_changes_no_place(node),
        ExprNode::Binary { left, right, .. } => {
            changes_no_place(left, reading) && changes_no_place(right, reading)
        }
        ExprNode::Unary { operand, .. } => changes_no_place(operand, reading),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => [condition, true_branch, false_branch]
            .iter()
            .all(|operand| changes_no_place(operand, reading)),
        ExprNode::Literal { .. } => true,
        ExprNode::Var { text, .. } => {
            !text.contains('[') && reading.existence.original_unobserved_variable(node)
        }
        ExprNode::String { text, .. } | ExprNode::CompiledWord { text, .. } => {
            reading.existence.text_changes_no_place(text)
        }
        ExprNode::Call { .. } | ExprNode::Raw { .. } => false,
    }
}

/// The places one existence query states a fact about, on its true and on
/// its false edge. `info exists` of a whole place binds it on the true
/// edge, as either kind, and unbinds it on the false edge; of a literal
/// element it binds the element as a scalar and its array as an array on
/// the true edge and unbinds the element on the false edge; of an element
/// under a computed key it binds the array, when the base is a bareword,
/// on the true edge. `array exists` of a whole place binds it as an array
/// on the true edge and states nothing on the false edge, where the place
/// may be a scalar or absent; of an element it states nothing.
fn query_facts(name: &str, kind: crate::existence_query::ExistenceKind) -> EdgeFacts {
    use crate::existence_query::ExistenceKind;
    use tcl_registry::value_transfer::BindingKind;
    let computed = name.contains('$') || name.contains('[');
    let base = crate::sccp::place_base(name);
    match kind {
        _ if computed => match (kind, crate::existence_query::computed_element_base(name)) {
            (ExistenceKind::AnyVariable, Some(base)) => {
                EdgeFacts::when_true(vec![PlaceFact::existence(
                    base,
                    Existence::Bound(BindingKind::Array),
                )])
            }
            _ => EdgeFacts::default(),
        },
        ExistenceKind::AnyVariable if base == name => EdgeFacts {
            on_true: vec![PlaceFact::existence(
                name,
                Existence::Bound(BindingKind::Either),
            )],
            on_false: vec![PlaceFact::existence(name, Existence::Unbound)],
        },
        ExistenceKind::AnyVariable => EdgeFacts {
            on_true: vec![
                PlaceFact::existence(name, Existence::Bound(BindingKind::Scalar)),
                PlaceFact::existence(base, Existence::Bound(BindingKind::Array)),
            ],
            on_false: vec![PlaceFact::existence(name, Existence::Unbound)],
        },
        ExistenceKind::Array if base == name => EdgeFacts::when_true(vec![PlaceFact::existence(
            name,
            Existence::Bound(BindingKind::Array),
        )]),
        ExistenceKind::Array => EdgeFacts::default(),
    }
}

/// The facts a comparison of one plain local variable with a literal
/// proves: `eq`'s and `ne`'s exact value, `==`'s and `!=`'s exact value or
/// number ([`equality_facts`]), the half-line of an ordered comparison with
/// an integer ([`ordering_facts`]), and the finite set of `in`'s and `ni`'s
/// list ([`membership_facts`]). A comparison reads either side as the
/// variable, an ordered one turned round when the literal comes first
/// ([`turned_round`]); membership asks whether the variable is in the list,
/// never the reverse.
fn comparison_facts(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    reading: ConditionReading<'_>,
) -> EdgeFacts {
    let operands = variable_operand(left, reading.config)
        .zip(literal_operand(right))
        .map(|(place, literal)| (op, place, literal))
        .or_else(|| {
            let turned = turned_round(op)?;
            literal_operand(left)
                .zip(variable_operand(right, reading.config))
                .map(|(literal, place)| (turned, place, literal))
        });
    let Some((op, place, literal)) = operands else {
        return EdgeFacts::default();
    };
    match op {
        BinOp::StrEq => EdgeFacts::when_true(vec![PlaceFact::exact(&place, &literal)]),
        BinOp::StrNe => EdgeFacts::when_false(vec![PlaceFact::exact(&place, &literal)]),
        BinOp::Eq => EdgeFacts::when_true(equality_facts(&place, &literal, reading.policy)),
        BinOp::Ne => EdgeFacts::when_false(equality_facts(&place, &literal, reading.policy)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            ordering_facts(op, &place, &literal, reading.policy)
        }
        BinOp::In => EdgeFacts::when_true(membership_facts(&place, &literal, reading.policy)),
        BinOp::Ni => EdgeFacts::when_false(membership_facts(&place, &literal, reading.policy)),
        _ => EdgeFacts::default(),
    }
}

/// The comparison `L op R` read as `R op' L`: an equality as it is, an
/// ordered comparison turned round (`5 < $x` is `$x > 5`); `in` and `ni`,
/// which ask about their left operand, never.
fn turned_round(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::StrEq | BinOp::StrNe | BinOp::Eq | BinOp::Ne => Some(op),
        BinOp::Lt => Some(BinOp::Gt),
        BinOp::Le => Some(BinOp::Ge),
        BinOp::Gt => Some(BinOp::Lt),
        BinOp::Ge => Some(BinOp::Le),
        _ => None,
    }
}

/// What an ordered comparison of `x` with an integer numeral proves on each
/// edge: the half-line `x` lies on when it is an integer. `$x < 5` holding
/// puts an integer `x` at 4 or below, and failing at 5 or above, the false
/// edge reading the operator's inverse ([`BinOp::inverse`]). The range is
/// conditional on the value being an integer: a value that is not compares
/// as a double (`2.5`, NaN, which fails every ordered comparison) or as a
/// string (`abc`, `end`), and an integer interval says nothing of it, so a
/// consumer that reads a value that may be no integer as one reads past the
/// fact. A literal that is no integer under the target's numeral grammar
/// proves nothing: a word, `true` and `false` among them, which a
/// comparison reads as a string; a double; a bignum; a leading zero the
/// target's release leaves open. A bound at the edge of the 64-bit range
/// saturates, one value wide, rather than wrapping.
pub(super) fn ordering_facts(
    op: BinOp,
    place: &str,
    literal: &str,
    policy: FoldPolicy,
) -> EdgeFacts {
    let numbers = policy.numbers.unwrap_or_default();
    let Operand::Num(TclValue::Int(bound)) =
        classify_operand(&FoldValue::Str(literal.to_owned()), policy.octal, numbers)
    else {
        return EdgeFacts::default();
    };
    let half_line = |op: BinOp| {
        let (lo, hi) = match op {
            BinOp::Lt => (None, Some(bound.saturating_sub(1))),
            BinOp::Le => (None, Some(bound)),
            BinOp::Gt => (Some(bound.saturating_add(1)), None),
            BinOp::Ge => (Some(bound), None),
            _ => return Vec::new(),
        };
        vec![PlaceFact::domain(
            place,
            FactDomain::Range,
            DomainFact::Range { lo, hi },
        )]
    };
    EdgeFacts {
        on_true: half_line(op),
        on_false: op.inverse().map_or_else(Vec::new, half_line),
    }
}

/// What `$x == LIT` holding proves. A literal that is no number under the
/// target's numeral grammar makes the comparison a string one, so `x` is
/// exactly `LIT`; a number makes it a numeric one, so `x` is a number equal
/// to it — the integer point when `LIT` is an integer — whose string may be
/// any spelling of it (`1.0`, ` 1`, `01`), so never `LIT`. A leading-zero
/// literal whose reading the target's release leaves open proves nothing,
/// and neither does a NaN, which equals nothing.
fn equality_facts(place: &str, literal: &str, policy: FoldPolicy) -> Vec<PlaceFact> {
    let numbers = policy.numbers.unwrap_or_default();
    match classify_operand(&FoldValue::Str(literal.to_owned()), policy.octal, numbers) {
        Operand::Str => vec![PlaceFact::exact(place, literal)],
        Operand::Num(TclValue::Float(f)) if f.is_nan() => Vec::new(),
        Operand::Num(value) => {
            let mut facts = vec![PlaceFact::domain(
                place,
                FactDomain::Type,
                DomainFact::Type {
                    intrep: Some(tcl_registry::TclType::Numeric),
                    shape: None,
                },
            )];
            if let TclValue::Int(point) = value {
                facts.push(PlaceFact::domain(
                    place,
                    FactDomain::Range,
                    DomainFact::Range {
                        lo: Some(point),
                        hi: Some(point),
                    },
                ));
            }
            facts
        }
        Operand::Ambiguous => Vec::new(),
    }
}

/// What `$x in LIST` holding proves: `x` is one of the list's elements, as
/// the target's list rules split it — a finite set, compared as strings. A
/// list with no element, or more than the lattice holds as a set, proves
/// nothing.
fn membership_facts(place: &str, list: &str, policy: FoldPolicy) -> Vec<PlaceFact> {
    let mut members: Vec<String> = Vec::new();
    for element in split_tcl_list(list, policy.word_rules) {
        if !members.contains(&element) {
            members.push(element);
        }
    }
    match members.as_slice() {
        [] => Vec::new(),
        [one] => vec![PlaceFact::exact(place, one)],
        _ if members.len() > crate::analyses::MAX_CONSTSET_SIZE => Vec::new(),
        _ => vec![PlaceFact {
            place: place.to_owned(),
            domain: FactDomain::ExactValue,
            fact: FactView::Finite(
                members
                    .iter()
                    .map(|member| ExactValue::from_literal(member))
                    .collect(),
                None,
            ),
            original_cell: None,
        }],
    }
}

/// The true edge of `[string is CLASS ?-strict? $x]`: the type every value
/// the test accepts belongs to
/// ([`tcl_registry::commands::tcl::string_is_member_type`]), for the one
/// variable the test's last word reads. Every other word is literal, and
/// the head is the registry's command the module has not rebound.
fn string_is_fact(node: &ExprNode, reading: ConditionReading<'_>) -> Option<PlaceFact> {
    if !reading.existence.is_standalone() {
        let tokens = reading.existence.original_command(node)?;
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                reading.registry,
                reading.existence.metadata_context(),
                &tokens,
            )?;
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::StringIs)
            || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let value_index = invocation.arguments.len().checked_sub(1)?;
        if !matches!(
            invocation
                .effective
                .origins
                .get(value_index.checked_add(1)?),
            Some(crate::registry_invocation::InvocationWordOrigin::Written(_))
        ) {
            return None;
        }
        let value = invocation
            .effective
            .words
            .get(value_index.checked_add(1)?)?;
        let (spelling, _) = value.sole_variable_substitution()?;
        let place = variable_name(spelling, reading.config)?;
        let mut values = (invocation.facts.argument_offset..value_index)
            .map(|index| invocation.argument_literal(index))
            .collect::<Option<Vec<_>>>()?;
        // Class selection consumes the final value's shape, not its contents.
        values.push(String::new());
        let arguments = values.iter().map(String::as_str).collect::<Vec<_>>();
        let member = tcl_registry::commands::tcl::string_is_member_type_for_dialect(
            &arguments,
            invocation.dialect?,
        )?;
        return Some(PlaceFact::domain(
            place,
            FactDomain::Type,
            DomainFact::Type {
                intrep: Some(member),
                shape: None,
            },
        ));
    }
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let commands = crate::ir_helpers::tokenise_command_words(inner, reading.config);
    let [words] = commands.as_slice() else {
        return None;
    };
    let (head, args) = words.split_first()?;
    let head = head.literal()?;
    let (value, options) = args.split_last()?;
    if value.expanded || !(reading.trusted)(head) {
        return None;
    }
    let place = variable_name(&value.raw, reading.config)?;
    let mut texts: Vec<&str> = options
        .iter()
        .map(crate::ir_helpers::CommandWord::literal)
        .collect::<Option<_>>()?;
    texts.push(&value.text);
    let operation = reading
        .registry
        .resolve_invocation(head, &texts, reading.registry.own_surface_query())?
        .semantics
        .operation;
    if operation
        != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::StringIs)
    {
        return None;
    }
    let member = tcl_registry::commands::tcl::string_is_member_type(texts.get(1..)?)?;
    Some(PlaceFact::domain(
        place,
        FactDomain::Type,
        DomainFact::Type {
            intrep: Some(member),
            shape: None,
        },
    ))
}

/// The plain local variable an operand reads whole: a `$x` or `${x}`
/// reference, or the flattened `switch` subject that carries one as `Raw`
/// text. A qualified name or an array element is no plain local.
fn variable_operand(node: &ExprNode, config: tcl_lexer::LexerConfig) -> Option<String> {
    match node {
        ExprNode::Var { text, .. } | ExprNode::Raw { text } => {
            variable_name(text, config).map(str::to_owned)
        }
        _ => None,
    }
}

/// The plain local name a `$x` or `${x}` spelling reads.
pub(super) fn variable_name(text: &str, config: tcl_lexer::LexerConfig) -> Option<&str> {
    crate::value_transfer::whole_variable_operand(text, config).filter(|name| {
        !name.is_empty()
            && !tcl_syntax::naming::is_qualified(name.as_bytes())
            && !name.contains('(')
    })
}

/// The value an operand's literal spelling holds: a bare literal's text, a
/// string operand's body where it is its value in every dialect
/// ([`tcl_syntax::expr::fixed_string_operand`]), or a compiled word's value
/// when it was braced or substitutes nothing.
fn literal_operand(node: &ExprNode) -> Option<String> {
    match node {
        ExprNode::Literal { text, .. } => Some(text.clone()),
        ExprNode::String { text, .. } => {
            tcl_syntax::expr::fixed_string_operand(text).map(str::to_owned)
        }
        ExprNode::CompiledWord { text, braced } => {
            (*braced || !text.contains(['$', '[', '\\'])).then(|| text.clone())
        }
        _ => None,
    }
}
