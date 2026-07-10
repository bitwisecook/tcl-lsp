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

//! Grammar-aware Tcl script generator for differential fuzzing.
//!
//! Scoped to the surface the native VM implements so a divergence points at a
//! real miscompile rather than an unimplemented command. Every emitted script:
//!
//! * is syntactically valid Tcl (balanced `{}` / `[]` / `""`);
//! * is **pure** — no I/O, file, socket, exec, clock, or `after` commands;
//! * has **bounded** loops (literal integer bounds), so neither backend hangs;
//! * prints deterministic output via `puts`, so the differential has something
//!   to compare.
//!
//! The generator is parameterised by a [`GenConfig`] and a seed, so any finding
//! replays exactly.

use std::fmt::Write as _;

use crate::rng::Rng;

/// Tunables for the generator.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_field_names)] // `max_*` reads naturally for bound knobs.
pub struct GenConfig {
    /// Maximum nesting depth of control structures.
    pub max_depth: u32,
    /// Maximum number of top-level statements.
    pub max_stmts: usize,
    /// Maximum generated list length.
    pub max_list_len: usize,
    /// Maximum expression nesting depth.
    pub max_expr_depth: u32,
}

impl Default for GenConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_stmts: 10,
            max_list_len: 5,
            max_expr_depth: 3,
        }
    }
}

/// Variable names the generator draws from (kept small so reads usually hit a
/// previously-assigned variable).
const VARS: &[&str] = &["a", "b", "c", "d", "i", "j", "x", "y", "z"];

/// Short literal words used as list elements / string operands.
const WORDS: &[&str] = &["foo", "bar", "baz", "qux", "hello", "world", "abc", "x", ""];

/// Proc names the generator defines and later calls. Kept distinct from any Tcl
/// builtin so a redefinition never shadows a command both engines rely on.
const PROC_NAMES: &[&str] = &["p0", "p1", "p2", "helper", "compute"];

/// Single-segment namespace names. Deliberately no `::` prefixes / trailing
/// runs: namespace-name canonicalisation of multiple/trailing `::` is a known
/// VM gap, so steering clear of it keeps a divergence pointed at a real
/// miscompile rather than name-normalisation noise.
const NS_NAMES: &[&str] = &["n1", "n2", "ns"];

/// Generate one script from `seed`.
#[must_use]
pub fn generate(seed: u64, config: &GenConfig) -> String {
    let mut g = Gen {
        rng: Rng::new(seed),
        config: *config,
        out: String::new(),
        procs: Vec::new(),
    };
    // Seed a few variables so later reads resolve, matching how real scripts
    // initialise state before using it.
    for v in ["a", "b", "i", "x"] {
        let _ = writeln!(g.out, "set {v} {}", g.rng.below(20));
    }
    let stmts = 1 + g.rng.below(g.config.max_stmts);
    for _ in 0..stmts {
        g.statement(0);
    }
    g.out
}

struct Gen {
    rng: Rng,
    config: GenConfig,
    out: String,
    /// Procs defined so far, as `(name, arity)`. Calls draw from this so a
    /// generated call always matches a real definition; a redefinition with a
    /// different arity replaces the old entry rather than accumulating.
    procs: Vec<(&'static str, usize)>,
}

impl Gen {
    fn var(&mut self) -> &'static str {
        self.rng.pick(VARS)
    }

    /// Emit one statement at nesting `depth`.
    fn statement(&mut self, depth: u32) {
        // At max depth, only emit leaf (non-nesting) statements.
        let leaf = depth >= self.config.max_depth || self.rng.chance(2, 3);
        if leaf {
            self.leaf_statement(depth);
        } else {
            // proc / namespace definitions are top-level only (matching the
            // oracle): nesting them changes scope semantics in ways the
            // generator doesn't model.
            let arms = if depth == 0 { 9 } else { 7 };
            match self.rng.below(arms) {
                0 => self.if_stmt(depth),
                1 => self.while_stmt(depth),
                2 => self.for_stmt(depth),
                3 => self.foreach_stmt(depth),
                4 => self.switch_stmt(depth),
                5 => self.catch_stmt(depth),
                6 => self.try_stmt(depth),
                7 => self.proc_stmt(depth),
                8 => self.namespace_stmt(depth),
                _ => self.leaf_statement(depth),
            }
        }
    }

