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

//! Evaluating a statement's definitions and a branch condition over the lattice, and the literal and `foreach` list ingress.

use super::BranchFold;
use crate::analyses::{ConstValue, LatticeValue};
use crate::codegen_helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaSourceView, SsaStatement, Symbol, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::HashMap;
use tcl_lexer::TokenType;
use tcl_registry::value_transfer::ExactValue;

/// Evaluate the lattice value produced by an SSA statement's
/// defs.
///
/// Focused subset: constant-assignment, expression-assignment via the
/// expression evaluator, the registry-declared value transfer of a typed
/// cell update, a synthetic loop header, or a command substitution, and a
/// conservative `Overdefined` fallback for everything else.
///
/// What one statement's evaluation leaves in its definitions: the one
/// value a typed statement computes, which every definition takes, or a
/// call's value per definition, from its ordered stores.
pub(crate) enum DefValues {
    /// Every definition takes this value, and the folded type the
    /// evaluation that produced it states.
    Each(LatticeValue, Option<crate::value_transfer::FoldedType>),
    /// Each definition's own answer; a definition absent here widens.
    PerDef(Vec<crate::value_transfer::DefAnswer>),
    /// The statement certainly raises, where a throw leaves from: its
    /// definitions are what the stores that ran left.
    Raised(Box<RaisedDefs>),
}

/// A statement that certainly raises, after the stores that ran.
pub(crate) struct RaisedDefs {
    /// Each definition after those stores; `None` when nothing was stored,
    /// so every definition keeps what its place held.
    pub(crate) prefix: Option<Vec<crate::value_transfer::DefAnswer>>,
    /// How many stores ran before the error.
    pub(crate) written: usize,
}

impl DefValues {
    /// The per-definition answers, for the shapes that have them.
    pub(super) fn answers(&self) -> Option<&[crate::value_transfer::DefAnswer]> {
        match self {
            Self::Each(..) => None,
            Self::PerDef(answers) => Some(answers),
            Self::Raised(raised) => raised.prefix.as_deref(),
        }
    }

    /// The value definition `key` takes.
    pub(super) fn of(&self, key: ValueKey) -> LatticeValue {
        match self {
            Self::Each(value, _) => value.clone(),
            Self::PerDef(_) | Self::Raised(_) => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .map_or(LatticeValue::Overdefined, |answer| answer.value.clone()),
        }
    }

    /// The folded type definition `key` takes, when its evaluation states
    /// one.
    pub(super) fn folded_of(&self, key: ValueKey) -> Option<crate::value_transfer::FoldedType> {
        match self {
            Self::Each(_, folded) => folded.clone(),
            Self::PerDef(_) | Self::Raised(_) => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .and_then(|answer| answer.folded.clone()),
        }
    }

    /// Whether an evaluated outcome's stores name definition `key`'s place
    /// ([`crate::value_transfer::DefAnswer::stated`]).
    pub(super) fn stated(&self, key: ValueKey) -> bool {
        self.answers().is_some_and(|answers| {
            answers
                .iter()
                .any(|answer| answer.key == key && answer.stated)
        })
    }

    /// Whether the outcome left definition `key`'s place untouched
    /// ([`crate::value_transfer::DefAnswer::preserved`]).
    pub(super) fn preserved(&self, key: ValueKey) -> bool {
        self.answers().is_some_and(|answers| {
            answers
                .iter()
                .any(|answer| answer.key == key && answer.preserved)
        })
    }

    /// The existence step definition `key` takes
    /// ([`crate::value_transfer::DefAnswer::existence`]); a statement the
    /// evaluation has no per-definition answer for widens, and one that
    /// raised before it stored anything leaves every place as it was.
    pub(super) fn existence(&self, key: ValueKey) -> crate::value_transfer::ExistenceStep {
        use crate::value_transfer::ExistenceStep;
        match self {
            Self::Raised(raised) if raised.prefix.is_none() => ExistenceStep::PRESERVE,
            _ => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .map_or(ExistenceStep::UNKNOWN, |answer| answer.existence),
        }
    }
}

/// Resolve each typed definition through the run's actual Registry driver.
pub(super) fn evaluate_defs_under<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> DefValues {
    driver.explaining(Some(stmt_ssa.statement.span()));
    let value = evaluate_def_dispatch(stmt_ssa, values, ssa, driver);
    driver.explaining(None);
    value
}

