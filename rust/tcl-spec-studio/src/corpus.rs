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

//! The corpus-derived shape heuristics: what a proc's **body** says about the
//! interface a caller sees.
//!
//! [`crate::infer`] reads a procedure's *signature* — the parameter list and
//! the analyser's per-parameter traits. That misses everything a Tcl library
//! expresses in the body instead, which is most of what a spec has to say:
//! `docs/design/spec-dsl-examples/external/` catalogues the recurring shapes,
//! and this module recovers the ones that can be recovered **deterministically**
//! — no model, no guess about intent, and every conclusion carrying the line of
//! evidence that produced it:
//!
//! - **Option tables** from a `-*` dispatch — a `switch` over an option word,
//!   or an `if`/`elseif` chain comparing one against `-…` literals. Whether an
//!   option consumes a value is read off the arm: an arm that steps the
//!   argument cursor (`incr i`, `lindex $args`, `lassign`, `lshift`) takes one.
//! - **`--` handling**, when an arm or a comparison names it.
//! - **Mode words that select tails**, which become subcommands: a dispatch on
//!   the *first* parameter of a variadic proc is a subcommand table, not a
//!   value set.
//! - **Closed value sets**, from the same dispatch when it is *not* a
//!   subcommand table and has no `default` arm — the accepted values are
//!   exactly the arms.
//! - **Callbacks with their observed appended arity**: a parameter invoked as
//!   a command head, with the number of words the body appends to it.
//!
//! What is deliberately **not** here: anything needing intent. Taint, side
//! effects, versions, and prose stay for the human (or the AI tab) — the
//! division of labour the design draws.
//!
//! ## How the body is read
//!
//! Statements come from the same segmenter the compiler uses, and the walk
//! descends into a word **only when the registry says that word is a script**
//! ([`ArgRole::Body`]) — so `foreach`, `while`, `if`, `catch` and a pack's own
//! body-taking commands are all followed without a single command name written
//! here. A `switch`'s arm list is the one shape read specially, because it is a
//! pattern/script *list* rather than a script.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_lexer::{LexerConfig, SourceMap};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::registry::CommandRegistry;

use crate::draft::{self, Draft};

/// How deep the body walk follows nested scripts.
const MAX_DEPTH: usize = 16;

// What a body can be made to admit

/// One option the body dispatches on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionShape {
    /// The option word, as written (`-loud`).
    pub name: String,
    /// Whether the arm consumes a following word.
    pub takes_value: bool,
    /// The evidence line for this row.
    pub evidence: String,
}

/// A parameter whose accepted values the body encloses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedValues {
    /// Parameter index, 0-based after the command name.
    pub index: usize,
    /// The accepted words, in the order the dispatch lists them.
    pub values: Vec<String>,
}

/// How many words the body appends to a callback when it invokes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appended {
    /// Every observed call site appended the same number.
    Exactly(u8),
    /// Call sites disagreed; the smallest is the guarantee.
    AtLeast(u8),
}

/// A parameter the body invokes as a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Callback {
    /// Parameter index, 0-based after the command name.
    pub index: usize,
    /// The observed appended arity.
    pub appended: Appended,
}

/// Everything the body admitted, with the evidence for each conclusion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// Options recovered from a `-*` dispatch.
    pub options: Vec<OptionShape>,
    /// Whether the body recognises `--` as the end of options.
    pub option_terminator: bool,
    /// Mode words that select tails — subcommands.
    pub subcommands: Vec<String>,
    /// A parameter whose value set the dispatch closes.
    pub closed: Option<ClosedValues>,
    /// Callbacks and their appended arity.
    pub callbacks: Vec<Callback>,
    /// One line per conclusion, in the order they were reached.
    pub evidence: Vec<String>,
}

