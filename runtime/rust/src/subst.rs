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

//! Tcl substitution engine (`subst`, and the eval loop's word expander) — T1.2.
//!
//! Implements `subst_flagged` on the shared [`crate::parse::scan_parts`]
//! component model so parse and subst share one scanner.
//!
//! Substitution is two halves:
//! 1. **Scan** the input into components ([`scan`]) — pure, done here.
//! 2. **Resolve** each component to bytes ([`resolve_with`]) — backslashes and
//!    literals resolve here; **variables** and **command substitutions** are
//!    supplied as caller closures, because resolving them needs the var tables
//!    and the eval loop (T1.3/T1.4). Wiring those closures to the runtime
//!    completes `subst`/word-expansion; until then the engine is complete and
//!    unit-tested against mock resolvers.
//!
//! `unsafe`-free.

#![forbid(unsafe_code)]

use tcl_core_types::RecursionLimit;

use crate::parse::{self, WordBody, WordPart};

/// Cap on `$name(index)` nesting depth [`resolve_parts`] will recurse into
/// while resolving a `WordPart::Variable`'s own `index` components. Separate
/// from (but the same class of bug as) `crate::parse::MAX_SCAN_PARTS_DEPTH`:
/// that cap bounds how deep the *scanner* will parse a nested array index
/// into its own substitution components; this one bounds how deep the
/// *resolver* will walk back down that same shape of tree when substituting
/// it, since a `WordBody` can in principle reach this function from other
/// callers than `crate::parse::scan_parts` — no shared helper ties the two
/// recursions together, so each needs its own guard (issue #996). Same
/// construct, same conservative value; see that constant's doc comment for
/// the full empirical crash-threshold measurements.
const MAX_RESOLVE_PARTS_DEPTH: RecursionLimit = RecursionLimit(64);

/// Which substitution kinds are active (mirrors `subst`'s `-no*` options).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubstFlags {
    /// `$var` / `${var}` / `$arr(i)` (disabled by `-novariables`).
    pub vars: bool,
    /// `[cmd]` (disabled by `-nocommands`).
    pub cmds: bool,
    /// `\x` escapes (disabled by `-nobackslashes`).
    pub backslashes: bool,
}

impl Default for SubstFlags {
    /// All substitutions active — bare/quoted word context, and plain `subst`.
    fn default() -> Self {
        SubstFlags {
            vars: true,
            cmds: true,
            backslashes: true,
        }
    }
}

/// Scan `src` into substitution components per `flags`, without evaluating.
pub fn scan(src: &[u8], flags: SubstFlags) -> WordBody<'_> {
    parse::scan_parts(src, flags.vars, flags.cmds, flags.backslashes)
}

/// Resolve a scanned [`WordBody`] to bytes. Literals are copied and backslashes
/// decoded here; `var` resolves a variable reference (name + optional resolved
/// index), `cmd` resolves a command substitution (the inner script). Either
/// returning `None` contributes nothing (an unset var / empty
/// result appends nothing) — error propagation is layered on with the eval loop.
pub fn resolve_with<V, C>(body: &WordBody, var: &V, cmd: &C) -> Vec<u8>
where
    V: Fn(&[u8], Option<&[u8]>) -> Option<Vec<u8>>,
    C: Fn(&[u8]) -> Option<Vec<u8>>,
{
    match body {
        WordBody::Literal(b) => b.to_vec(),
        WordBody::Parts(parts) => resolve_parts(parts, var, cmd, 0),
    }
}

/// [`resolve_with`]'s worker, threading the `$name(index)` nesting `depth`
/// through the self-recursive index-resolution call — see
/// [`MAX_RESOLVE_PARTS_DEPTH`]. Past the cap, a nested `$name(index)` is
/// resolved as if it had no index at all (`var` is called with `None`
/// instead of recursively resolving the index's own components) rather than
/// recursing further — a bounded, defined fallback instead of an uncatchable
/// native-stack overflow.
fn resolve_parts<V, C>(parts: &[WordPart], var: &V, cmd: &C, depth: u32) -> Vec<u8>
where
    V: Fn(&[u8], Option<&[u8]>) -> Option<Vec<u8>>,
    C: Fn(&[u8]) -> Option<Vec<u8>>,
{
    let past_cap = MAX_RESOLVE_PARTS_DEPTH.exceeded(depth);
    let mut out = Vec::new();
    for part in parts {
        match part {
            WordPart::Text(b) => out.extend_from_slice(b),
            WordPart::Variable(v) => {
                let index = if past_cap {
                    None
                } else {
                    v.index
                        .as_ref()
                        .map(|p| resolve_parts(p, var, cmd, depth + 1))
                };
                if let Some(val) = var(v.name, index.as_deref()) {
                    out.extend_from_slice(&val);
                }
            }
            WordPart::Command(script) => {
                if let Some(val) = cmd(script) {
                    out.extend_from_slice(&val);
                }
            }
        }
    }
    out
}

