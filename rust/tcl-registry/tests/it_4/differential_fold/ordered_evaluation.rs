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

//! Evaluation order: the ordered-state witnesses and the prefix rule over the routes.

use super::*;

/// What stops an ordered evaluation: a nested command that ends it with an
/// error, or one nothing here can answer.
#[derive(Debug, PartialEq)]
pub(super) enum Stop {
    Ended,
    Declined(String),
}

/// `command args…` through the registry's route with `x` holding `prior`:
/// the result's text and the stores it makes, each resolved to its place.
fn ordered_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    command: &str,
    args: &[&str],
    prior: &str,
) -> Option<(String, Vec<(PlaceRef, StoreOutcome)>)> {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        AnalysisInputs, Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs,
        OperandId, resolve_semantics,
    };

    let semantics = resolve_semantics(reg.get(command)?, None, None);
    let semantics = semantics.semantics()?;
    let mut inputs = LiteralInputs::new(command, None, args, profile)
        .with_prior("x", ExactValue::from_literal(prior));
    for role in [ArgRole::VarWrite, ArgRole::VarRead] {
        for index in reg.arg_indices_for_role(command, args, role) {
            inputs = inputs.with_role(OperandId(index), role);
        }
    }
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(result) = outcome.result else {
        return None;
    };
    let stores = outcome
        .ordered_stores
        .into_iter()
        .map(|store| Some((inputs.place(store.target().0).ok()?, store)))
        .collect::<Option<Vec<_>>>()?;
    Some((String::from_utf8(result.bytes).ok()?, stores))
}

/// An expression evaluated in the shared walker's order over the ordered
/// evaluation state: a read of `x` consults the state's writes before the
/// value `x` held when the expression began, and a nested `[incr x]`,
/// `[set x 10]` or `[incr x 10]` runs through its registry route over the
/// value the state gives `x` and applies the stores it makes to the state, in
/// order. `[error mid]` ends the evaluation with the writes made so far.
struct OrderedProbe<'a> {
    reg: &'a CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    pub(super) start: i64,
    state: tcl_registry::value_transfer::EvaluationState,
}

impl<'a> OrderedProbe<'a> {
    pub(super) fn new(
        reg: &'a CommandRegistry,
        profile: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        use tcl_registry::value_transfer::{EvaluationState, NestedPolicy};
        Self {
            reg,
            profile,
            start: 1,
            state: EvaluationState::new(NestedPolicy::LocalWrites),
        }
    }

    /// What a read of `name` finds.
    pub(super) fn read(&self, name: &str) -> Result<i64, Stop> {
        use tcl_registry::value_transfer::WrittenPlace;
        match self.state.written(name) {
            WrittenPlace::Exact(value) => String::from_utf8(value.bytes)
                .ok()
                .and_then(|text| text.parse().ok())
                .ok_or_else(|| Stop::Declined(format!("the value of {name}"))),
            WrittenPlace::Untouched if name == "x" => Ok(self.start),
            WrittenPlace::Untouched | WrittenPlace::Unknown => {
                Err(Stop::Declined(format!("a read of {name}")))
            }
        }
    }

    /// Run the nested command `script`.
    pub(super) fn run(&mut self, script: &str) -> Result<i64, Stop> {
        let words: Vec<&str> = script.split_whitespace().collect();
        let Some((head, args)) = words.split_first() else {
            return Err(Stop::Declined(script.to_owned()));
        };
        if *head == "error" {
            return Err(Stop::Ended);
        }
        let prior = self.read("x")?.to_string();
        let (result, stores) = ordered_route(self.reg, self.profile, head, args, &prior)
            .ok_or_else(|| Stop::Declined(script.to_owned()))?;
        self.state.writes.extend(stores);
        result
            .parse()
            .map_err(|_| Stop::Declined(script.to_owned()))
    }