impl Shape {
    /// Whether the body said anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
            && self.subcommands.is_empty()
            && self.closed.is_none()
            && self.callbacks.is_empty()
    }

    /// Write these findings into `d`, appending each evidence line to `notes`.
    ///
    /// Nothing here overwrites a field the signature pass already filled with
    /// something stronger: roles and traits are the analyser's, and only the
    /// fields the body is the sole witness for are set.
    pub fn apply(&self, d: &mut Draft, notes: &mut Vec<String>) {
        notes.extend(self.evidence.iter().cloned());

        if !self.options.is_empty() {
            let rows: Vec<Value> = self.options.iter().map(option_row).collect();
            d.insert("options".into(), Value::Array(rows));
        }
        if !self.subcommands.is_empty() {
            let rows: Vec<Value> = self
                .subcommands
                .iter()
                .map(|name| {
                    let mut sub = draft::default_subcommand_draft();
                    sub.insert("name".into(), json!(name));
                    sub.insert(
                        "detail".into(),
                        json!("dispatched by the body's `switch` on its first argument"),
                    );
                    Value::Object(sub)
                })
                .collect();
            d.insert("subcommands".into(), Value::Array(rows));
        }
        if let Some(closed) = &self.closed {
            d.insert(
                "arg_values".into(),
                json!([{
                    "index": closed.index,
                    "values": closed
                        .values
                        .iter()
                        .map(|v| json!({
                            "value": v,
                            "detail": "",
                            "min_tcl": Value::Null,
                            "code": Value::Null,
                        }))
                        .collect::<Vec<Value>>(),
                }]),
            );
            d.insert("closed_value_args".into(), json!([closed.index]));
        }
        if !self.callbacks.is_empty() {
            d.insert(
                "command_prefixes".into(),
                Value::Array(
                    self.callbacks
                        .iter()
                        .map(|callback| {
                            json!({
                                "index": callback.index,
                                "arity": match callback.appended {
                                    Appended::Exactly(n) => json!({ "kind": "Exactly", "n": n }),
                                    Appended::AtLeast(n) => json!({ "kind": "AtLeast", "n": n }),
                                },
                            })
                        })
                        .collect(),
                ),
            );
        }
    }
}

/// One option row in the draft's own shape, as a flag or a value option.
fn option_row(option: &OptionShape) -> Value {
    let value = if option.takes_value {
        json!({
            "arity": { "kind": "One" },
            "role": "Value",
            "also_role": Value::Null,
            "body_kind": "Plain",
            "values": [],
            "closed": false,
            "integer": Value::Null,
            "hint": "value",
            "appended_arity": { "kind": "Unknown" },
            "taints_var_write": false,
        })
    } else {
        Value::Null
    };
    json!({
        "name": option.name,
        "detail": option.evidence,
        "surface": Value::Null,
        "aliases": [],
        "introduced_version": Value::Null,
        "deprecated_version": Value::Null,
        "retired_version": Value::Null,
        "deprecation_fix": Value::Null,
        "min_abbrev": Value::Null,
        "value": value,
    })
}

// Reading the body

/// One statement of the body, flattened to its word texts.
struct Stmt {
    words: Vec<String>,
}

impl Stmt {
    fn head(&self) -> &str {
        self.words.first().map_or("", String::as_str)
    }
    fn word(&self, index: usize) -> &str {
        self.words.get(index).map_or("", String::as_str)
    }
}

/// Every statement in `source`, following script arguments as the registry
/// declares them.
fn statements(
    registry: &CommandRegistry,
    config: LexerConfig,
    source: &str,
    depth: usize,
    out: &mut Vec<Stmt>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(source, config);
    for segment in segments_from_document(document, &source_map) {
        let words = segment.texts.clone();
        let head = words.first().cloned().unwrap_or_default();
        let args: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
        for index in registry.arg_indices_for_role(&head, &args, ArgRole::Body) {
            if let Some(body) = args.get(index) {
                statements(registry, config, body, depth + 1, out);
            }
        }
        out.push(Stmt { words });
    }
}

/// One arm of a `switch`: its pattern and the script it runs.
struct Arm {
    pattern: String,
    script: String,
}