/// `subst` with only backslashes active (`-novariables -nocommands`): `$` and
/// `[` are literal, `\x` decodes. Equivalent to decoding the whole span via the
/// shared [`tcl_syntax::backslash`] decoder.
pub fn backslashes_only(src: &[u8]) -> Vec<u8> {
    tcl_syntax::backslash::decode_bytes(src).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::VarRef;

    // A mock variable table + command evaluator for the resolve tests.
    fn var(name: &[u8], index: Option<&[u8]>) -> Option<Vec<u8>> {
        match (name, index) {
            (b"x", None) => Some(b"42".to_vec()),
            (b"greeting", None) => Some(b"hi".to_vec()),
            (b"arr", Some(b"k")) => Some(b"val".to_vec()),
            (b"arr", Some(b"42")) => Some(b"indexed-by-x".to_vec()),
            // Self-referential index chaining, for the moderate-depth
            // `resolve_parts` regression test below: `$arr(k)` -> "val", so
            // `$arr($arr(k))` -> `$arr(val)` -> "nested-val".
            (b"arr", Some(b"val")) => Some(b"nested-val".to_vec()),
            _ => None,
        }
    }
    fn cmd(script: &[u8]) -> Option<Vec<u8>> {
        // Pretend `[upper hi]` evaluates to `HI`; everything else echoes.
        if script == b"upper hi" {
            Some(b"HI".to_vec())
        } else {
            Some(script.to_vec())
        }
    }

    fn subst(src: &[u8]) -> Vec<u8> {
        resolve_with(&scan(src, SubstFlags::default()), &var, &cmd)
    }

    #[test]
    fn literal_passthrough() {
        assert_eq!(subst(b"plain text"), b"plain text");
    }

    #[test]
    fn variable_simple_and_braced() {
        assert_eq!(subst(b"$x"), b"42");
        assert_eq!(subst(b"a${greeting}b"), b"ahib");
        assert_eq!(subst(b"$greeting $x"), b"hi 42");
    }

    #[test]
    fn array_index_is_itself_substituted() {
        assert_eq!(subst(b"$arr(k)"), b"val");
        // index `$x` → "42" → arr(42)
        assert_eq!(subst(b"$arr($x)"), b"indexed-by-x");
    }

    /// A moderately nested array index — `$arr($arr(k))`, two levels of
    /// self-referential index resolution via the normal `scan` ->
    /// `resolve_with` pipeline — well under `MAX_RESOLVE_PARTS_DEPTH` (64).
    /// The safety net must not alter this at all: `$arr(k)` resolves to
    /// "val" first, so the outer becomes `$arr(val)` -> "nested-val". (The
    /// trailing `)` is a pre-existing, unrelated property of `scan_parts`'s
    /// `)`-terminator search — see `parse::tests::
    /// moderately_nested_array_index_still_scans_fully` — not something this
    /// fix changes.)
    #[test]
    fn moderately_nested_array_index_resolves_correctly() {
        assert_eq!(subst(b"$arr($arr(k))"), b"nested-val)");
    }

    /// Regression coverage for issue #996: `resolve_parts` recurses once per
    /// `$name(index)` nesting level while resolving a `WordPart::Variable`'s
    /// own `index` components, with no depth cap before this fix. In the
    /// live pipeline this tree comes from `crate::parse::scan_parts`, which
    /// this same sweep capped at `MAX_SCAN_PARTS_DEPTH` (64) — bounding what
    /// `resolve_with` receives via the normal `scan` -> `resolve_with` path —
    /// but `resolve_parts` has no shared helper with `scan_parts` (see
    /// `MAX_RESOLVE_PARTS_DEPTH`'s doc comment) and `resolve_with` is public
    /// API surface in its own right, reachable from any `WordBody` a caller
    /// builds. This test builds a `WordPart` tree directly — bypassing
    /// `scan_parts` entirely — to exercise `resolve_parts`'s own cap in
    /// isolation. 5000 levels is comfortably past `MAX_RESOLVE_PARTS_DEPTH`
    /// (64) and the same order of magnitude past the crash depths this class
    /// of unguarded recursion hit elsewhere in this sweep (SIGABRT between
    /// depth 100-150 on a 256 KiB stack, still crashing at depth 2000 on a 1
    /// MiB stack); the assertion is that resolution returns at all, not what
    /// it returns.
    #[test]
    fn deeply_nested_array_index_survives_resolve_parts() {
        const DEPTH: usize = 5000;
        // $v($v($v(...($v)))) — built directly as a `WordPart` tree (every
        // level reuses the same borrowed name; only the nesting depth
        // matters for this test).
        let mut index: Option<Vec<WordPart>> = None;
        for _ in 0..DEPTH {
            index = Some(vec![WordPart::Variable(VarRef { name: b"v", index })]);
        }
        let body = WordBody::Parts(index.expect("DEPTH > 0"));
        let _ = resolve_with(&body, &var, &cmd);
    }

    #[test]
    fn command_substitution() {
        assert_eq!(subst(b"[upper hi] there"), b"HI there");
        // nested brackets stay together
        assert_eq!(subst(b"[a [b] c]"), b"a [b] c");
    }

    #[test]
    fn backslash_decoding() {
        assert_eq!(subst(b"a\\tb"), b"a\tb");
        assert_eq!(subst(b"\\x41\\x42"), b"AB");
        // `\$` is a literal dollar (unknown escape → byte verbatim)
        assert_eq!(subst(b"\\$x"), b"$x");
    }

    #[test]
    fn dollar_not_a_name_is_literal() {
        // `$` not followed by a name char / `{` is a literal `$` (tclParse.c).
        assert_eq!(subst(b"$ x"), b"$ x");
        assert_eq!(subst(b"price$"), b"price$");
        assert_eq!(subst(b"a$%b"), b"a$%b");
    }

    #[test]
    fn flags_disable_kinds() {
        // -novariables: `$x` stays literal, `\t` still decodes.
        let f = SubstFlags {
            vars: false,
            cmds: true,
            backslashes: true,
        };
        assert_eq!(resolve_with(&scan(b"$x\\t", f), &var, &cmd), b"$x\t");
        // backslashes_only: `$`/`[` literal, `\n` decodes.
        assert_eq!(backslashes_only(b"$x[y]\\n"), b"$x[y]\n");
    }
}
