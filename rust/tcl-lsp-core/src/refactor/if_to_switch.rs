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

//! Convert an `if`/`elseif` equality chain to a `switch` statement.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::ArgRole;

use super::datagroup::{ScalarVariableSourceSyntax, scalar_variable_source_syntax};

use super::source_rewrite::{RewriteObligation, select};
use super::{RefactorEdit, Refactoring, reindent_body};
use crate::code_actions::ActionKind;

/// Parsed equality condition: `(var_name, value, negated)`.
struct EqTest {
    subject: ScalarVariableSourceSyntax,
    value: String,
    negated: bool,
}

/// Parse an equality condition `$var eq "value"` / `"value" eq $var`
/// (and the `==` / `ne` / `!=` operators).
fn parse_eq_test(condition: &str, config: LexerConfig) -> Option<EqTest> {
    let mut cond = condition.trim();

    // Strip outer braces: `{ $x eq "foo" }`.
    if cond.starts_with('{') && cond.ends_with('}') && cond.len() >= 2 {
        cond = cond[1..cond.len() - 1].trim();
    }

    // Negation: `!($x eq "foo")` or `!{$x eq "foo"}`.
    let mut negated = false;
    if let Some(rest) = cond.strip_prefix('!') {
        let inner = rest.trim();
        if inner.len() >= 2
            && ((inner.starts_with('(') && inner.ends_with(')'))
                || (inner.starts_with('{') && inner.ends_with('}')))
        {
            cond = inner[1..inner.len() - 1].trim();
            negated = true;
        }
    }

    if let Some((var, op, value)) = split_var_op_value(cond, config) {
        let is_ne = op == "ne" || op == "!=";
        return Some(EqTest {
            subject: var,
            value: value.trim().to_owned(),
            negated: negated ^ is_ne,
        });
    }
    if let Some((value, op, var)) = split_value_op_var(cond, config) {
        let is_ne = op == "ne" || op == "!=";
        return Some(EqTest {
            subject: var,
            value: value.trim().to_owned(),
            negated: negated ^ is_ne,
        });
    }
    None
}

/// Recognised equality operators, longest-first so `==` wins over a bare
/// fragment.
// registry-axis-ok: irreducible — `eq` / `ne` are expr comparison-operator
// spellings (`tcl_syntax::expr::operators`'s own vocabulary), read here as
// parsed condition text; they coincide with `::tcl::mathop::eq` / `ne`'s
// bare registration only by spelling; until never
const OPS: &[&str] = &["==", "!=", "eq", "ne"];

/// Split `$var OP value` / `"$var" OP value` → `(var, op, value)`.
fn split_var_op_value(
    cond: &str,
    config: LexerConfig,
) -> Option<(ScalarVariableSourceSyntax, String, String)> {
    for op in OPS {
        let needle = format!(" {op} ");
        if let Some(pos) = cond.find(&needle) {
            let lhs = cond[..pos].trim();
            let value = cond[pos + needle.len()..].to_owned();
            if let Some(var) = scalar_variable_source_syntax(lhs, config) {
                return Some((var, (*op).to_owned(), value));
            }
        }
    }
    None
}

/// Split `value OP $var` / `value OP "$var"` → `(value, op, var)`.
fn split_value_op_var(
    cond: &str,
    config: LexerConfig,
) -> Option<(String, String, ScalarVariableSourceSyntax)> {
    for op in OPS {
        let needle = format!(" {op} ");
        // Use the *last* occurrence so a value containing the operator
        // text doesn't mis-split (the var is anchored on the right with
        // `$`).
        if let Some(pos) = cond.rfind(&needle) {
            let value = cond[..pos].trim().to_owned();
            let rhs = cond[pos + needle.len()..].trim();
            if let Some(var) = scalar_variable_source_syntax(rhs, config) {
                return Some((value, (*op).to_owned(), var));
            }
        }
    }
    None
}

