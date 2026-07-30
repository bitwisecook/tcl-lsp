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

//! Command-table indirection: the one hop-walk shared by the analyser's
//! diagnostics and the LSP's navigation providers.
//!
//! Tcl's command table is mutable at run time.  `rename OLD NEW` moves a
//! command to a new name; `interp alias {} ALIAS {} TARGET` installs a second
//! name that re-resolves to `TARGET` on every invocation (and silently
//! replaces an existing command of that name).  A call site written after
//! either statement therefore reaches a *different* definition than its
//! spelling suggests, and everything that answers "what does this word name?"
//! — the W307/W308 method checks, go-to-definition, find-references, rename,
//! call hierarchy — has to follow the same chain, the same way, or they
//! disagree with each other and with `tclsh`.
//!
//! This module is that single implementation.  It was factored out of
//! `diagnostics::var_command`'s `class_reachable_by_indirection` (issue
//! #1049, PR #1062), which now consumes it, so the navigation providers in
//! `tcl-lsp-core` cannot drift from the diagnostics.
//!
//! # Rules
//!
//! - **Order-gating.**  A hop counts only once the statement that established
//!   it has run: textual order at top level, unconditional inside a
//!   proc/class body (the whole file loads, running every top-level
//!   statement, before any body runs).  Oracle (tclsh 8.6.16 and 9.0.4):
//!   with `proc greet {} {…}`, a `hello` written before `rename greet hello`
//!   raises `invalid command name "hello"`, and one written after returns
//!   `greet`'s body — while `greet` itself then raises `invalid command name
//!   "greet"`.
//! - **Hop cap.**  Eight hops, the same cap the user-call arity resolver
//!   (`diagnostics::validity`) applies to the identical chains, so a
//!   `rename a b; rename b c; …` cycle cannot spin.
//! - **Prepended arguments decline.**  `interp alias {} Cat {} Dog extra`
//!   binds a leading argument, so `Cat …` is not the call `Dog …` would be
//!   (tclsh 8.6.16/9.0.4: `interp alias {} withextra {} target pre` makes
//!   `withextra x` fail `wrong # args: should be "withextra"`).  Such a chain
//!   is declined outright rather than resolved to the target.
//! - **Self-alias decline.**  An alias whose canonical target is its own name
//!   is not a hop.

use std::collections::HashMap;

use super::types::AnalysisResult;

/// Maximum command-name hops [`walk`] follows.
pub const MAX_COMMAND_NAME_HOPS: u8 = 8;

/// Which kind of command-table mutation the last hop of a chain crossed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastHop {
    /// `rename OLD NEW` — the definition keeps its original identity and the
    /// source name is vacated, so a consumer may treat a *rename source* as
    /// still denoting the thing it named.
    Rename,
    /// `interp alias {} ALIAS {} TARGET` — re-resolved by name on every
    /// invocation, so the terminal name must itself be a live command.
    Alias,
}

/// Where a written command name ends up after following the command table's
/// recorded `rename` / `interp alias` indirection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indirection {
    /// The canonical name the chain terminates on.
    pub target: String,
    /// The kind of the final hop — see [`LastHop`].
    pub last_hop: LastHop,
    /// The latest offset among the statements the chain crossed: the whole
    /// chain is in effect at a call site only once *this* offset is
    /// ([`in_effect`]).
    pub established: u32,
}

/// Whether an indirection established at `established` is observably in
/// effect by the time the call at `call_off` runs.
///
/// Order-gated at top level and unconditional inside a definition body — the
/// whole file loads, running every top-level statement, before any body runs,
/// so a rename written after a method that uses it is still in effect when
/// that method executes.  The same rule `diagnostics::validity`'s
/// `fact_in_effect` applies to proc definitions, renames, and aliases.
#[must_use]
pub fn in_effect(result: &AnalysisResult, established: u32, call_off: u32) -> bool {
    established < call_off || result.offset_is_inside_any_definition_body(call_off)
}

/// Follow `written`'s `rename` / `interp alias` chain as observed by a call
/// at `call_off`, returning where it terminates — or `None` when the word
/// names no mutated command, when the chain is not yet in effect, or when it
/// crosses an argument-prepending alias.
///
/// `canonicalise` maps a written name onto the identity the caller keys its
/// own tables by; pass a plain qualifier for command names, or a class-name
/// resolver for the class tables (which is what
/// `diagnostics::var_command::class_reachable_by_indirection` does).  The
/// maps themselves are keyed by the qualified name the scanner resolved, so
/// every lookup normalises through [`crate::naming::normalise_qualified_name`].
///
/// Cost is `O(MAX_COMMAND_NAME_HOPS)` hash lookups — no scan of the
/// invocation list or of the source, so this is safe on a per-request LSP
/// path.
#[must_use]
pub fn walk(
    result: &AnalysisResult,
    written: &str,
    call_off: u32,
    canonicalise: &dyn Fn(&str) -> String,
) -> Option<Indirection> {
    let mut cur = canonicalise(written);
    let mut hopped = false;
    let mut last_hop = LastHop::Rename;
    let mut established = 0u32;
    for _ in 0..MAX_COMMAND_NAME_HOPS {
        let key = crate::naming::normalise_qualified_name(&cur);
        if let Some(old) = result.renamed_commands.get(&key) {
            let at = *result.rename_offsets.get(&key)?;
            if !in_effect(result, at, call_off) {
                return None;
            }
            established = established.max(at);
            cur = canonicalise(old);
            // A rename destination is a live command name in its own right —
            // the definition just keeps its original identity — so the chain
            // is back on the rename-chase rule.
            last_hop = LastHop::Rename;
        } else if let Some(alias) = result.command_aliases.get(&key) {
            if !alias.extras.is_empty() {
                return None;
            }
            let at = *result.alias_offsets.get(&key)?;
            if !in_effect(result, at, call_off) {
                return None;
            }
            let target = canonicalise(&alias.target);
            if target == cur {
                return None;
            }
            established = established.max(at);
            cur = target;
            last_hop = LastHop::Alias;
        } else {
            return hopped.then_some(Indirection {
                target: cur,
                last_hop,
                established,
            });
        }
        hopped = true;
    }
    None
}

