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
//! # The model
//!
//! A command *name* is not a definition; it is a slot whose contents change
//! over the life of the script.  Every statement that writes such a slot —
//! `proc`, `rename`, `interp alias` — is an event on that name's timeline, and
//! the question every consumer really asks is *"what does this name hold at
//! this point, in this execution context?"*.  This module answers it for the
//! two mutation kinds; [`AnalysisResult::proc_def_in_effect_at`] answers the
//! `proc` half of the same question, under the same order rules.
//!
//! # Rules
//!
//! - **Order-gating.**  A hop counts only once the statement that established
//!   it has run: textual order at top level, and — for a statement written
//!   *outside* the body now executing — unconditionally inside a proc/class
//!   body, because the whole file loads, running every top-level statement,
//!   before any body runs.  A mutation that is itself a statement *of that
//!   same body* is an ordinary statement of the running script and stays
//!   order-gated by offset ([`in_effect`]).  Oracle (tclsh 8.6.14/9.0.4):
//!   with `proc greet {} {…}`, a `hello` written before `rename greet hello`
//!   raises `invalid command name "hello"`, and one written after returns
//!   `greet`'s body — while `greet` itself then raises `invalid command name
//!   "greet"`.
//! - **Latest binding wins.**  A name may carry both a `rename` record and an
//!   `interp alias` record; the one written later is the one the slot holds
//!   ([`latest_binding`]).  Oracle (8.6.14/9.0.4): `proc a {} {return A};
//!   proc b {} {return B}; rename a x; interp alias {} x {} b; x` → `B` — the
//!   alias replaced the renamed-in command.  (The reverse order is not a
//!   reachable program: `rename a x` onto a live `x` aborts with `can't
//!   rename to "x": command already exists`, so the code after it never runs;
//!   the same latest-wins rule is applied there for want of a reachable
//!   alternative.)
//! - **A rename moves the command object, an alias re-resolves by name.**
//!   `rename p oldp` hands `oldp` the *object* `p` held at the rename, so a
//!   later `proc p` does not change what `oldp` runs — oracle (8.6.14/9.0.4):
//!   `proc p {} {return first}; rename p oldp; proc p {} {return second}` has
//!   `oldp` → `first` (and `oldp`'s arity is the *first* signature:
//!   `wrong # args: should be "oldp a"` for `proc p {a}`), while `p` →
//!   `second`.  An `interp alias`, by contrast, looks its target up by name on
//!   every invocation, so it sees the table as it stands *at the call*.
//!   [`Indirection::resolve_at`] carries whichever of the two as-of times
//!   applies, and consumers resolve the terminal name at that time.
//! - **Hop cap.**  Eight hops, the same cap the user-call arity resolver
//!   (`diagnostics::validity`) applies to the identical chains, so a
//!   `rename a b; rename b c; …` cycle cannot spin.
//! - **Prepended arguments decline.**  `interp alias {} Cat {} Dog extra`
//!   binds a leading argument, so `Cat …` is not the call `Dog …` would be
//!   (tclsh 8.6.14/9.0.4: `interp alias {} withextra {} target pre` makes
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
    /// The point in the document at which [`Self::target`]'s own definition
    /// must be resolved — the *as-of* time the chain's last hop fixed.
    ///
    /// A `rename` hands over the command **object**, so the terminal name has
    /// to be read as the table stood *at the rename*: after `proc p {} {return
    /// first}; rename p oldp; proc p {} {return second}`, `oldp` still runs the
    /// first definition (oracle in the module docs), and resolving `::p`
    /// against the final proc table would answer with the second.  An
    /// `interp alias` re-resolves its target by name on every invocation, so
    /// there the as-of time is the call site's own offset and this field
    /// carries the `call_off` the walk was given.
    pub resolve_at: u32,
}

/// Whether an indirection established at `established` is observably in
/// effect by the time the call at `call_off` runs.
///
/// Order-gated by offset, with one exception: a statement written *outside*
/// the definition body that is executing at `call_off` has already run,
/// because the whole file loads — running every top-level statement — before
/// any body runs, so a rename written after a method that uses it is still in
/// effect when that method executes.
///
/// The exception does **not** extend to a mutation that is itself a statement
/// of that same body: there it is an ordinary statement of the running script
/// and the offsets are in genuine execution order (oracle in
/// [`AnalysisResult::proc_def_in_effect_at`], whose `proc` timeline obeys the
/// identical rule).  Bodies nest, so "that same body" means the innermost
/// recorded body containing `call_off`; a mutation sitting in some *other*
/// proc's body stays leniently in effect, since whether that proc ever runs is
/// not statically decidable.
#[must_use]
pub fn in_effect(result: &AnalysisResult, established: u32, call_off: u32) -> bool {
    if established < call_off {
        return true;
    }
    result
        .innermost_definition_body_span(call_off)
        .is_some_and(|body| !(body.start() <= established && established < body.end()))
}