/// Remove surrounding double quotes if present.
fn strip_quotes(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Render a switch pattern, preserving whitespace literals by bracing.
fn render_switch_pattern(value: &str) -> String {
    if value.is_empty() {
        return "{}".to_owned();
    }
    if value.chars().any(char::is_whitespace) {
        format!("{{{}}}", value.replace('}', "\\}"))
    } else {
        value.to_owned()
    }
}

/// Convert an `if`/`elseif` chain at byte offset `cursor` to a `switch`.
///
/// The complete current analysis supplies Registry and source grammar axes.
/// Changing branch evaluation requires its own independent permission.
#[must_use]
pub fn if_to_switch(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let (cmd, obligation) = select(
        source,
        cursor,
        analysis,
        tcl_registry::hooks::LoweringHookId::If,
        RewriteObligation::ControlFlowEvaluation,
    )?;
    if let Some(obligation) = obligation {
        return Some(obligation.refusal("Convert if chain to switch"));
    }
    let config = analysis.body_lexer_config?;
    let texts = &cmd.texts;
    if texts.len() < 3 {
        return None;
    }

    // Walk the chain through `if`'s own clause grammar — the condition and
    // body of each `if` / `elseif` clause, and the default (`else`, or its
    // optional-keyword bare final body) — rather than comparing keyword
    // spellings by hand; a structural defect (a stray word, a chain the
    // grammar cannot parse) declines the conversion.
    let args: Vec<&str> = texts[1..].iter().map(String::as_str).collect();
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let original = crate::original_invocation::source_registry_words(source, analysis, &cmd)?;
    let plan = original.with_source_schema(&context, |schema| schema.clause_plan())??;
    if plan.defect.is_some() {
        return None;
    }

    // Parse the if/elseif chain: `if cond body ?elseif cond body?... ?else body?`.
    let mut branches: Vec<(String, String)> = Vec::new(); // (value, body)
    let mut else_body: Option<String> = None;
    let mut target_var: Option<ScalarVariableSourceSyntax> = None;

    for clause in &plan.clauses {
        let body = args[clause.operand(ArgRole::Body)?].to_owned();
        if clause.is_default {
            else_body = Some(body);
            continue;
        }
        let condition = args[clause.operand(ArgRole::Expr)?];

        let parsed = parse_eq_test(condition, config)?;
        match &target_var {
            None => target_var = Some(parsed.subject.clone()),
            Some(v) if v.name() != parsed.subject.name() => return None,
            _ => {}
        }
        // Negated (ne / !=) conditions make switch conversion awkward.
        if parsed.negated {
            return None;
        }
        let value = strip_quotes(&parsed.value);
        // The emitted `switch -exact -- $x { … }` uses the braced case-list
        // form, whose patterns are LITERAL. A RHS that carried a substitution
        // in the original `eq` comparison (`$y`, `[cmd]`, a `\`-escape) cannot
        // be reproduced as a literal pattern — the arm would never match — so
        // decline the conversion.
        if value.bytes().any(|b| matches!(b, b'$' | b'[' | b'\\')) {
            return None;
        }
        branches.push((value.to_owned(), body));
    }

    let target_var = target_var?;
    if branches.len() < 2 {
        return None;
    }

    // Build the switch statement.
    let lines: Vec<&str> = source.split('\n').collect();
    let cmd_line = line_index.line_at(cmd.span.start());
    let indent = lines
        .get(cmd_line as usize)
        .map_or("", |l| super::line_indent(l));
    let replacement = render_switch(&target_var, &branches, else_body.as_deref(), indent, config)?;

    let (start, end) = super::command_span_offsets(source, &cmd);
    Some(Refactoring {
        title: format!("Convert to switch on {}", target_var.reference()),
        edits: vec![RefactorEdit {
            start,
            end,
            new_text: replacement,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
        disabled: None,
    })
}

fn render_switch(
    subject: &ScalarVariableSourceSyntax,
    branches: &[(String, String)],
    else_body: Option<&str>,
    indent: &str,
    config: LexerConfig,
) -> Option<String> {
    let inner = format!("{indent}    ");

    let mut parts: Vec<String> = vec![format!("switch -exact -- {} {{", subject.reference())];
    for (value, body) in branches {
        let body_trimmed = reindent_body(body, &format!("{inner}    "));
        parts.push(format!("{inner}{} {{", render_switch_pattern(value)));
        parts.push(body_trimmed);
        parts.push(format!("{inner}}}"));
    }
    if let Some(eb) = else_body {
        let body_trimmed = reindent_body(eb, &format!("{inner}    "));
        parts.push(format!("{inner}default {{"));
        parts.push(body_trimmed);
        parts.push(format!("{inner}}}"));
    }
    parts.push(format!("{indent}}}"));
    let replacement = parts.join("\n");

    let generated = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(&replacement),
        tcl_lexer::Span::new(0, u32::try_from(replacement.len()).ok()?),
        config,
    )
    .ok()?;
    (generated.fatal_tail.is_none() && generated.commands.len() == 1).then_some(replacement)
}

#[cfg(test)]
mod tests {

    #[test]
    fn selected_if_switch_scalar_syntax_preserves_reference_and_edit_permission() {
        // Implementation contract: naming.refactor.selected-if-scalar-source-syntax
        // docs/design/analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md
        for subject in [
            "${a b}",
            "\"${café}\"",
            "${literal$name}",
            "${a(k)tail}",
            r"${a\b}",
            "$café",
        ] {
            let source = format!(
                "if {{{subject} eq \"a\"}} {{puts one}} elseif {{{subject} eq \"b\"}} {{puts two}}"
            );
            let action = run(&source, 0).expect("explicit Logical scalar source rewrite");
            assert!(action.disabled.is_none());
            assert!(
                action
                    .apply(&source)
                    .starts_with(&format!("switch -exact -- {subject} {{"))
            );
        }
        let config = LexerConfig::for_dialect("tcl8.6");
        for subject in [
            "$a(k)",
            "${a(k)}",
            "$café",
            "{$x}",
            "\"$x[set y]\"",
            "${missing",
        ] {
            assert!(
                parse_eq_test(&format!("{subject} eq one"), config).is_none(),
                "{subject:?}"
            );
        }
        let source = "if {${a b} eq \"a\"} {puts one} elseif {${a b} eq \"b\"} {puts two}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let action = if_to_switch(source, 0, &analysis, &LineIndex::new(source))
            .expect("source advice refuses edit");
        assert!(
            action
                .disabled
                .as_deref()
                .unwrap()
                .contains("missing-control-flow-equivalence")
        );
        assert!(action.edits.is_empty());
    }

    use super::*;

    fn run(source: &str, cursor: u32) -> Option<Refactoring> {
        let reg = super::super::test_registry();
        let analysis = super::super::source_rewrite::lexical_analysis(source, &reg);
        let li = LineIndex::new(source);
        if_to_switch(source, cursor, &analysis, &li)
    }

    #[test]
    fn simple_eq_chain() {
        let source = "if {$x eq \"a\"} {\n    puts \"alpha\"\n} elseif {$x eq \"b\"} {\n    puts \"beta\"\n} elseif {$x eq \"c\"} {\n    puts \"gamma\"\n}";
        let r = run(source, 0).expect("result");
        assert!(r.title.to_lowercase().contains("switch"));
        let applied = r.apply(source);
        assert!(applied.contains("switch -exact -- $x"), "{applied:?}");
    }

    #[test]
    fn with_else_clause() {
        let source = "if {$cmd eq \"GET\"} {\n    handle_get\n} elseif {$cmd eq \"POST\"} {\n    handle_post\n} else {\n    handle_other\n}";
        let r = run(source, 0).expect("result");
        assert!(r.apply(source).contains("default"));
    }

    #[test]
    fn dynamic_rhs_pattern_returns_none() {
        // `$x eq $y` — the RHS `$y` can't be a literal braced
        // switch pattern (it would never match), so decline the conversion.
        let source = "if {$x eq $y} {\n    puts \"a\"\n} elseif {$x eq $z} {\n    puts \"b\"\n}";
        assert!(run(source, 0).is_none());
        // A `[cmd]` RHS is equally unusable as a literal pattern.
        let source2 =
            "if {$x eq [f]} {\n    puts \"a\"\n} elseif {$x eq \"b\"} {\n    puts \"b\"\n}";
        assert!(run(source2, 0).is_none());
        // FP-guard: literal string RHS values still convert.
        let source3 =
            "if {$x eq \"a\"} {\n    puts \"a\"\n} elseif {$x eq \"b\"} {\n    puts \"b\"\n}";
        assert!(run(source3, 0).is_some());
    }

    #[test]
    fn different_vars_returns_none() {
        let source =
            "if {$x eq \"a\"} {\n    puts \"x\"\n} elseif {$y eq \"b\"} {\n    puts \"y\"\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn single_branch_returns_none() {
        assert!(run("if {$x eq \"a\"} { puts \"alpha\" }", 0).is_none());
    }

    #[test]
    fn ne_operator_returns_none() {
        let source = "if {$x ne \"a\"} {\n    puts \"not a\"\n} elseif {$x ne \"b\"} {\n    puts \"not b\"\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn rewrite_covers_entire_if_command() {
        let source = "if {$x eq \"a\"} {\n    puts 1\n} elseif {$x eq \"b\"} {\n    puts 2\n}";
        let applied = run(source, 0).expect("result").apply(source);
        assert_eq!(
            applied,
            "switch -exact -- $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}"
        );
        let trimmed = applied.trim();
        assert!(trimmed.ends_with('}'));
        assert!(!trimmed.ends_with("}}"), "{trimmed:?}");
    }

    #[test]
    fn values_with_spaces_are_braced() {
        let source =
            "if {$x eq \"a b\"} {\n    puts one\n} elseif {$x eq \"c d\"} {\n    puts two\n}";
        let applied = run(source, 0).expect("result").apply(source);
        assert!(applied.contains("{a b}"), "{applied:?}");
        assert!(applied.contains("{c d}"), "{applied:?}");
    }

    #[test]
    fn inside_proc_body() {
        let source = "proc handler {method} {\n    if {$method eq \"GET\"} {\n        handle_get\n    } elseif {$method eq \"POST\"} {\n        handle_post\n    } elseif {$method eq \"PUT\"} {\n        handle_put\n    }\n}";
        // Cursor at line 1, col 4 — inside the `if`.
        let cursor = u32::try_from(source.find("if {").unwrap()).unwrap();
        let r = run(source, cursor).expect("nested result");
        assert!(r.title.to_lowercase().contains("switch"));
    }

    /// The same conversion must fire inside an `apply` lambda
    /// body.  `apply`'s literal is `ArgRole::LambdaLiteral`, so
    /// `find_command_at` has to split it and descend into element 1;
    /// reading the `{m}` argument list as a command name finds no `if` at
    /// all, silently disabling this and the four other
    /// `body_words`-backed code actions there.
    #[test]
    fn inside_apply_lambda_body() {
        let source = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            handle_get\n        } elseif {$m eq \"POST\"} {\n            handle_post\n        } elseif {$m eq \"PUT\"} {\n            handle_put\n        }\n    }} $x\n}\n";
        let cursor = u32::try_from(source.find("if {$m").unwrap()).unwrap();
        let r = run(source, cursor).expect("result inside the apply lambda");
        assert!(r.title.to_lowercase().contains("switch"));
        let applied = r.apply(source);
        assert!(applied.contains("switch -exact -- $m"), "{applied:?}");
        // The rewrite lands inside the lambda body, leaving `apply`'s own
        // argument list and the enclosing proc intact.
        assert!(applied.contains("apply {{m} {"), "{applied:?}");
        assert!(applied.starts_with("proc handler {} {\n"), "{applied:?}");
    }

    /// This refactor walks whatever clause plan the registry resolves for
    /// `if` — never a hardcoded Rust-side `if`/`elseif`/`else` keyword
    /// table — so a pack overlay that redeclares `if`'s own grammar (a
    /// registry fact a `.tclspec` pack states exactly the same way) is
    /// still read correctly. Negative: a pack overlay for `if` with no
    /// clause grammar at all has nothing for `clause_plan` to walk, so the
    /// conversion declines rather than guessing.
    #[test]
    fn if_to_switch_reads_the_clause_plan() {
        use tcl_registry::{
            Arity, ClauseGrammarSpec, ClauseRow, ClauseSelection, ClauseSlot, ClauseTiming,
            CommandSpec,
        };

        const COND_CLAUSE: &[ClauseSlot] =
            &[ClauseSlot::of(ArgRole::Expr), ClauseSlot::of(ArgRole::Body)];
        // Deliberately not the shipped grammar's shape (no `then` noise
        // word, no `else` tail) — a pack's own if-shaped grammar, not a
        // copy of `if`'s real one.
        const PACK_GRAMMAR: ClauseGrammarSpec = ClauseGrammarSpec {
            head: ClauseRow::head(COND_CLAUSE, ClauseTiming::Selected),
            rows: &[ClauseRow::repeated(
                "elseif",
                COND_CLAUSE,
                ClauseTiming::Selected,
            )],
            tail: None,
            fallthrough_body: None,
            default_clause: None,
            selection: ClauseSelection::FirstMatch,
            surface: None,
        };

        let source = "if {$x eq \"a\"} {\n    puts \"alpha\"\n} elseif {$x eq \"b\"} {\n    puts \"beta\"\n}";
        let li = LineIndex::new(source);

        let mut with_grammar = super::super::test_registry();
        with_grammar.insert(CommandSpec {
            name: "if",
            arity: Arity::at_least(2),
            arg_role_resolver_roles: &[ArgRole::Expr, ArgRole::Body],
            clause_grammar: Some(&PACK_GRAMMAR),
            ..CommandSpec::DEFAULT
        });
        let r = if_to_switch(source, 0, &with_grammar, &li, LexerConfig::default());
        assert!(
            r.is_some(),
            "a pack-declared if-shaped grammar still converts"
        );

        let mut without_grammar = super::super::test_registry();
        without_grammar.insert(CommandSpec {
            name: "if",
            arity: Arity::at_least(2),
            clause_grammar: None,
            ..CommandSpec::DEFAULT
        });
        assert!(
            if_to_switch(source, 0, &without_grammar, &li, LexerConfig::default()).is_none(),
            "a command without a clause grammar is never converted"
        );
    }
}
