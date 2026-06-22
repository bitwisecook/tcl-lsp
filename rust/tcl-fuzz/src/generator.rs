//! Grammar-aware Tcl script generator for differential fuzzing.
//!
//! Port of `tooling/fuzzing/tcl_gen.py`, scoped to the surface the native VM
//! implements so a divergence points at a real miscompile rather than an
//! unimplemented command. Every emitted script:
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

/// Generate one script from `seed`.
#[must_use]
pub fn generate(seed: u64, config: &GenConfig) -> String {
    let mut g = Gen {
        rng: Rng::new(seed),
        config: *config,
        out: String::new(),
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
            self.leaf_statement();
        } else {
            match self.rng.below(5) {
                0 => self.if_stmt(depth),
                1 => self.while_stmt(depth),
                2 => self.for_stmt(depth),
                3 => self.foreach_stmt(depth),
                _ => self.leaf_statement(),
            }
        }
    }

    fn leaf_statement(&mut self) {
        match self.rng.below(8) {
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
            _ => {
                let e = self.expr(0);
                let _ = writeln!(self.out, "puts [expr {{{e}}}]");
            }
        }
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

    /// A `string` ensemble subcommand producing a value.
    fn string_op(&mut self) -> String {
        let w = self.rng.pick(WORDS);
        match self.rng.below(6) {
            0 => format!("string length {w}"),
            1 => format!("string toupper {w}"),
            2 => format!("string tolower {w}"),
            3 => format!("string reverse {w}"),
            4 => format!("string index {w} {}", self.rng.below(4)),
            _ => format!("string range {w} 0 {}", self.rng.below(4)),
        }
    }

    /// A `list`/`l*` operation producing a value.
    fn list_op(&mut self) -> String {
        let list = self.list_literal();
        match self.rng.below(6) {
            0 => format!("llength {list}"),
            1 => format!("lindex {list} {}", self.rng.below(self.config.max_list_len)),
            2 => format!("lreverse {list}"),
            3 => format!("lsort {list}"),
            4 => format!("lrange {list} 0 {}", self.rng.below(self.config.max_list_len)),
            _ => format!("lsearch {list} {}", self.rng.pick(WORDS)),
        }
    }

    /// A (possibly nested) integer expression. Division/modulo guard against a
    /// zero divisor so both engines agree on the (non-error) result.
    fn expr(&mut self, depth: u32) -> String {
        if depth >= self.config.max_expr_depth || self.rng.chance(1, 2) {
            // Leaf: a variable read or a small integer literal.
            return if self.rng.chance(1, 2) {
                format!("${}", self.var())
            } else {
                self.rng.below(20).to_string()
            };
        }
        let op = *self.rng.pick(&["+", "-", "*", "/", "%", "<", ">", "==", "!=", "&&", "||"]);
        let left = self.expr(depth + 1);
        let right = self.expr(depth + 1);
        match op {
            // Guard divide/modulo so a zero divisor can't make one engine error
            // and the other not — that is a divergence in the *test*, not the VM.
            "/" | "%" => format!("({left}) {op} (({right}) == 0 ? 1 : ({right}))"),
            _ => format!("({left}) {op} ({right})"),
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
            let (mut brace, mut brack) = (0i32, 0i32);
            for b in src.bytes() {
                match b {
                    b'{' => brace += 1,
                    b'}' => brace -= 1,
                    b'[' => brack += 1,
                    b']' => brack -= 1,
                    _ => {}
                }
                assert!(brace >= 0 && brack >= 0, "seed {seed}: closer before opener");
            }
            assert_eq!(brace, 0, "seed {seed}: unbalanced braces\n{src}");
            assert_eq!(brack, 0, "seed {seed}: unbalanced brackets\n{src}");
        }
    }

    #[test]
    fn nonempty_and_prints() {
        let cfg = GenConfig::default();
        // Most scripts should contain a `puts` so the differential has output.
        let with_puts = (0..100).filter(|s| generate(*s, &cfg).contains("puts")).count();
        assert!(with_puts > 50, "too few scripts print output: {with_puts}/100");
    }
}