    /// A quoted word's text after Tcl substitutes it, left to right.
    pub(super) fn substitute(&mut self, text: &str) -> Result<String, Stop> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(at) = rest.find(['$', '[']) {
            out.push_str(&rest[..at]);
            let tail = &rest[at + 1..];
            if rest[at..].starts_with('$') {
                let len = tail
                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(tail.len());
                out.push_str(&self.read(&tail[..len])?.to_string());
                rest = &tail[len..];
            } else {
                let close = tail.find(']').expect("a closed substitution");
                out.push_str(&self.run(&tail[..close])?.to_string());
                rest = &tail[close + 1..];
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// The value of `expression`, braced as `expr` takes it.
    pub(super) fn expr(&mut self, expression: &str) -> Result<i64, Stop> {
        let node = tcl_syntax::expr::parser::parse_expr_for_profile(expression, self.profile);
        tcl_syntax::expr::eval(&node, self)
    }
}

impl tcl_syntax::expr::ExprOps for OrderedProbe<'_> {
    type Value = i64;
    type Error = Stop;

    fn literal(&mut self, text: &str) -> Result<i64, Stop> {
        text.parse().map_err(|_| Stop::Declined(text.to_owned()))
    }
    fn string(&mut self, inner: &str, _substitutes: bool) -> Result<i64, Stop> {
        Err(Stop::Declined(inner.to_owned()))
    }
    fn var(&mut self, name: &str) -> Result<i64, Stop> {
        self.read(name)
    }
    fn command(&mut self, script: &str) -> Result<i64, Stop> {
        self.run(script)
    }
    fn call(&mut self, function: &str, _args: Vec<i64>) -> Result<i64, Stop> {
        Err(Stop::Declined(function.to_owned()))
    }
    fn arith(&mut self, op: tcl_syntax::expr::BinOp, left: i64, right: i64) -> Result<i64, Stop> {
        match op {
            tcl_syntax::expr::BinOp::Add => Ok(left + right),
            other => Err(Stop::Declined(format!("{other:?}"))),
        }
    }
    fn unary(&mut self, op: tcl_syntax::expr::UnaryOp, _value: i64) -> Result<i64, Stop> {
        Err(Stop::Declined(format!("{op:?}")))
    }
    fn compare_numeric(
        &mut self,
        left: &i64,
        right: &i64,
    ) -> Option<tcl_syntax::expr::NumericCompare> {
        Some(tcl_syntax::expr::NumericCompare::Ordered(left.cmp(right)))
    }
    fn compare_string(&mut self, left: &i64, right: &i64) -> Result<std::cmp::Ordering, Stop> {
        Ok(left.to_string().cmp(&right.to_string()))
    }
    fn in_list(&mut self, _needle: &i64, _list: &i64) -> Result<bool, Stop> {
        Err(Stop::Declined("in".to_owned()))
    }
    fn to_bool(&mut self, value: &i64) -> Result<bool, Stop> {
        Ok(*value != 0)
    }
    // Explicit mathematical adapter: no original native object/effect claim.
    fn to_bool_for_purpose(
        &mut self,
        value: &i64,
        _purpose: tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose,
    ) -> Result<bool, Stop> {
        self.to_bool(value)
    }
    fn logical_right_truth(
        &mut self,
        _left: &i64,
        right: &i64,
        _conjunction: bool,
    ) -> Result<bool, Stop> {
        self.to_bool(right)
    }
    fn bool_value(&mut self, b: bool) -> i64 {
        i64::from(b)
    }
    fn unsupported(&mut self, what: &str) -> Stop {
        Stop::Declined(what.to_owned())
    }
}

/// The interface page's seven ordered-state programs over the registry's own
/// routes, against the real `tclsh` of each release found on `PATH`: a read
/// consults the writes the expression's nested commands have made before the
/// value it began with, each nested `incr` and `set` takes the value the state
/// gives `x` and applies its stores in order, and the evaluation ends with the
/// writes so far where a nested command raises. The expression's value, what
/// `x` holds afterwards and how many writes the state made are the same as
/// `tclsh` leaves: 5 and 2 (one write), 0 and 1 (none: the right operand is
/// never reached), 21 and 10 (one), 5 and 3 (two), 2 and 2 (one: the arm not
/// taken writes nothing), 3 and 2 for the quoted word, which substitutes
/// before it is an expression (one), and, for `[incr x] + [error mid]`, no
/// value and 2 (one).
#[test]
fn the_seven_ordered_state_witnesses() {
    let programs: [(&str, Option<&str>, &str, &str, usize); 7] = [
        (
            "{$x + [incr x] + $x}",
            Some("5"),
            "2",
            "puts [expr {$x + [incr x] + $x}]",
            1,
        ),
        (
            "{0 && [incr x]}",
            Some("0"),
            "1",
            "puts [expr {0 && [incr x]}]",
            0,
        ),
        (
            "{$x + [set x 10] + $x}",
            Some("21"),
            "10",
            "puts [expr {$x + [set x 10] + $x}]",
            1,
        ),
        (
            "{[incr x] + [incr x]}",
            Some("5"),
            "3",
            "puts [expr {[incr x] + [incr x]}]",
            2,
        ),
        (
            "{$x ? [incr x] : [incr x 10]}",
            Some("2"),
            "2",
            "puts [expr {$x ? [incr x] : [incr x 10]}]",
            1,
        ),
        (
            "\"$x + [incr x]\"",
            Some("3"),
            "2",
            "puts [expr \"$x + [incr x]\"]",
            1,
        ),
        (
            "{[incr x] + [error mid]}",
            None,
            "2",
            "catch {expr {[incr x] + [error mid]}}",
            1,
        ),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        for (expression, value, after, statement, writes) in programs {
            let mut probe = OrderedProbe::new(reg, profile);
            let got = match expression.strip_prefix('"') {
                Some(inner) => probe
                    .substitute(inner.trim_end_matches('"'))
                    .and_then(|text| probe.expr(&text)),
                None => probe.expr(expression.trim_matches(|c| c == '{' || c == '}')),
            };
            let tcl = format!("set x 1\n{statement}\nputs $x\n");
            let (_, out) = run_tcl(&tclsh, &tcl).expect("tclsh runs");
            let mut lines = out.lines();
            let oracle_value = value.map(|_| lines.next().expect("a value").to_owned());
            let oracle_after = lines.next().expect("x afterwards").to_owned();
            let at = format!("tclsh{} {expression}", version.version_string());
            match (value, got) {
                (Some(value), Ok(got)) => {
                    assert_eq!(got.to_string(), value, "{at}");
                    assert_eq!(oracle_value.as_deref(), Some(value), "{at}: the oracle");
                }
                (None, Err(Stop::Ended)) => {}
                (_, got) => panic!("{at}: the probe answered {got:?}"),
            }
            assert_eq!(oracle_after, after, "{at}: the oracle's x");
            assert_eq!(probe.read("x"), Ok(after.parse().unwrap()), "{at}: x");
            assert_eq!(probe.state.writes.len(), writes, "{at}: the writes made");
        }
    }
}

/// What one place holds after a witness: an array, a scalar's value, or
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Held {
    Array,
    Value(String),
    Unset,
}

/// One prefix witness: the command and its words, the places that hold an
/// array before it, and the first release that has the command.
type PrefixWitness = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
);