    fn leaf_statement(&mut self, depth: u32) {
        // A call to an already-defined proc, emitted only at top level so a
        // recursive definition can't drive unbounded generated recursion. The
        // call exercises the proc's return value through the differential.
        if depth == 0 && !self.procs.is_empty() && self.rng.chance(1, 4) {
            self.proc_call_stmt();
            return;
        }
        match self.rng.below(18) {
            0 => {
                let v = self.var();
                let e = self.expr(0);
                let _ = writeln!(self.out, "set {v} [expr {{{e}}}]");
            }
            1 => {
                let v = self.var();
                let _ = writeln!(self.out, "incr {v} {}", self.rng.below(5));
            }
            2 => {
                let v = self.var();
                let w = self.rng.pick(WORDS);
                let _ = writeln!(self.out, "append {v} {w}");
            }
            3 => {
                let v = self.var();
                let list = self.list_literal();
                let _ = writeln!(self.out, "set {v} {list}");
            }
            4 => {
                let v = self.var();
                let w = self.rng.pick(WORDS);
                let _ = writeln!(self.out, "lappend {v} {w}");
            }
            5 => {
                let s = self.string_op();
                let _ = writeln!(self.out, "puts [{s}]");
            }
            6 => {
                let l = self.list_op();
                let _ = writeln!(self.out, "puts [{l}]");
            }
            7 => {
                let e = self.expr(0);
                let _ = writeln!(self.out, "puts [expr {{{e}}}]");
            }
            8 => self.dict_mutator(),
            9 => {
                let d = self.dict_op();
                let _ = writeln!(self.out, "puts [{d}]");
            }
            10 => {
                let f = self.format_op();
                let _ = writeln!(self.out, "puts [{f}]");
            }
            11 => {
                let s = self.scan_op();
                let _ = writeln!(self.out, "puts [{s}]");
            }
            12 => self.array_stmt(),
            13 => {
                // `apply` a pure lambda to a small integer — result deterministic.
                let n = self.rng.below(20);
                let body =
                    *self
                        .rng
                        .pick(&["expr {$_p * 2}", "expr {$_p + 1}", "string length $_p"]);
                let _ = writeln!(self.out, "puts [apply {{{{_p}} {{{body}}}}} {n}]");
            }
            14 => {
                // `eval` a small assignment script (a list of words is a command).
                let v = self.var();
                let n = self.rng.below(20);
                let _ = writeln!(self.out, "eval [list set {v} {n}]");
            }
            15 => {
                // `subst` a string with an embedded variable read and command.
                let v = self.var();
                let _ = writeln!(self.out, "puts [subst {{${v}-[expr {{1+1}}]}}]");
            }
            16 => self.scope_stmt(),
            _ => {
                let r = self.regex_op();
                let _ = writeln!(self.out, "puts [{r}]");
            }
        }
    }

    /// A scoping statement: a one-shot proc that reaches an enclosing/global
    /// variable through `global` or `upvar`, then is called. The referenced
    /// variables (`a`/`b`) are seeded at top level, so the result is
    /// deterministic (`RUST_ISSUE_064`).
    fn scope_stmt(&mut self) {
        if self.rng.chance(1, 2) {
            // `global`: read module-global scalars from inside a proc.
            self.out
                .push_str("proc _gp {} {global a b\n return [list $a $b]}\n");
            self.out.push_str("puts [_gp]\n");
        } else {
            // `upvar`: alias the caller's variable (the global, at call level 1).
            let v = *self.rng.pick(&["a", "b", "i", "x"]);
            self.out
                .push_str("proc _up {vn} {upvar 1 $vn v\n return $v}\n");
            let _ = writeln!(self.out, "puts [_up {v}]");
        }
    }

    /// A `regexp` / `regsub` over a fixed, safe pattern and a word/variable
    /// subject — deterministic 0/1 (match) or substituted-string output.
    fn regex_op(&mut self) -> String {
        let pat = *self.rng.pick(&["[a-z]+", "[0-9]", "o+", "^ba", "a|o"]);
        let subject = if self.rng.chance(1, 2) {
            format!("${}", self.var())
        } else {
            self.nonempty_word().to_string()
        };
        match self.rng.below(3) {
            0 => format!("regexp {{{pat}}} {subject}"),
            1 => format!("regexp -all {{{pat}}} {subject}"),
            // `regsub` with an explicit result var (portable across 8.x/9.x),
            // printing the substituted string.
            _ => format!("regsub -all {{{pat}}} {subject} X _rs; set _rs"),
        }
    }