/// The arms of a braced `switch` body.
///
/// The list is `pattern script pattern script …`, one pair per line by
/// convention — which is exactly what the segmenter reports as a "command" of
/// two words, so the same parse serves.
fn arms(body: &str, config: LexerConfig) -> Vec<Arm> {
    let source_map = SourceMap::new(body);
    let (document, _warnings) = build_document(body, config);
    let mut out: Vec<Arm> = Vec::new();
    for segment in segments_from_document(document, &source_map) {
        let mut words = segment.texts.iter();
        while let Some(pattern) = words.next() {
            let script = words.next().cloned().unwrap_or_default();
            out.push(Arm {
                pattern: pattern.clone(),
                script,
            });
        }
    }
    // A `-` script is Tcl's fall-through: the arm runs the *next* arm's body.
    for i in 0..out.len() {
        if out[i].script.trim() == "-" {
            let next = out
                .iter()
                .skip(i + 1)
                .find(|a| a.script.trim() != "-")
                .map(|a| a.script.clone())
                .unwrap_or_default();
            out[i].script = next;
        }
    }
    out
}

/// The variable a `switch` dispatches on, if it dispatches on one.
///
/// `switch $x { … }`, `switch -- $x { … }`, `switch -exact -- $x { … }` — in
/// every form the string word is the second to last.
fn switched_variable(stmt: &Stmt) -> Option<String> {
    if stmt.head() != "switch" || stmt.words.len() < 3 {
        return None;
    }
    let word = stmt.word(stmt.words.len() - 2);
    variable_name(word)
}

/// `$name` / `${name}` → `name`; anything else → `None`.
fn variable_name(word: &str) -> Option<String> {
    let rest = word.strip_prefix('$')?;
    let name = rest
        .strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .unwrap_or(rest);
    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Some(name.to_owned())
    } else {
        None
    }
}

/// Whether a pattern is a literal word rather than a glob.
fn is_literal(pattern: &str) -> bool {
    !pattern.is_empty()
        && pattern != "default"
        && !pattern.contains(['*', '?', '[', ']', '$'])
        && !pattern.contains(char::is_whitespace)
}

/// Whether a word reads as an option name — `-` then a letter, so `-1` and a
/// bare `-` (subtraction, or a fall-through arm) are never mistaken for one.
fn is_option_word(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next() == Some('-') && chars.next().is_some_and(char::is_alphabetic)
}

/// Whether an arm's script consumes a word from the argument list.
///
/// The four shapes a hand-written option loop uses to step its cursor. This is
/// the one judgement in the module, and it is reported as such: the evidence
/// line says *why* the option was called a value option.
fn consumes_a_value(script: &str) -> bool {
    ["incr ", "lindex ", "lassign ", "lshift", "lpop", "lrange "]
        .iter()
        .any(|needle| script.contains(needle))
}

// The scan

/// Read `body` — the text between a proc's braces — for the shapes above.
///
/// `params` is the proc's parameter list, so a dispatch can be tied back to
/// the argument position it constrains. `dialect` selects the registry whose
/// script-argument declarations drive the walk.
#[must_use]
pub fn scan(body: &str, params: &[ParamDef], dialect: &str) -> Shape {
    let registry = crate::environment::store_for_dialect(dialect);
    // The body is the authored pack's own Tcl, so it is read under the
    // grammar of the environment the pack is being authored for.
    let config = LexerConfig::for_profile(Some(crate::environment::profile_for_dialect(dialect)));
    let mut stmts = Vec::new();
    statements(registry, config, body, 0, &mut stmts);

    let mut shape = Shape::default();
    options_from_dispatch(&stmts, config, &mut shape);
    values_from_dispatch(&stmts, params, config, &mut shape);
    callbacks(&stmts, params, &mut shape);
    shape
}