/// The typed dispatch of [`evaluate_defs_under`].
fn evaluate_def_dispatch<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> DefValues {
    let value = match &stmt_ssa.statement {
        // A typed assignment is `set`'s lowering, so it means nothing once
        // the module rebinds `set`.
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
            if !driver.typed_assignment_trusted() =>
        {
            LatticeValue::Overdefined
        }
        // A whole-word command substitution's value comes with the folded
        // type its route states.
        Statement::AssignValue {
            value,
            value_needs_backsubst,
            ..
        } => {
            let (value, folded) = fold_assign_value(
                value,
                *value_needs_backsubst,
                &stmt_ssa.statement,
                &stmt_ssa.uses,
                values,
                ssa,
                driver,
            );
            return DefValues::Each(value, folded);
        }
        // The value is a braced word's content by construction (the
        // lowering's other arm is a canonical integer), so it is read as
        // Tcl reads a braced word: backslash-newline collapses.
        Statement::AssignConst { value, .. } => driver
            .literal_value(value, TokenType::Str)
            .map_or(LatticeValue::Overdefined, |value| {
                LatticeValue::Const(parse_literal_value(&value))
            }),
        Statement::AssignExpr {
            expr,
            command_binding,
            expr_base,
            ..
        } => {
            let original = driver.original_expression_parent(&stmt_ssa.statement);
            return driver.evaluate_assign_expr(
                expr,
                command_binding.as_ref(),
                original.as_ref().zip(*expr_base),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        Statement::ExprEval {
            expr,
            command_binding,
            expr_base,
            ..
        } => {
            let original = driver.original_expression_parent(&stmt_ssa.statement);
            return driver.evaluate_expr_eval(
                expr,
                command_binding,
                original.as_ref().zip(*expr_base),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        Statement::Call { .. } => {
            return driver.evaluate_call(stmt_ssa, values, ssa, &stmt_ssa.uses);
        }
        Statement::Incr {
            name,
            amount,
            amount_braced,
            ..
        } => {
            return driver.evaluate_incr(
                name,
                amount.as_deref().map(|text| (text, *amount_braced)),
                &crate::value_transfer::named_defs(stmt_ssa, ssa),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        _ => LatticeValue::Overdefined,
    };
    DefValues::Each(value, None)
}

/// Resolve `$var` / `${var}` to a lattice value by looking up the
/// SSA version in `uses` and indexing `values`. Returns None when
/// the text isn't a simple var reference.
pub(super) fn resolve_simple_var_ref<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> Option<LatticeValue> {
    let key = simple_var_ref_key(text, uses, ssa, config)?;
    Some(values.get(&key).cloned().unwrap_or(LatticeValue::Unknown))
}

/// The SSA value a `$var` / `${var}` word reads at this statement, or
/// `None` when the text isn't a simple var reference.
fn simple_var_ref_key<S: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> Option<ValueKey> {
    let name = tcl_syntax::naming::whole_scalar_variable_reference_name(text, config)?;
    let sym = ssa.var_symbol(name)?;
    let ver = *uses.get(&sym)?;
    Some((sym, ver))
}

/// Evaluate a branch condition.
///
/// The condition runs through the shared engine over the driver's services
/// under the effect-free nested policy, so a nested pure command resolves
/// (`if {[string length $acc] == 6}`), and with exactly one distinct finite
/// SSA value among its reads it runs once per member: every member true is
/// `Some(true)`, every member false `Some(false)`, and a mixed or undecided
/// answer is `None`. Two finite values decline as correlated. The reads
/// are the block's exit versions, and a variable the block never defines
/// — a parameter's caller-provided seed — reads its version 0. A `Raw`
/// operand reads a variable only as [`with_whole_variable_operands`] proves
/// it; any other `Raw` text leaves the condition undecided.
#[must_use]
pub(crate) fn evaluate_branch_under<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    fold: BranchFold<'_>,
    original: Option<(&crate::ir::CommandTokens, u32)>,
) -> Option<bool> {
    // A substituted condition word reads as the word written as the
    // expression, which agrees with C whenever its value is a number.
    let written = condition.substituted_word_source().map(|word| {
        tcl_syntax::expr::parser::parse_expr_with_grammar(&word, fold.driver.grammar())
    });
    let condition = written.as_ref().unwrap_or(condition);
    let mut uses: HashMap<Symbol, crate::ssa::Version> = ssa_block
        .exit_versions
        .iter()
        .map(|(&sym, &ver)| (sym, ver))
        .collect();
    let config = fold.driver.source_lexer_config();
    for name in condition.vars_element_qualified_with_config(config) {
        if let Some(sym) = ssa.var_symbol(&name) {
            uses.entry(sym).or_insert(0);
        }
    }
    let resolved = fold
        .driver
        .source_reference_lexer_config()
        .and_then(|config| with_whole_variable_operands(condition, config));
    fold.driver.evaluate_condition(
        resolved.as_ref().unwrap_or(condition),
        original,
        &uses,
        values,
        ssa,
    )
}

/// `condition` with every `Raw` *operand* the variable-name owner proves is
/// exactly one variable reference read as that variable — the flattened
/// dispatch's whole-variable subject, which
/// `switch_subject_operand` keeps `Raw` so codegen loads the name intact.
/// `None` when no operand resolves. The proof is
/// [`crate::value_transfer::whole_variable_operand`] under the complete
/// document configuration; a root `Raw` is an unparsed condition, not
/// an operand, and stays undecided, as does any `Raw` text the owner does
/// not prove.
pub(crate) fn with_whole_variable_operands(
    condition: &ExprNode,
    config: tcl_lexer::LexerConfig,
) -> Option<ExprNode> {
    let operand = |node: &ExprNode| match node {
        ExprNode::Raw { text } => {
            crate::value_transfer::whole_variable_operand(text, config).map(|name| ExprNode::Var {
                text: text.clone(),
                name: name.to_owned(),
                start: 0,
                end: 0,
            })
        }
        other => with_whole_variable_operands(other, config),
    };
    let or_same = |resolved: Option<ExprNode>, node: &ExprNode| {
        Box::new(resolved.unwrap_or_else(|| node.clone()))
    };
    match condition {
        ExprNode::Binary { op, left, right } => {
            let (l, r) = (operand(left), operand(right));
            (l.is_some() || r.is_some()).then(|| ExprNode::Binary {
                op: *op,
                left: or_same(l, left),
                right: or_same(r, right),
            })
        }
        ExprNode::Unary { op, operand: inner } => operand(inner).map(|inner| ExprNode::Unary {
            op: *op,
            operand: Box::new(inner),
        }),
        ExprNode::Ternary {
            condition: test,
            true_branch,
            false_branch,
        } => {
            let (c, t, f) = (operand(test), operand(true_branch), operand(false_branch));
            (c.is_some() || t.is_some() || f.is_some()).then(|| ExprNode::Ternary {
                condition: or_same(c, test),
                true_branch: or_same(t, true_branch),
                false_branch: or_same(f, false_branch),
            })
        }
        ExprNode::Call {
            function,
            args,
            start,
            end,
        } => {
            let resolved: Vec<Option<ExprNode>> = args.iter().map(operand).collect();
            resolved
                .iter()
                .any(Option::is_some)
                .then(|| ExprNode::Call {
                    function: function.clone(),
                    args: resolved
                        .into_iter()
                        .zip(args)
                        .map(|(resolved, arg)| resolved.unwrap_or_else(|| arg.clone()))
                        .collect(),
                    start: *start,
                    end: *end,
                })
        }
        _ => None,
    }
}

/// Extract iteration-variable elements from a foreach list arg
/// that is a literal (no `$` / `[` substitution).
///
/// `list_text` must already be delimiter-stripped by the segmenter (the shape
/// `Statement::Foreach`'s `list_arg` is built in, and the shape every caller
/// of this function must pass) — a second strip here would wrongly peel a
/// single-element list like `{{a b} {c d}}` (segmented to `{a b} {c d}`) down
/// to the elements `a` and `b}`. See `cfg_builder::list_literal_nonempty` and
/// `analyser::bounds_checks` for the same contract.
///
/// Behaviour:
/// - Strip whitespace.
/// - Split on Tcl list semantics (`Tcl_SplitList`), not ASCII whitespace.
/// - Returns `None` for anything that starts with `$` or `[` so
///   callers fall through to
///   [`resolve_foreach_list_via_lattice`].
#[must_use]
pub fn extract_foreach_elements(
    list_text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>> {
    let stripped = list_text.trim();
    if stripped.is_empty() {
        return Some(Vec::new());
    }
    if stripped.starts_with('[') || stripped.starts_with('$') {
        return None;
    }
    // List-aware split (Tcl_SplitList semantics): a nested-brace list like
    // `a {b c} d` yields the three elements `a`, `b c`, `d` — not the four
    // whitespace runs `a`, `{b`, `c}`, `d` a naive split would produce.
    Some(split_list_values(stripped, rules))
}

/// Resolve `$var` / `${var}` to a `Vec<String>` of list elements
/// via the SCCP lattice. Returns `None` when the operand is not a
/// simple var reference or its lattice value is not a
/// Const(String).
#[must_use]
pub fn resolve_foreach_list_via_lattice<'a, S1, S2>(
    list_text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    rules: tcl_syntax::word_rules::WordValueRules,
    config: impl Into<Option<tcl_lexer::LexerConfig>>,
) -> Option<Vec<String>>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    let stripped = list_text.trim();
    let name = tcl_syntax::naming::whole_scalar_variable_reference_name(stripped, config.into()?)?;
    let sym = ssa.into().symbol(name)?;
    let ver = uses.get(&sym).copied()?;
    match values.get(&(sym, ver))? {
        // The lattice value is the variable's runtime string; splitting it as a
        // `foreach` list uses Tcl list semantics (nested-brace aware), not a
        // whitespace split.
        LatticeValue::Const(ConstValue::String(s)) => Some(split_list_values(s.as_str(), rules)),
        _ => None,
    }
}

// AssignValue folding

/// Fold the RHS of an `AssignValue` statement to a lattice value.
///
/// Covers three tiers:
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)` of
///    the word's value, kept exactly: a value is a value, not a source token
///    to trim. The statement carries the word's spelling, so a word the
///    lowering marked `value_needs_backsubst` (a bare or quoted word with an
///    escape) is cooked under the document's escape grammar — `set s
///    "a\tb"` holds three characters — and a spelling that holds a
///    backslash the lowering did not account for has no known value.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → the resolved command's
///    declared route through the value-transfer driver
///    ([`crate::value_transfer`]), then — when the driver holds the trust
///    fact — the registry const-fold engine ([`BuiltinFoldInputs`]).
///
/// Anything else widens to `Overdefined`.
pub(super) fn fold_assign_value<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    needs_backsubst: bool,
    statement: &Statement,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> (LatticeValue, Option<crate::value_transfer::FoldedType>) {
    // Plain literal.
    if !value.contains('$') && !value.contains('[') {
        let cooked = if needs_backsubst {
            driver.literal_value(value, TokenType::Esc)
        } else if value.contains('\\') {
            None
        } else {
            Some(std::borrow::Cow::Borrowed(value))
        };
        let value = cooked.map_or(LatticeValue::Overdefined, |value| {
            LatticeValue::Const(parse_literal_value(&value))
        });
        return (value, None);
    }
    // Simple var reference: a copy shares its source's value, and the
    // source's folded type with it.
    if let Some(config) = driver.source_reference_lexer_config()
        && let Some(resolved) = resolve_simple_var_ref(value, uses, values, ssa, config)
    {
        let folded =
            simple_var_ref_key(value, uses, ssa, config).and_then(|key| driver.folded_of(key));
        return (resolved, folded);
    }
    // Command substitution.
    if let Some(folded) = driver.fold_cmd_subst(value, statement, uses, values, ssa) {
        return folded;
    }
    (LatticeValue::Overdefined, None)
}

/// The value ingress for a literal: the text kept exactly, classified as
/// an integer only when the canonical integer spelling round-trips
/// (`str(int(s)) == s`). A leading-zero literal such as `"08"` or `"010"`
/// parses as 8 / 10 but does not round-trip, so it stays a string —
/// preserving the identity SCCP needs to compare it correctly under each
/// dialect's leading-zero rule (octal in tcl8.x, decimal in tcl9.0);
/// `"+5"` and `"-0"` are kept as strings for the same reason. Nothing is
/// trimmed: `set p { again}` holds ` again`, and a rewrite that reads the
/// lattice sees the space (#2052). The classification rule is the
/// registry's ([`ExactValue::from_literal`]); this is its lattice
/// projection.
#[must_use]
pub fn parse_literal_value(text: &str) -> ConstValue {
    crate::value_transfer::exact_to_const(&ExactValue::from_literal(text))
        .unwrap_or_else(|| ConstValue::String(text.into()))
}