    /// An `array` statement: seed a scratch array with a literal, then read it
    /// back through a deterministic op. Enumeration (`array names`/`get`) is
    /// wrapped in `lsort` because Tcl's hash-iteration order is unspecified and
    /// legitimately differs between engines — only a *sorted* view is comparable.
    fn array_stmt(&mut self) {
        let k1 = self.nonempty_word();
        let k2 = self.nonempty_word();
        let (v1, v2) = (self.rng.below(20), self.rng.below(20));
        let _ = writeln!(self.out, "array set _arr {{{k1} {v1} {k2} {v2}}}");
        match self.rng.below(5) {
            0 => {
                let _ = writeln!(self.out, "puts [array size _arr]");
            }
            1 => {
                let _ = writeln!(self.out, "puts [array exists _arr]");
            }
            2 => {
                // A read of a key the seed guarantees is present.
                let _ = writeln!(self.out, "puts [set _arr({k1})]");
            }
            3 => {
                let _ = writeln!(self.out, "puts [lsort [array names _arr]]");
            }
            _ => {
                let _ = writeln!(self.out, "puts [lsort [array get _arr]]");
            }
        }
        // Drop the scratch array so a later re-seed with different keys doesn't
        // accumulate stale elements (which would desync the two engines' views).
        self.out.push_str("array unset _arr\n");
    }

    fn if_stmt(&mut self, depth: u32) {
        let cond = self.expr(0);
        let _ = writeln!(self.out, "if {{{cond}}} {{");
        self.block(depth + 1);
        if self.rng.chance(1, 2) {
            self.out.push_str("} else {\n");
            self.block(depth + 1);
        }
        self.out.push_str("}\n");
    }

    fn while_stmt(&mut self, depth: u32) {
        // Bounded: a fresh counter var counts up to a small literal.
        let n = 1 + self.rng.below(4);
        let _ = writeln!(self.out, "set _w 0\nwhile {{$_w < {n}}} {{");
        self.block(depth + 1);
        self.out.push_str("incr _w\n}\n");
    }