/// Options from a `switch` over option words, or an `if` chain comparing one.
fn options_from_dispatch(stmts: &[Stmt], config: LexerConfig, shape: &mut Shape) {
    // Preserve first-seen order, and let a later site upgrade a flag to a
    // value option rather than contradict it.
    let mut found: Vec<OptionShape> = Vec::new();
    let mut add = |name: String, takes_value: bool, evidence: String| {
        if let Some(existing) = found.iter_mut().find(|o| o.name == name) {
            if takes_value && !existing.takes_value {
                existing.takes_value = true;
                existing.evidence = evidence;
            }
            return;
        }
        found.push(OptionShape {
            name,
            takes_value,
            evidence,
        });
    };

    let mut terminator = false;
    for stmt in stmts {
        if stmt.head() == "switch" {
            let Some(last) = stmt.words.last() else {
                continue;
            };
            for arm in arms(last, config) {
                if arm.pattern == "--" {
                    terminator = true;
                    continue;
                }
                if !is_option_word(&arm.pattern) {
                    continue;
                }
                let takes_value = consumes_a_value(&arm.script);
                add(
                    arm.pattern.clone(),
                    takes_value,
                    if takes_value {
                        format!(
                            "`{}` is a `switch` arm whose body reads a following word — a value option",
                            arm.pattern
                        )
                    } else {
                        format!(
                            "`{}` is a `switch` arm that reads no following word — a flag",
                            arm.pattern
                        )
                    },
                );
            }
        }
        if stmt.head() == "if" || stmt.head() == "elseif" {
            let condition = stmt.word(1);
            if !condition.contains('$') {
                continue;
            }
            for word in condition.split_whitespace() {
                let word = word.trim_matches(|c| c == '"' || c == '{' || c == '}' || c == ')');
                if word == "--" {
                    terminator = true;
                } else if is_option_word(word) {
                    add(
                        word.to_owned(),
                        false,
                        format!("`{word}` is compared against an argument in an `if` condition"),
                    );
                }
            }
        }
    }

    if found.is_empty() && !terminator {
        return;
    }
    if !found.is_empty() {
        shape.evidence.push(format!(
            "the body dispatches on {} option word(s): {}",
            found.len(),
            found
                .iter()
                .map(|o| o.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        for option in &found {
            shape.evidence.push(option.evidence.clone());
        }
    }
    if terminator {
        shape.evidence.push(
            "the body handles `--` as the end of options — check the spec's option terminator"
                .to_owned(),
        );
    }
    shape.options = found;
    shape.option_terminator = terminator;
}

/// Mode words and closed value sets from a dispatch on a parameter.
///
/// The two are the same evidence read against the parameter list: a dispatch
/// on the **first** parameter of a proc whose tail is variadic selects *tails*
/// — that is a subcommand table. Anything else closes a value set.
fn values_from_dispatch(stmts: &[Stmt], params: &[ParamDef], config: LexerConfig, shape: &mut Shape) {
    let positions: BTreeMap<&str, usize> = params
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    let variadic = params.last().is_some_and(|p| p.name == "args");

    for stmt in stmts {
        let Some(variable) = switched_variable(stmt) else {
            continue;
        };
        let Some(&index) = positions.get(variable.as_str()) else {
            continue;
        };
        let Some(last) = stmt.words.last() else {
            continue;
        };
        let all = arms(last, config);
        let has_default = all.iter().any(|a| a.pattern == "default");
        let literals: Vec<String> = all
            .iter()
            .map(|a| a.pattern.clone())
            .filter(|p| is_literal(p) && !is_option_word(p))
            .collect();
        if literals.len() < 2 {
            continue;
        }

        if index == 0 && variadic {
            shape.evidence.push(format!(
                "`switch ${variable}` on the first parameter of a variadic proc selects a tail per \
                 word — {} of them ({}), so they are read as subcommands",
                literals.len(),
                literals.join(", ")
            ));
            shape.subcommands = literals;
            return;
        }
        if has_default {
            shape.evidence.push(format!(
                "`switch ${variable}` has a `default` arm, so its {} listed value(s) are not a \
                 closed set",
                literals.len()
            ));
            return;
        }
        shape.evidence.push(format!(
            "`switch ${variable}` has no `default` arm, so argument {} accepts exactly {}",
            index + 1,
            literals.join(", ")
        ));
        shape.closed = Some(ClosedValues {
            index,
            values: literals,
        });
        return;
    }
}

/// Callbacks: a parameter used as a command head, and how many words the body
/// appends to it.
fn callbacks(stmts: &[Stmt], params: &[ParamDef], shape: &mut Shape) {
    let positions: BTreeMap<&str, usize> = params
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    // parameter index → every observed appended count.
    let mut seen: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for stmt in stmts {
        let Some(name) = variable_name(stmt.head()) else {
            continue;
        };
        let Some(&index) = positions.get(name.as_str()) else {
            continue;
        };
        seen.entry(index).or_default().push(stmt.words.len() - 1);
    }
    for (index, counts) in seen {
        let min = counts.iter().copied().min().unwrap_or(0);
        let max = counts.iter().copied().max().unwrap_or(0);
        let Ok(min_u8) = u8::try_from(min) else {
            continue;
        };
        let appended = if min == max {
            Appended::Exactly(min_u8)
        } else {
            Appended::AtLeast(min_u8)
        };
        let name = params.get(index).map_or("", |p| p.name.as_str()).to_owned();
        shape.evidence.push(if min == max {
            format!(
                "`${name}` is invoked as a command with {min} word(s) appended, at {} call site(s)",
                counts.len()
            )
        } else {
            format!(
                "`${name}` is invoked as a command with {min}..{max} words appended across {} call \
                 site(s) — the guarantee is the smallest",
                counts.len()
            )
        });
        shape.callbacks.push(Callback { index, appended });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str) -> ParamDef {
        ParamDef {
            name: name.to_owned(),
            has_default: false,
            default_value: None,
        }
    }

    #[test]
    fn an_option_loop_becomes_an_option_table() {
        let body = r#"
            foreach opt $args {
                switch -- $opt {
                    -loud { set loud 1 }
                    -times { set n [lindex $args [incr i]] }
                    default { error "bad option $opt" }
                }
            }
        "#;
        let shape = scan(body, &[param("args")], "tcl9.0");
        let names: Vec<&str> = shape.options.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["-loud", "-times"]);
        assert!(!shape.options[0].takes_value, "{:?}", shape.options[0]);
        assert!(shape.options[1].takes_value, "{:?}", shape.options[1]);
        assert!(
            shape.evidence.iter().any(|e| e.contains("-times")),
            "{:?}",
            shape.evidence
        );
    }

    #[test]
    fn the_option_terminator_is_noticed() {
        let body = r"
            foreach opt $args {
                switch -- $opt {
                    -- { break }
                    -x { set x 1 }
                }
            }
        ";
        let shape = scan(body, &[param("args")], "tcl9.0");
        assert!(shape.option_terminator);
        assert_eq!(
            shape
                .options
                .iter()
                .map(|o| o.name.as_str())
                .collect::<Vec<_>>(),
            vec!["-x"],
            "`--` is the terminator, not an option"
        );
    }

    #[test]
    fn a_fall_through_arm_takes_the_next_arms_body() {
        let body = r"
            switch -- $opt {
                -q -
                -quiet { set verbose 0 }
                -n { set n [lindex $args 1] }
            }
        ";
        let shape = scan(body, &[param("opt"), param("args")], "tcl9.0");
        let names: Vec<&str> = shape.options.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["-q", "-quiet", "-n"]);
        assert!(!shape.options[0].takes_value, "a fall-through flag");
    }

    #[test]
    fn a_mode_word_dispatch_on_a_variadic_first_parameter_becomes_subcommands() {
        let body = r"
            switch -- $mode {
                add { return [addit $args] }
                remove { return [rm $args] }
                list { return $items }
            }
        ";
        let shape = scan(body, &[param("mode"), param("args")], "tcl9.0");
        assert_eq!(shape.subcommands, vec!["add", "remove", "list"]);
        assert!(
            shape.closed.is_none(),
            "a subcommand table is not a value set"
        );
        assert!(
            shape.evidence.iter().any(|e| e.contains("subcommands")),
            "{:?}",
            shape.evidence
        );
    }

    #[test]
    fn a_dispatch_without_a_default_closes_the_value_set() {
        let body = r"
            switch -- $how {
                fast { set n 1 }
                slow { set n 2 }
            }
        ";
        let shape = scan(body, &[param("what"), param("how")], "tcl9.0");
        let closed = shape.closed.expect("a closed set");
        assert_eq!(closed.index, 1);
        assert_eq!(closed.values, vec!["fast", "slow"]);
    }

    #[test]
    fn a_default_arm_leaves_the_value_set_open() {
        let body = r"
            switch -- $how {
                fast { set n 1 }
                slow { set n 2 }
                default { set n 3 }
            }
        ";
        let shape = scan(body, &[param("what"), param("how")], "tcl9.0");
        assert!(shape.closed.is_none());
        assert!(
            shape
                .evidence
                .iter()
                .any(|e| e.contains("not a closed set")),
            "{:?}",
            shape.evidence
        );
    }

    #[test]
    fn a_callback_carries_its_appended_arity() {
        let body = r"
            foreach item $items {
                $callback $item $index
            }
        ";
        let shape = scan(body, &[param("items"), param("callback")], "tcl9.0");
        assert_eq!(
            shape.callbacks,
            vec![Callback {
                index: 1,
                appended: Appended::Exactly(2)
            }]
        );
        assert!(
            shape
                .evidence
                .iter()
                .any(|e| e.contains("2 word(s) appended")),
            "{:?}",
            shape.evidence
        );
    }

    #[test]
    fn disagreeing_call_sites_report_the_smallest_appended_arity() {
        let body = r"
            if {$early} {
                $cb $a
            } else {
                $cb $a $b
            }
        ";
        let shape = scan(body, &[param("cb"), param("a"), param("b")], "tcl9.0");
        assert_eq!(shape.callbacks[0].appended, Appended::AtLeast(1));
    }

    #[test]
    fn a_body_that_says_nothing_yields_nothing() {
        let shape = scan(
            "return [expr {$a + $b}]",
            &[param("a"), param("b")],
            "tcl9.0",
        );
        assert!(shape.is_empty(), "{shape:?}");
        assert!(shape.evidence.is_empty());
    }

    #[test]
    fn arithmetic_is_never_mistaken_for_an_option() {
        let body = r"
            if {$n - 1 > 0} { set n [expr {$n - 1}] }
            if {$x == -1} { set x 0 }
        ";
        let shape = scan(body, &[param("n"), param("x")], "tcl9.0");
        assert!(shape.options.is_empty(), "{:?}", shape.options);
    }

    #[test]
    fn the_findings_land_in_the_draft_in_its_own_shapes() {
        let body = r"
            switch -- $opt {
                -loud { set loud 1 }
                -times { set n [lindex $args 1] }
            }
            $cb $item
        ";
        let shape = scan(body, &[param("opt"), param("cb"), param("args")], "tcl9.0");
        let mut d = draft::default_command_draft();
        let mut notes = Vec::new();
        shape.apply(&mut d, &mut notes);
        let options = d["options"].as_array().expect("option rows");
        assert_eq!(options.len(), 2);
        assert_eq!(options[0]["name"], json!("-loud"));
        assert_eq!(options[0]["value"], Value::Null, "a flag takes no value");
        assert_eq!(options[1]["value"]["role"], json!("Value"));
        assert_eq!(d["command_prefixes"][0]["index"], json!(1));
        assert_eq!(d["command_prefixes"][0]["arity"]["kind"], json!("Exactly"));
        assert!(!notes.is_empty());
    }
}