/// What the place `name` holds before `command` runs: an array where the
/// witness says so; for `unset`, 1 in `p`, 2 in `q` and nothing in its
/// absent name; and `old` everywhere else.
fn held_before(command: &str, arrays: &[&str], name: &str) -> Held {
    if arrays.contains(&name) {
        Held::Array
    } else if command == "unset" {
        match name {
            "p" => Held::Value("1".to_owned()),
            "q" => Held::Value("2".to_owned()),
            _ => Held::Unset,
        }
    } else {
        Held::Value("old".to_owned())
    }
}

/// `catch {command args…}` under `tclsh` over the places as
/// [`held_before`] gives them: the message, and what each place holds
/// afterwards. The command must raise.
fn prefix_oracle(
    tclsh: &str,
    (command, args, arrays): (&str, &[&str], &[&str]),
    places: &[&str],
) -> (String, Vec<Held>) {
    use std::fmt::Write as _;
    let mut script = String::new();
    for &name in places {
        match held_before(command, arrays, name) {
            Held::Array => writeln!(script, "array set {name} {{k keep}}"),
            Held::Value(value) => writeln!(script, "set {name} {value}"),
            Held::Unset => Ok(()),
        }
        .expect("a script");
    }
    let invocation = tcl_command(command, None, args);
    writeln!(script, "puts [catch {{{invocation}}} msg]\nputs $msg").expect("a script");
    for &name in places {
        writeln!(
            script,
            "if {{[array exists {name}]}} {{puts array}} elseif {{[info exists {name}]}} \
             {{puts \"=[set {name}]\"}} else {{puts unset}}"
        )
        .expect("a script");
    }
    let (_, out) = run_tcl(tclsh, &script).expect("tclsh runs");
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("1"), "{invocation}: tclsh raises");
    let message = lines.next().expect("the message").to_owned();
    let held = lines
        .map(|line| match line {
            "array" => Held::Array,
            "unset" => Held::Unset,
            value => Held::Value(value.trim_start_matches('=').to_owned()),
        })
        .collect();
    (message, held)
}