    fn for_stmt(&mut self, depth: u32) {
        let n = 1 + self.rng.below(4);
        let _ = writeln!(self.out, "for {{set _f 0}} {{$_f < {n}}} {{incr _f}} {{");
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    fn foreach_stmt(&mut self, depth: u32) {
        let v = self.var();
        let list = self.list_literal();
        let _ = writeln!(self.out, "foreach {v} {list} {{");
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    fn switch_stmt(&mut self, depth: u32) {
        // `switch -- <val>` over a couple of literal patterns plus a default,
        // so exactly one arm fires and the output is deterministic.
        let val = self.simple_value();
        let _ = writeln!(self.out, "switch -- {val} {{");
        let ncases = 1 + self.rng.below(3);
        for _ in 0..ncases {
            // Non-empty pattern word so the brace-form parse is unambiguous.
            let pat = self.nonempty_word();
            let _ = writeln!(self.out, "{pat} {{");
            self.block(depth + 1);
            self.out.push_str("}\n");
        }
        self.out.push_str("default {\n");
        self.block(depth + 1);
        self.out.push_str("}\n}\n");
    }

    fn catch_stmt(&mut self, depth: u32) {
        // `catch` of a small body, printing the return code (0/1) so the
        // differential compares the caught status, not just side effects.
        self.out.push_str("puts [catch {\n");
        self.block(depth + 1);
        self.out.push_str("} _cm]\n");
    }

    fn try_stmt(&mut self, depth: u32) {
        self.out.push_str("try {\n");
        self.block(depth + 1);
        self.out.push_str("} on error {_e} {\n");
        self.block(depth + 1);
        if self.rng.chance(1, 2) {
            self.out.push_str("} finally {\n");
            self.block(depth + 1);
        }
        self.out.push_str("}\n");
    }

    fn proc_stmt(&mut self, depth: u32) {
        let name = *self.rng.pick(PROC_NAMES);
        let nparams = self.rng.below(3);
        // Distinct parameter names so the signature is well-formed.
        let mut params: Vec<&'static str> = Vec::new();
        for &v in VARS {
            if params.len() >= nparams {
                break;
            }
            params.push(v);
        }
        let param_str = params.join(" ");
        let _ = writeln!(self.out, "proc {name} {{{param_str}}} {{");
        self.block(depth + 1);
        // A literal return value keeps the proc's result deterministic.
        let ret = self.simple_value();
        let _ = writeln!(self.out, "return {ret}");
        self.out.push_str("}\n");
        // Record (or refresh) the proc's arity for later calls.
        self.procs.retain(|(n, _)| *n != name);
        self.procs.push((name, params.len()));
    }

    fn proc_call_stmt(&mut self) {
        let procs = self.procs.clone();
        let (name, arity) = *self.rng.pick(&procs);
        let mut call = String::from(name);
        for _ in 0..arity {
            let _ = write!(call, " {}", self.rng.below(20));
        }
        let _ = writeln!(self.out, "puts [{call}]");
    }

    fn namespace_stmt(&mut self, depth: u32) {
        let ns = *self.rng.pick(NS_NAMES);
        let _ = writeln!(self.out, "namespace eval {ns} {{");
        self.block(depth + 1);
        self.out.push_str("}\n");
    }

    /// Mutate a dict-valued variable in place. An unset target is created by
    /// the first `dict set`, matching Tcl semantics; `dict incr` on a
    /// non-integer errors in both engines (so it stays a match, not a finding).
    fn dict_mutator(&mut self) {
        let v = self.var();
        let key = self.nonempty_word();
        match self.rng.below(5) {
            0 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict set {v} {key} {val}");
            }
            1 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict append {v} {key} {val}");
            }
            2 => {
                let _ = writeln!(self.out, "dict incr {v} {key} {}", self.rng.below(5));
            }
            3 => {
                let val = self.nonempty_word();
                let _ = writeln!(self.out, "dict lappend {v} {key} {val}");
            }
            _ => {
                let _ = writeln!(self.out, "dict unset {v} {key}");
            }
        }
    }

    /// A `dict` ensemble subcommand producing a value, over a literal dict.
    fn dict_op(&mut self) -> String {
        let d = self.dict_literal();
        match self.rng.below(5) {
            0 => format!("dict size {d}"),
            1 => format!("dict keys {d}"),
            2 => format!("dict values {d}"),
            3 => format!("dict exists {d} {}", self.nonempty_word()),
            // `dict get` of a present key (the literal always contains `foo`).
            _ => format!("dict get {d} foo"),
        }
    }

    /// A small literal dict. Always includes the `foo` key so a `dict get foo`
    /// resolves; remaining pairs are short non-empty words.
    fn dict_literal(&mut self) -> String {
        let mut s = String::from("{foo 1");
        let extra = self.rng.below(3);
        for _ in 0..extra {
            let k = self.nonempty_word();
            let val = self.rng.below(20);
            let _ = write!(s, " {k} {val}");
        }
        s.push('}');
        s
    }