/// Every written command name whose indirection chain terminates on
/// `target`, paired with the offset at which the whole chain takes effect.
///
/// The reverse of [`walk`], for consumers that start from a definition rather
/// than from a call site — find-references has to attribute a call spelled
/// through a live alias (`interp alias {} sayHi {} greet` makes `[sayHi]` a
/// call site of `greet`) to the proc it really reaches (issue #923 idx 21).
///
/// Built by walking each recorded alias / rename name once, so the cost is
/// `O((aliases + renames) × MAX_COMMAND_NAME_HOPS)` — bounded by the size of
/// two normally-tiny maps, never by the number of invocations or the size of
/// the tree.  The `call_off` used for the walk's own order gate is
/// [`u32::MAX`] (every fact in effect); the caller re-gates each candidate
/// call site against the returned offset with [`in_effect`], which is what
/// keeps a call written *before* the alias out of the set.
#[must_use]
pub fn names_reaching(
    result: &AnalysisResult,
    target: &str,
    canonicalise: &dyn Fn(&str) -> String,
) -> HashMap<String, u32> {
    let canonical_target = canonicalise(target);
    let mut out = HashMap::new();
    let names = result
        .renamed_commands
        .keys()
        .chain(result.command_aliases.keys());
    for name in names {
        let Some(hop) = walk(result, name, u32::MAX, canonicalise) else {
            continue;
        };
        if hop.target == canonical_target {
            out.insert(name.clone(), hop.established);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{LastHop, names_reaching, walk};
    use crate::analyser::Analyser;
    use crate::analyser::types::AnalysisResult;

    fn analyse(src: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(src, "tcl9.0")
    }

    /// Byte offset just past the last occurrence of `needle`.
    fn after(src: &str, needle: &str) -> u32 {
        u32::try_from(src.rfind(needle).expect("needle present") + needle.len())
            .expect("tiny test source")
    }

    fn qualify(name: &str) -> String {
        crate::naming::normalise_qualified_name(name)
    }

    #[test]
    fn rename_destination_reaches_the_original_name() {
        let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
        let r = analyse(src);
        let hop = walk(&r, "hello", after(src, "hello"), &qualify).expect("hop");
        assert_eq!(hop.target, "::greet");
        assert_eq!(hop.last_hop, LastHop::Rename);
    }

    #[test]
    fn a_rename_written_after_the_call_is_not_in_effect() {
        let src = "proc greet {} { return hi }\nhello\nrename greet hello\n";
        let r = analyse(src);
        let call = u32::try_from(src.find("hello").expect("call")).expect("tiny test source");
        assert!(walk(&r, "hello", call, &qualify).is_none());
    }

    #[test]
    fn an_alias_chain_walks_through_a_rename() {
        // tclsh 8.6.16/9.0.4: `rename Dog Cat; interp alias {} Pup {} Cat`
        // makes `Pup new` build a Dog.
        let src = concat!(
            "proc Dog {args} { }\n",
            "rename Dog Cat\n",
            "interp alias {} Pup {} Cat\n",
            "Pup new\n",
        );
        let r = analyse(src);
        let hop = walk(&r, "Pup", after(src, "Pup new"), &qualify).expect("hop");
        assert_eq!(hop.target, "::Dog");
        // The final hop landed on a rename destination, so the chain ends on
        // the lenient rename-chase rule, not the strict alias one.
        assert_eq!(hop.last_hop, LastHop::Rename);
    }

    #[test]
    fn an_argument_prepending_alias_is_declined() {
        let src = "proc target {a} { }\ninterp alias {} withextra {} target pre\nwithextra\n";
        let r = analyse(src);
        assert!(walk(&r, "withextra", after(src, "withextra\n"), &qualify).is_none());
    }

    #[test]
    fn an_unmutated_name_is_not_a_hop() {
        let src = "proc greet {} { }\ngreet\n";
        let r = analyse(src);
        assert!(walk(&r, "greet", after(src, "greet\n"), &qualify).is_none());
    }

    #[test]
    fn names_reaching_finds_both_hop_kinds() {
        let src = concat!(
            "proc greet {} { }\n",
            "interp alias {} sayHi {} greet\n",
            "rename greet hello\n",
        );
        let r = analyse(src);
        let reaching = names_reaching(&r, "::greet", &qualify);
        let mut names: Vec<&str> = reaching.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["::hello", "::sayHi"]);
    }

    #[test]
    fn names_reaching_is_empty_for_an_unmutated_command() {
        let src = "proc greet {} { }\ngreet\n";
        let r = analyse(src);
        assert!(names_reaching(&r, "::greet", &qualify).is_empty());
    }
}