/// The same invocation through the registry's route, each place's kind as
/// the existence rung would prove it: the message the route proves, if it
/// proves one, and what each place holds after the stores it says ran.
fn prefix_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    (command, args, arrays): (&str, &[&str], &[&str]),
    places: &[(usize, &str)],
) -> (Option<String>, Vec<Held>) {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        AnalysisInputs, BindingKind, Budget, CompletionOutcome, EvalAnswer, ExactValue,
        ExactValueOrUnavailable, Existence, LiteralInputs, OperandId, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get(command).expect(command), None, None);
    let semantics = semantics.semantics().expect("a registry-owned route");
    let mut inputs = LiteralInputs::new(command, None, args, profile);
    for &(index, name) in places {
        inputs = inputs.with_role(OperandId(index), ArgRole::VarWrite);
        let existence = match held_before(command, arrays, name) {
            Held::Array => Existence::Bound(BindingKind::Array),
            Held::Value(value) => {
                inputs = inputs.with_prior(name, ExactValue::from_literal(&value));
                Existence::Bound(BindingKind::Scalar)
            }
            Held::Unset => Existence::Unbound,
        };
        inputs = inputs.with_existence(name, existence);
    }
    let answer = semantics.evaluate(&inputs, &mut Budget::evaluation());
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("{command}: the route answered {answer:?}");
    };
    let CompletionOutcome::Error {
        written, message, ..
    } = &outcome.completion
    else {
        panic!(
            "{command}: the route proves no error: {:?}",
            outcome.completion
        );
    };
    let mut held: Vec<(String, Held)> = places
        .iter()
        .map(|&(_, name)| (name.to_owned(), held_before(command, arrays, name)))
        .collect();
    for store in &outcome.ordered_stores[..*written] {
        let name = inputs.place(store.target().0).expect("a place").name;
        let slot = held
            .iter_mut()
            .find(|(place, _)| *place == name)
            .expect("a witness place");
        match store {
            StoreOutcome::Write { value, .. } => {
                slot.1 = Held::Value(String::from_utf8_lossy(&value.bytes).into_owned());
            }
            StoreOutcome::Unbind { .. } => slot.1 = Held::Unset,
            StoreOutcome::Preserve { .. } => {}
            other => panic!("{command}: a store the witness does not read: {other:?}"),
        }
    }
    let proved = match message {
        ExactValueOrUnavailable::Exact(text) => {
            Some(String::from_utf8_lossy(&text.bytes).into_owned())
        }
        ExactValueOrUnavailable::Unavailable(_) => None,
    };
    (proved, held.into_iter().map(|(_, held)| held).collect())
}

/// A command that raises part-way through its stores, run through the
/// registry's route with each place's kind as the existence rung proves it,
/// against the real `tclsh` of each release found on `PATH` from the
/// command's first: what each place holds after `catch {…}` is what the
/// stores the route says ran leave it, and the message is tclsh's wherever the
/// route proves one. `lassign` stops at the first place it cannot write (8.5
/// on), so `lassign {new second} a b` over an array `b` writes `a` alone, and
/// `lassign {x y z} a b c` leaves `c` as well; `regexp` stops the same way;
/// `scan` goes on past the array, so with `a` an array `scan {1 2} {%d %d} a
/// b` still writes `b`; and `unset p nosuch q` unbinds `p`, raises on the
/// absent name and leaves `q` at 2.
#[test]
fn the_prefix_rule_over_the_routes_matches_every_release_on_path() {
    use tcl_registry::ArgRole;
    let witnesses: [PrefixWitness; 6] = [
        ("lassign", &["new second", "a", "b"], &["b"], "8.5"),
        ("lassign", &["x y z", "a", "b", "c"], &["b"], "8.5"),
        ("regexp", &["(x)(y)", "xy", "a", "b"], &["b"], "8.4"),
        ("scan", &["1 2", "%d %d", "a", "b"], &["b"], "8.4"),
        ("scan", &["1 2", "%d %d", "a", "b"], &["a"], "8.4"),
        ("unset", &["p", "nosuch", "q"], &[], "8.4"),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        for (command, args, arrays, first) in witnesses {
            if version.version_string() < first {
                continue;
            }
            let at = format!(
                "tclsh{} {}",
                version.version_string(),
                tcl_command(command, None, args)
            );
            let places: Vec<(usize, &str)> = reg
                .arg_indices_for_role(command, args, ArgRole::VarWrite)
                .into_iter()
                .map(|index| (index, args[index]))
                .collect();
            let names: Vec<&str> = places.iter().map(|&(_, name)| name).collect();
            let (message, oracle) = prefix_oracle(&tclsh, (command, args, arrays), &names);
            let (proved, held) = prefix_route(reg, profile, (command, args, arrays), &places);
            if let Some(proved) = proved {
                assert_eq!(proved, message, "{at}: the message");
            }
            assert_eq!(held, oracle, "{at}: what each place holds");
        }
    }
}