/// The binding a command name's slot holds, as of `as_of` — the later of its
/// `rename` and `interp alias` records among those already in effect.
///
/// Both maps may carry the same key: `rename a x` then `interp alias {} x {}
/// b` leaves `x` with a rename record *and* an alias record, and only the
/// offsets say which one the slot actually holds (oracle in the module docs:
/// the alias wins, because it ran last).  Reading one map before the other —
/// as this walk originally did — silently prefers whichever kind the code
/// happened to check first.
fn latest_binding<'a>(result: &'a AnalysisResult, key: &str, as_of: u32) -> Option<Binding<'a>> {
    let rename = result
        .renamed_commands
        .get(key)
        .zip(result.rename_offsets.get(key))
        .map(|(old, &at)| Binding {
            kind: LastHop::Rename,
            source: old.as_str(),
            at,
            prepends_args: false,
        });
    let alias = result
        .command_aliases
        .get(key)
        .zip(result.alias_offsets.get(key))
        .map(|(alias, &at)| Binding {
            kind: LastHop::Alias,
            source: alias.target.as_str(),
            at,
            prepends_args: !alias.extras.is_empty(),
        });
    [rename, alias]
        .into_iter()
        .flatten()
        .filter(|b| in_effect(result, b.at, as_of))
        .max_by_key(|b| b.at)
}

/// One command-table binding as [`latest_binding`] resolved it.
struct Binding<'a> {
    kind: LastHop,
    /// Where the name's contents came from: the rename's `OLD` word, or the
    /// alias's target command.
    source: &'a str,
    /// Offset of the statement that installed it.
    at: u32,
    /// Whether an alias binds leading arguments (never true for a rename).
    prepends_args: bool,
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
    // The as-of time each further hop is read at.  A rename freezes it to the
    // rename's own offset (the object handed over is the one the source name
    // held *then*); an alias releases it back to the call, since it re-resolves
    // its target by name every time it fires.
    let mut as_of = call_off;
    for _ in 0..MAX_COMMAND_NAME_HOPS {
        let key = crate::naming::normalise_qualified_name(&cur);
        let Some(binding) = latest_binding(result, &key, as_of) else {
            return hopped.then_some(Indirection {
                target: cur,
                last_hop,
                established,
                resolve_at: as_of,
            });
        };
        if binding.prepends_args {
            return None;
        }
        let source = canonicalise(binding.source);
        if source == cur {
            return None;
        }
        established = established.max(binding.at);
        as_of = match binding.kind {
            LastHop::Rename => binding.at,
            LastHop::Alias => call_off,
        };
        cur = source;
        // A rename destination is a live command name in its own right — the
        // definition just keeps its original identity — so a chain ending
        // there is back on the rename-chase rule.
        last_hop = binding.kind;
        hopped = true;
    }
    None
}

/// How a name found by [`names_reaching`] reaches the queried definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reaching {
    /// The offset from which the whole chain is in effect — a call site
    /// earlier than this is not reached by it ([`in_effect`]).
    pub established: u32,
    /// The as-of time at which the terminal name's definition must be read to
    /// decide *which* declaration of it this chain actually captured
    /// ([`Indirection::resolve_at`]).
    ///
    /// `None` when the chain's final hop was an alias: an alias re-resolves
    /// its target by name at every invocation, so the as-of time is each call
    /// site's own offset rather than one fixed point.
    pub resolve_at: Option<u32>,
}