    /// A non-empty short word (`""` excluded so it can't collapse a list/dict
    /// element or swallow a switch pattern).
    fn nonempty_word(&mut self) -> &'static str {
        loop {
            let w = *self.rng.pick(WORDS);
            if !w.is_empty() {
                break w;
            }
        }
    }

    /// A simple scalar value: a variable read or a small literal word/integer.
    fn simple_value(&mut self) -> String {
        match self.rng.below(3) {
            0 => format!("${}", self.var()),
            1 => self.rng.below(20).to_string(),
            _ => self.nonempty_word().to_string(),
        }
    }

    /// Emit a small block of 1–3 statements at `depth`.
    fn block(&mut self, depth: u32) {
        let n = 1 + self.rng.below(3);
        for _ in 0..n {
            self.statement(depth);
        }
    }

    /// A braced list literal of short words.
    fn list_literal(&mut self) -> String {
        let n = self.rng.below(self.config.max_list_len + 1);
        let mut s = String::from("{");
        for k in 0..n {
            if k > 0 {
                s.push(' ');
            }
            // Use a non-empty word so empty elements don't collapse.
            let w = loop {
                let w = self.rng.pick(WORDS);
                if !w.is_empty() {
                    break *w;
                }
            };
            s.push_str(w);
        }
        s.push('}');
        s
    }

    /// A `string` ensemble subcommand producing a value. Every form is
    /// deterministic (no locale/encoding-sensitive output) so a divergence is a
    /// real bug (`RUST_ISSUE_064`).
    fn string_op(&mut self) -> String {
        let w = self.nonempty_word();
        let w2 = self.nonempty_word();
        match self.rng.below(14) {
            0 => format!("string length {w}"),
            1 => format!("string toupper {w}"),
            2 => format!("string tolower {w}"),
            3 => format!("string totitle {w}"),
            4 => format!("string reverse {w}"),
            5 => format!("string index {w} {}", self.rng.below(4)),
            6 => format!("string range {w} 0 {}", self.rng.below(4)),
            7 => format!("string compare {w} {w2}"),
            8 => format!("string equal {w} {w2}"),
            9 => format!("string first {w2} {w}"),
            10 => format!("string last {w2} {w}"),
            11 => format!("string repeat {w} {}", self.rng.below(4)),
            12 => format!("string map {{{w} {w2}}} {w}"),
            _ => format!("string trim {w}{w2} {w}"),
        }
    }

    /// A `list`/`l*` operation producing a value.
    fn list_op(&mut self) -> String {
        let list = self.list_literal();
        match self.rng.below(11) {
            0 => format!("llength {list}"),
            1 => format!("lindex {list} {}", self.rng.below(self.config.max_list_len)),
            2 => format!("lreverse {list}"),
            3 => format!("lsort {list}"),
            4 => format!(
                "lrange {list} 0 {}",
                self.rng.below(self.config.max_list_len)
            ),
            5 => format!("lsearch {list} {}", self.nonempty_word()),
            6 => format!("concat {list} {}", self.list_literal()),
            7 => format!("join {list} -"),
            8 => format!(
                "linsert {list} {} {}",
                self.rng.below(self.config.max_list_len),
                self.nonempty_word()
            ),
            9 => format!(
                "lreplace {list} 0 {} {}",
                self.rng.below(self.config.max_list_len),
                self.nonempty_word()
            ),
            // `lmap` over a bounded literal list, mapping each element with a pure
            // expression — deterministic element order and values.
            _ => format!("lmap _e {list} {{string length $_e}}"),
        }
    }

    /// A `format` over integer / string conversions only (float conversions are
    /// left out so shortest-`double` formatting can't introduce spurious
    /// divergences); every conversion here is byte-deterministic across engines.
    fn format_op(&mut self) -> String {
        let n = self.rng.below(256);
        let w = self.nonempty_word();
        match self.rng.below(8) {
            0 => format!("format %d {n}"),
            1 => format!("format %05d {n}"),
            2 => format!("format %x {n}"),
            3 => format!("format %o {n}"),
            4 => format!("format %-6s|%s {w} {w}"),
            5 => format!("format %c {}", 65 + self.rng.below(26)),
            6 => format!("format {{%d-%d}} {n} {}", self.rng.below(256)),
            _ => format!("format %%{n}"),
        }
    }

    /// A `scan` returning its conversion count (a deterministic small integer);
    /// the scanned-into variables are scratch and not read back.
    fn scan_op(&mut self) -> String {
        match self.rng.below(3) {
            0 => format!("scan {} %d _s0", self.rng.below(256)),
            1 => format!(
                "scan {{{} {}}} {{%d %d}} _s0 _s1",
                self.rng.below(256),
                self.rng.below(256)
            ),
            _ => format!("scan {} %c _s0", self.nonempty_word()),
        }
    }

    /// A (possibly nested) expression. Division/modulo guard against a zero
    /// divisor so both engines agree on the (non-error) result. The leaves mix
    /// integers, floats, and variable reads; a fraction of interior nodes are
    /// string-relational (`eq`/`ne`/`in`/`ni`) so those operators are exercised
    /// (`RUST_ISSUE_064`).
    fn expr(&mut self, depth: u32) -> String {
        if depth >= self.config.max_expr_depth || self.rng.chance(1, 2) {
            return self.expr_leaf();
        }
        // A minority of interior nodes are string/list relational (`eq`/`ne`/
        // `in`/`ni`), which take word operands rather than nested arithmetic.
        if self.rng.chance(1, 5) {
            let op = *self.rng.pick(&["eq", "ne", "in", "ni"]);
            let left = self.nonempty_word();
            return match op {
                // `in`/`ni` test membership in a list literal.
                "in" | "ni" => format!("{{{left}}} {op} {}", self.list_literal()),
                _ => format!("{{{left}}} {op} {{{}}}", self.nonempty_word()),
            };
        }
        let op = *self.rng.pick(&[
            "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "&&", "||",
        ]);
        let left = self.expr(depth + 1);
        let right = self.expr(depth + 1);
        match op {
            // Guard divide/modulo so a zero divisor can't make one engine error
            // and the other not — that is a divergence in the *test*, not the VM.
            // `int(...)` keeps both operands integral so `%` (which rejects a
            // floating-point operand) never errors on a generated float leaf.
            "/" | "%" => {
                format!("int({left}) {op} (int({right}) == 0 ? 1 : int({right}))")
            }
            _ => format!("({left}) {op} ({right})"),
        }
    }

    /// An expression leaf: a variable read, a small integer, or a simple float
    /// (shortest-`double` formatting of the *result* is itself under test).
    fn expr_leaf(&mut self) -> String {
        match self.rng.below(4) {
            0 | 1 => format!("${}", self.var()),
            2 => self.rng.below(20).to_string(),
            // A terminating decimal so the value is exact in binary where
            // possible; the engines must still agree on how they print it.
            _ => format!("{}.{}", self.rng.below(10), self.rng.below(10)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_seed() {
        let cfg = GenConfig::default();
        assert_eq!(generate(42, &cfg), generate(42, &cfg));
        assert_ne!(generate(1, &cfg), generate(2, &cfg));
    }

    #[test]
    fn balanced_delimiters() {
        let cfg = GenConfig::default();
        for seed in 0..200 {
            let src = generate(seed, &cfg);
            let (mut braces, mut brackets) = (0i32, 0i32);
            for b in src.bytes() {
                match b {
                    b'{' => braces += 1,
                    b'}' => braces -= 1,
                    b'[' => brackets += 1,
                    b']' => brackets -= 1,
                    _ => {}
                }
                assert!(
                    braces >= 0 && brackets >= 0,
                    "seed {seed}: closer before opener"
                );
            }
            assert_eq!(braces, 0, "seed {seed}: unbalanced braces\n{src}");
            assert_eq!(brackets, 0, "seed {seed}: unbalanced brackets\n{src}");
        }
    }

    #[test]
    fn nonempty_and_prints() {
        let cfg = GenConfig::default();
        // Most scripts should contain a `puts` so the differential has output.
        let with_puts = (0..100)
            .filter(|s| generate(*s, &cfg).contains("puts"))
            .count();
        assert!(
            with_puts > 50,
            "too few scripts print output: {with_puts}/100"
        );
    }

    #[test]
    fn broadened_grammar_is_exercised() {
        // Over a decent seed range each newly-added production should appear at
        // least once, so the broadened surface is actually generated (not dead).
        let cfg = GenConfig {
            max_depth: 4,
            max_stmts: 16,
            ..GenConfig::default()
        };
        let corpus: Vec<String> = (0..600).map(|s| generate(s, &cfg)).collect();
        let appears = |needle: &str| corpus.iter().any(|s| s.contains(needle));
        for needle in [
            "proc ",
            "namespace eval ",
            "dict ",
            "switch -- ",
            "catch {",
            "try {",
            // Command families added for RUST_ISSUE_064.
            "format ",
            "scan ",
            "array set ",
            "apply {",
            "eval [list",
            "subst {",
            "string compare ",
            "concat ",
            "lmap ",
            "global ",
            "upvar ",
            "regexp ",
            "regsub ",
        ] {
            assert!(appears(needle), "production never generated: {needle:?}");
        }
        // The string/list relational operators reach the expression grammar.
        assert!(
            corpus
                .iter()
                .any(|s| s.contains(" eq ") || s.contains(" in ")),
            "string/list relational operators never generated"
        );
        // A defined proc should sometimes be called back (`puts [p0 ...]`).
        let proc_called = corpus.iter().any(|s| {
            ["p0", "p1", "p2", "helper", "compute"]
                .iter()
                .any(|p| s.contains(&format!("puts [{p}")))
        });
        assert!(proc_called, "no generated proc was ever called");
    }
}