/// Every written command name whose indirection chain terminates on
/// `target`, paired with how it gets there ([`Reaching`]).
///
/// The reverse of [`walk`], for consumers that start from a definition rather
/// than from a call site — find-references has to attribute a call spelled
/// through a live alias (`interp alias {} sayHi {} greet` makes `[sayHi]` a
/// call site of `greet`) to the proc it really reaches (issue #923 idx 21).
///
/// The name alone is not the answer: when the terminal name has been
/// redeclared, only *one* of its declarations is the one the chain captured,
/// so [`Reaching::resolve_at`] is carried back for the caller to check the
/// identity against ([`AnalysisResult::proc_def_in_effect_at`]).
///
/// Built by walking each recorded alias / rename name once, so the cost is
/// `O((aliases + renames) × MAX_COMMAND_NAME_HOPS)` — bounded by the size of
/// two normally-tiny maps, never by the number of invocations or the size of
/// the tree.  The `call_off` used for the walk's own order gate is
/// [`u32::MAX`] (every fact in effect); the caller re-gates each candidate
/// call site against [`Reaching::established`] with [`in_effect`], which is
/// what keeps a call written *before* the alias out of the set.
#[must_use]
pub fn names_reaching(
    result: &AnalysisResult,
    target: &str,
    canonicalise: &dyn Fn(&str) -> String,
) -> HashMap<String, Reaching> {
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
            out.insert(
                name.clone(),
                Reaching {
                    established: hop.established,
                    resolve_at: match hop.last_hop {
                        LastHop::Rename => Some(hop.resolve_at),
                        LastHop::Alias => None,
                    },
                },
            );
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

    /// tclsh 9.0.4 and 8.6.14: `proc a …; proc b …; rename a x; interp alias
    /// {} x {} b; x` prints `B`.  Both records key on `::x`; only the offsets
    /// say which one the slot holds.
    #[test]
    fn the_later_of_a_rename_and_an_alias_on_one_name_wins() {
        let src = concat!(
            "proc a {} { return A }\n",
            "proc b {} { return B }\n",
            "rename a x\n",
            "interp alias {} x {} b\n",
            "x\n",
        );
        let r = analyse(src);
        let hop = walk(&r, "x", after(src, "x\n"), &qualify).expect("hop");
        assert_eq!(hop.target, "::b");
        assert_eq!(hop.last_hop, LastHop::Alias);
    }

    /// The same document read between the two mutations: only the rename has
    /// run, so the earlier binding is still the live one.
    #[test]
    fn the_earlier_binding_governs_until_the_later_one_runs() {
        let src = concat!(
            "proc a {} { return A }\n",
            "proc b {} { return B }\n",
            "rename a x\n",
            "x\n",
            "interp alias {} x {} b\n",
        );
        let r = analyse(src);
        let call = u32::try_from(src.find("\nx\n").expect("call") + 1).expect("tiny test source");
        let hop = walk(&r, "x", call, &qualify).expect("hop");
        assert_eq!(hop.target, "::a");
        assert_eq!(hop.last_hop, LastHop::Rename);
    }

    /// A rename freezes the as-of time at the rename itself: `oldp` holds the
    /// object `p` had *then*, so a later `proc p` cannot change it (oracle:
    /// `oldp` → `first`, `p` → `second` on 9.0.4 and 8.6.14).
    #[test]
    fn a_rename_hop_resolves_the_target_as_of_the_rename() {
        let src = concat!(
            "proc p {} { return first }\n",
            "rename p oldp\n",
            "proc p {} { return second }\n",
            "oldp\n",
        );
        let r = analyse(src);
        let hop = walk(&r, "oldp", after(src, "oldp\n"), &qualify).expect("hop");
        assert_eq!(hop.target, "::p");
        let rename_off = u32::try_from(src.find("rename").expect("rename")).expect("tiny");
        assert!(
            hop.resolve_at >= rename_off && hop.resolve_at < after(src, "rename p oldp"),
            "as-of time is the rename statement, not the call site"
        );
        let captured = r
            .proc_def_in_effect_at("::p", hop.resolve_at)
            .expect("captured definition");
        assert_eq!(
            captured.name_span.start(),
            u32::try_from(src.find("p {}").expect("first header")).expect("tiny"),
            "the chain captured the first declaration"
        );
    }

    /// An alias looks its target up by name on every invocation, so its as-of
    /// time is the call site, not the alias statement.
    #[test]
    fn an_alias_hop_resolves_the_target_as_of_the_call() {
        let src = concat!(
            "proc greet {} { }\n",
            "interp alias {} sayHi {} greet\n",
            "sayHi\n",
        );
        let r = analyse(src);
        let call = after(src, "sayHi\n");
        let hop = walk(&r, "sayHi", call, &qualify).expect("hop");
        assert_eq!(hop.last_hop, LastHop::Alias);
        assert_eq!(hop.resolve_at, call);
    }

    /// A mutation that is a statement of the body now executing is ordinary
    /// script order — the load-before-body shortcut stops at the body's edge.
    #[test]
    fn a_same_body_mutation_below_the_call_is_not_in_effect() {
        let src = concat!(
            "proc a {} { return A }\n",
            "proc outer {} {\n",
            "    x\n",
            "    rename a x\n",
            "}\n",
        );
        let r = analyse(src);
        let call = u32::try_from(src.find("    x\n").expect("call") + 4).expect("tiny");
        assert!(walk(&r, "x", call, &qualify).is_none());
    }

    /// …but a mutation in some *other* body stays leniently in effect: whether
    /// that body ever runs is not statically decidable.
    #[test]
    fn a_mutation_in_another_body_is_still_in_effect() {
        let src = concat!(
            "proc a {} { return A }\n",
            "proc installer {} {\n",
            "    rename a x\n",
            "}\n",
            "proc caller {} {\n",
            "    x\n",
            "}\n",
        );
        let r = analyse(src);
        let call = u32::try_from(src.rfind("    x\n").expect("call") + 4).expect("tiny");
        let hop = walk(&r, "x", call, &qualify).expect("hop");
        assert_eq!(hop.target, "::a");
    }
}
