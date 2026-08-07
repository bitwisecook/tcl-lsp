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

//! Effective command identity for a command head (issues #1185, #1275).
//!
//! Tcl resolves a command by its interpreter-level *binding*, not by the
//! spelling used to invoke it.  Three statically visible statements move that
//! binding, and this module turns the ones a document states unconditionally
//! at top level into a small, offset-keyed table the source-text consumers
//! consult before they hand a head to the registry:
//!
//! ```tcl
//! namespace import ::tcltest::*   ;# `test`   now *is* `::tcltest::test`
//! interp alias {} myfmt {} format ;# `myfmt`  now *is* `format`
//! rename format origfmt           ;# `origfmt` is `format`, and `format` is gone
//! proc format {args} { … }        ;# `format` is a user proc, not the built-in
//! ```
//!
//! Every fact carries the byte offset of the statement that established it and
//! applies only to heads **at or after** it, so an alias cannot retroactively
//! re-tag an earlier call and a `rename` correctly leaves the calls before it
//! alone.  Verified against tclsh 9.0.4 and 8.6.16 (byte-identical): after
//! `rename format origfmt; proc format {args} {return USER}`, `format x`
//! answers `USER` and `origfmt %d 7` answers `7`.
//!
//! # What this deliberately does not do
//!
//! The table is **sound by abstention** — every shape it cannot prove leaves
//! the head unchanged, and every shape that provably *breaks* a registry
//! binding marks the head [`HeadIdentity::Rebound`] so no registry grammar is
//! applied to it.  Specifically:
//!
//! * **Dynamic heads and dynamic bindings** — `rename $old new`,
//!   `interp alias {} $n {} eval`, `interp alias {} n {} $t` — record nothing
//!   ([`tcl_syntax::naming::is_dynamic_word`] rejects them), so a call through
//!   the name keeps its literal identity rather than gaining a wrong one.
//! * **Pre-bound alias arguments** — `interp alias {} pad {} format %08x`
//!   shifts every argument index, so the layout cannot be reused; the alias
//!   name is marked `Rebound` instead of aliased.
//! * **Another interpreter** — a non-empty `srcPath` (`interp alias slave …`)
//!   binds a name in a *child* interpreter and changes nothing here, so it
//!   records nothing at all; a non-empty `targetPath` points at a command this
//!   document cannot see, so the name is marked `Rebound`.  Hidden commands in
//!   a safe interpreter are likewise invisible: this document's own command
//!   table is what is being described.
//! * **Conditional or nested bindings** — only *top-level* statements are
//!   scanned.  A `rename` inside an `if` body, a proc, an `eval`, or a
//!   `namespace eval` block is not an unconditional document-wide fact, and a
//!   `proc` inside `namespace eval ::n` defines `::n::format`, not `::format`
//!   (tclsh 9.0.4: `::n::format q` → `ns:q`, global `format` untouched).
//! * **`unknown` fallback and traces** — nothing is inferred from them.
//!
//! # Positioned and unpositioned readers
//!
//! [`HeadIdentityMap::resolve`] answers for a head at a known byte offset.
//! Several consumers re-lex a body out of its own decoded text (the formatter
//! reformats `arg.text`, the minifier re-minifies a body slice, the call-graph
//! scan segments a body string at offset 0), so no absolute offset exists at
//! the point of the query.  Those read
//! [`HeadIdentityMap::resolve_unpositioned`], which considers *every* fact
//! about the spelling at once and abstains ([`HeadIdentity::Rebound`]) unless
//! they all agree — a document that binds a name twice cannot be read without
//! a position, and guessing one of the two is exactly the fallback-to-spelling
//! this module exists to remove.

use crate::alias::{detect_interp_alias, detect_interp_alias_delete, detect_rename};
use crate::segmenter::segment_commands_with_offset_and_config;
use rustc_hash::FxHashMap;
use tcl_registry::{CommandRegistry, CommandTableEffect};
use tcl_syntax::naming::is_dynamic_word;

/// What a command head resolves to at one point in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadIdentity<'a> {
    /// The head names this registry command — the head text itself when no
    /// fact applies, or the effective target of a proven import / alias /
    /// rename.
    Command(&'a str),
    /// The head's binding was provably taken over by something the registry
    /// does not model: a `rename` moved the built-in away, a user `proc`
    /// redefined it, or an alias bound it to an unresolvable target.  No
    /// registry grammar applies.
    Rebound,
}

impl<'a> HeadIdentity<'a> {
    /// The name to resolve registry grammar against.
    ///
    /// [`Self::Rebound`] answers with the empty string, which
    /// [`CommandRegistry::get`] never resolves — so every registry query a
    /// consumer already makes (`arg_indices_for_role`, `format_string_args`,
    /// `handle_binding`, …) answers "not a known command" without that
    /// consumer testing the variant.
    #[must_use]
    pub fn spec_name(self) -> &'a str {
        match self {
            Self::Command(name) => name,
            Self::Rebound => "",
        }
    }

    /// Whether the head's registry binding was provably taken over.
    #[must_use]
    pub fn is_rebound(self) -> bool {
        matches!(self, Self::Rebound)
    }
}

/// A command head in its two forms — the spelling the source wrote, and the
/// registry name that spelling effectively resolves to.
///
/// The two are the same for almost every head, and differ once the document
/// rebinds a name.  Which of the two a test must read is a real distinction,
/// not a convenience:
///
/// * a **global command** lookup reads [`Self::resolved`], so `::snit::type`,
///   a proven alias of it, and the bare spelling all answer alike (and a
///   rebound spelling answers nothing);
/// * a **lexical** test reads [`Self::written`] — a class-body member
///   sub-keyword (`method`, `constructor`) or a `$var` head is not a command
///   binding at all, so a top-level `rename method …` says nothing about the
///   word inside an `oo::define`.
#[derive(Debug, Clone, Copy)]
pub struct HeadWords<'a> {
    /// The head exactly as the source spells it.
    pub written: &'a str,
    /// The registry name the spelling resolves to — empty when the head was
    /// provably rebound, which every registry query then answers "unknown" for.
    pub resolved: &'a str,
}

impl<'a> HeadWords<'a> {
    /// A head with nothing proven about it: written and resolved alike.
    #[must_use]
    pub fn plain(written: &'a str) -> Self {
        Self {
            written,
            resolved: written,
        }
    }
}

/// One binding fact about one head spelling.
#[derive(Debug, Clone)]
struct HeadFact {
    /// Byte offset of the statement that established it; the fact applies to
    /// heads at or after this offset only.
    from: u32,
    /// The registry name the head now resolves to, or `None` when the head was
    /// rebound to something unmodelled.
    target: Option<String>,
}

/// Every statically proven command-identity fact in one document, keyed by the
/// head spelling as written.
///
/// Both the bare and the explicitly global spelling of a bound name are
/// recorded, because C Tcl resolves them to the same command
/// (`namespace which -command ::myfmt` → `::myfmt`) and a consumer must not
/// have to strip qualifiers itself.
#[derive(Debug, Default)]
pub struct HeadIdentityMap {
    facts: FxHashMap<String, Vec<HeadFact>>,
}

/// The shared empty map, for a consumer that has no document to scan (an
/// IR-only caller, a unit-test harness).  Nothing is bound, so every head
/// keeps its own spelling.
static NO_IDENTITIES: std::sync::LazyLock<HeadIdentityMap> =
    std::sync::LazyLock::new(HeadIdentityMap::default);

impl HeadIdentityMap {
    /// The empty map — no document, so no binding fact.
    #[must_use]
    pub fn none() -> &'static Self {
        &NO_IDENTITIES
    }

    /// Whether the document stated any binding fact at all — lets a caller
    /// skip per-head lookups entirely for the overwhelmingly common document
    /// that imports, aliases, and renames nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// The effective identity of `head` invoked at byte offset `at`.
    ///
    /// The **latest** fact at or before `at` wins, so a document that renames a
    /// name and then rebinds it again reads correctly at every point between.
    /// With no applicable fact the head keeps its own spelling.
    #[must_use]
    pub fn resolve<'a>(&'a self, head: &'a str, at: u32) -> HeadIdentity<'a> {
        let Some(facts) = self.facts.get(head) else {
            return HeadIdentity::Command(head);
        };
        let Some(fact) = facts.iter().filter(|f| f.from <= at).max_by_key(|f| f.from) else {
            return HeadIdentity::Command(head);
        };
        fact.target
            .as_deref()
            .map_or(HeadIdentity::Rebound, HeadIdentity::Command)
    }

    /// The effective identity of `head` when the call's byte offset is not
    /// available — a body the consumer re-lexed out of its own decoded text.
    ///
    /// Every fact about the spelling is considered at once.  With none, the
    /// head keeps its own spelling; with facts that all name the same target,
    /// that target; otherwise [`HeadIdentity::Rebound`], because the reader
    /// cannot tell which of two bindings is in force and the written spelling
    /// is precisely the answer that must not be assumed.
    #[must_use]
    pub fn resolve_unpositioned<'a>(&'a self, head: &'a str) -> HeadIdentity<'a> {
        let Some(facts) = self.facts.get(head) else {
            return HeadIdentity::Command(head);
        };
        let Some(first) = facts.first() else {
            return HeadIdentity::Command(head);
        };
        if facts.iter().any(|f| f.target != first.target) {
            return HeadIdentity::Rebound;
        }
        first
            .target
            .as_deref()
            .map_or(HeadIdentity::Rebound, HeadIdentity::Command)
    }

    /// `head` in both its forms, resolved at byte offset `at`.
    #[must_use]
    pub fn head_words<'a>(&'a self, head: &'a str, at: u32) -> HeadWords<'a> {
        if self.facts.is_empty() {
            return HeadWords::plain(head);
        }
        HeadWords {
            written: head,
            resolved: self.resolve(head, at).spec_name(),
        }
    }

    /// `head` in both its forms, resolved without a position — see
    /// [`Self::resolve_unpositioned`].
    #[must_use]
    pub fn head_words_unpositioned<'a>(&'a self, head: &'a str) -> HeadWords<'a> {
        if self.facts.is_empty() {
            return HeadWords::plain(head);
        }
        HeadWords {
            written: head,
            resolved: self.resolve_unpositioned(head).spec_name(),
        }
    }

    /// Whether an *earlier* fact already gives `head` a registry identity at
    /// offset `at` — used to notice that a `proc` takes back a name a previous
    /// `rename` / alias had bound to a built-in.
    fn resolves_to_a_command(&self, head: &str, at: u32) -> bool {
        matches!(self.resolve(head, at), HeadIdentity::Command(name) if name != head)
    }

    /// Record `head` → `target` (or a rebinding, with `target` `None`) from
    /// byte offset `from`.
    fn record(&mut self, head: &str, target: Option<String>, from: u32) {
        if head.is_empty() {
            return;
        }
        self.facts
            .entry(head.to_owned())
            .or_default()
            .push(HeadFact { from, target });
    }

    /// Record a fact under both the written spelling and its explicitly global
    /// twin, so `myfmt` and `::myfmt` classify alike.
    fn record_both_spellings(&mut self, name: &str, target: Option<String>, from: u32) {
        let bare = name.strip_prefix("::").unwrap_or(name);
        self.record(bare, target.clone(), from);
        self.record(&format!("::{bare}"), target, from);
    }
}

/// Whether `word` is a name this scan may treat as a static command spelling.
fn is_static_name(word: &str) -> bool {
    !word.is_empty() && !is_dynamic_word(word) && !word.contains(char::is_whitespace)
}

/// Scan `source` for the top-level statements that move a command binding and
/// build the document's [`HeadIdentityMap`].
///
/// Which commands mutate the command table is registry data
/// ([`CommandTableEffect`]) and the argument shapes come from the compiler's
/// own detectors ([`detect_interp_alias`] / [`detect_rename`]) — the same ones
/// the IR-lowering pipeline uses — so no command name is spelled here.
#[must_use]
pub fn command_head_identities(
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
) -> HeadIdentityMap {
    command_head_identities_with_config(
        source,
        tcl_lexer::LexerConfig::for_file_dialect(dialect),
        registry,
    )
}

/// [`command_head_identities`] with an explicit lexer configuration, for a
/// consumer that already holds one (the formatter and the param-trait scan
/// carry a [`tcl_lexer::LexerConfig`] rather than a dialect string).
#[must_use]
pub fn command_head_identities_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
) -> HeadIdentityMap {
    // One top-level segmentation feeds both halves — the `namespace import`
    // scan and the command-table mutators.
    let segments = segment_commands_with_offset_and_config(source, 0, config);
    let mut map = HeadIdentityMap::default();
    for (name, (qualified, offset)) in imported_command_aliases(&segments, registry) {
        map.record(&name, Some(qualified), offset);
    }
    for seg in &segments {
        let Some(head) = seg.texts.first() else {
            continue;
        };
        let args = &seg.texts[1..];
        let Some(effect) = registry.command_table_effect(head, args.first().map(String::as_str))
        else {
            continue;
        };
        let at = seg.argv[0].span.start();
        match effect {
            CommandTableEffect::RenamesCommands => record_rename(&mut map, args, at, registry),
            CommandTableEffect::CreatesAliases => record_alias(&mut map, args, at, registry),
            CommandTableEffect::DefinesProcedure => record_proc(&mut map, args, at, registry),
        }
    }
    map
}

/// `rename OLD NEW` — `NEW` inherits `OLD`'s identity and `OLD` stops existing;
/// `rename OLD {}` deletes `OLD` outright.
fn record_rename(map: &mut HeadIdentityMap, args: &[String], at: u32, registry: &CommandRegistry) {
    // `rename` takes exactly two arguments; any other arity is a `wrong # args`
    // error that moves nothing, so a malformed call must state nothing rather
    // than half a fact.
    if args.len() != 2 {
        return;
    }
    let Some(old) = args.first() else { return };
    if !is_static_name(old) {
        // `rename $old new` — the source is unknown, so neither half of the
        // move can be stated.  Abstain entirely.
        return;
    }
    if let Some((old, new)) = detect_rename(args)
        && is_static_name(&new)
    {
        // Only a source the registry models — directly or through an earlier
        // fact — is worth aliasing; renaming an ordinary user proc leaves `NEW`
        // an ordinary unknown name, which is already what an absent fact
        // produces.  A *provably rebound* source is stated, though: it moves
        // something the registry does not model onto `NEW`, and `NEW` must not
        // then be read under the built-in's grammar.
        let inherited = inherited_target(map, &old, at, registry);
        if inherited.is_some() || map.resolve(&old, at).is_rebound() {
            map.record_both_spellings(&new, inherited, at);
        }
    }
    // Either way the old name is gone from this point on.
    map.record_both_spellings(old, None, at);
}

/// The registry name `source` names at offset `at`, folding in any earlier
/// fact so a **chain** of bindings composes.
///
/// Reading `source` through the map rather than straight off the registry is
/// what makes `interp alias {} a {} format; rename a b` leave `b` naming
/// `format` — the behaviour C Tcl has (tclsh 8.6.16 and 9.0.4, byte-identical:
/// `b %08x 42` answers `0000002a` while `info commands a` answers empty).  The
/// same read is what stops a chain inheriting a *broken* binding:
/// `proc format {…} {…}; rename format myfmt` moves the **user proc**, so
/// `myfmt` must not pick up the built-in's grammar.
///
/// `None` means "names nothing the registry models" — either a provably
/// taken-over spelling or an ordinary unknown name.  The two callers differ in
/// what they do with that, so the distinction stays at the call site.
fn inherited_target(
    map: &HeadIdentityMap,
    source: &str,
    at: u32,
    registry: &CommandRegistry,
) -> Option<String> {
    match map.resolve(source, at) {
        HeadIdentity::Rebound => None,
        HeadIdentity::Command(name) => registry.get(name).map(|_| name.to_owned()),
    }
}

/// `interp alias {} NEW {} TARGET ?arg…?` and its deletion form.
fn record_alias(map: &mut HeadIdentityMap, args: &[String], at: u32, registry: &CommandRegistry) {
    if let Some(deleted) = detect_interp_alias_delete(args) {
        map.record_both_spellings(&deleted, None, at);
        return;
    }
    let Some((qualified, target, prepended)) = detect_interp_alias(args) else {
        // Not a current-interpreter creation: a foreign `srcPath` binds a name
        // in a child interpreter (nothing changes here), and a dynamic name or
        // target cannot be stated.  A *foreign target path* is the one shape
        // worth marking rebound — the alias exists here but runs elsewhere.
        if let (Some(src), Some(name), Some(tgt)) = (args.get(1), args.get(2), args.get(3))
            && matches!(src.as_str(), "" | "{}")
            && !matches!(tgt.as_str(), "" | "{}")
            && is_static_name(name)
        {
            map.record_both_spellings(name, None, at);
        }
        return;
    };
    // Pre-bound arguments shift every index, so the target's layout cannot be
    // reused; a target that names nothing the registry models has no layout to
    // reuse either.  Both cases still *take over* the name — C Tcl lets an
    // alias shadow an existing command outright (tclsh 8.6.16 / 9.0.4:
    // `proc myproc …; interp alias {} lindex {} myproc` makes `lindex {a b c}
    // 1` answer `MINE`) — so the name is marked rebound rather than left alone.
    // The target is read through the map, so a chain of bindings composes.
    let effective = prepended
        .is_empty()
        .then(|| inherited_target(map, &target, at, registry))
        .flatten();
    map.record_both_spellings(&qualified, effective, at);
}

/// `proc NAME …` at top level — a user proc that shadows a registry built-in
/// takes the name over (tclsh 9.0.4: after `proc format {args} {return USER}`,
/// `format x` is `USER`).
///
/// Only a name that *would otherwise* resolve to a registry command is
/// recorded — either directly, or through an earlier fact (a `proc origfmt`
/// after `rename format origfmt` takes the moved built-in's name back).  Every
/// other `proc` leaves the head an ordinary unknown name, which is already what
/// an absent fact produces.
fn record_proc(map: &mut HeadIdentityMap, args: &[String], at: u32, registry: &CommandRegistry) {
    let Some(name) = args.first() else { return };
    if !is_static_name(name) || name.contains("::") {
        // A qualified `proc ::ns::format` defines a *different* command; only
        // the global-namespace shadow is stated here.
        return;
    }
    if registry.get(name).is_some() || map.resolves_to_a_command(name, at) {
        map.record_both_spellings(name, None, at);
    }
}

/// Scan `source` for `namespace import` declarations and map each bare command
/// name they bring into the global scope to its qualified registry spec name
/// (`test` → `tcltest::test`, issue #776).
///
/// Recognises the two literal forms — `namespace import EXPORTING::*`
/// (import-all) and `namespace import EXPORTING::name` (single) — matched
/// against the registry's `is_namespace_exported` commands in the exporting
/// namespace.  A bare name that already resolves to a global command is left
/// alone (Tcl's own `namespace import` refuses to shadow an existing command
/// without `-force`, and we must not mis-resolve a genuine builtin).  This is a
/// highlighting-only convenience: it never changes which commands exist, only
/// lets the registry-driven argument overrides see the real spec for an
/// unqualified imported command.  Returns an empty map when nothing is imported.
fn imported_command_aliases(
    segments: &[crate::segmenter::SegmentedCommand],
    registry: &CommandRegistry,
) -> FxHashMap<String, (String, u32)> {
    let mut aliases: FxHashMap<String, (String, u32)> = FxHashMap::default();
    // Wholesale imports (`ns::*`) and single-name imports (`ns::name`), each
    // tagged with the byte offset of its `namespace import` statement so an
    // alias only applies to heads at or after it (source order).  Only
    // top-level imports are seen — a `namespace import` nested inside a
    // `namespace eval` body is not a top-level segment, so it never leaks a
    // global bare alias.
    let mut import_all: Vec<(String, u32)> = Vec::new();
    let mut import_one: Vec<(String, String, u32)> = Vec::new();
    for seg in segments {
        if seg.texts.len() < 3 || seg.texts[0] != "namespace" || seg.texts[1] != "import" {
            continue;
        }
        let import_off = seg.argv[0].span.start();
        for pat in &seg.texts[2..] {
            // Skip option flags (`-force`); computed patterns are left alone.
            if pat.starts_with('-') {
                continue;
            }
            if let Some(ns) = pat.strip_suffix("::*") {
                import_all.push((ns.trim_start_matches(':').to_string(), import_off));
            } else if let Some((ns, name)) = pat.rsplit_once("::") {
                import_one.push((
                    ns.trim_start_matches(':').to_string(),
                    name.to_string(),
                    import_off,
                ));
            }
        }
    }
    if import_all.is_empty() && import_one.is_empty() {
        return aliases;
    }
    // Record `name → (qualified, offset)`, keeping the *earliest* enabling
    // import when several would produce the same alias.
    let mut record = |name: String, qualified: String, off: u32| {
        aliases
            .entry(name)
            .and_modify(|(_, o)| *o = (*o).min(off))
            .or_insert((qualified, off));
    };
    // Import-all: every exported command in an imported namespace whose bare
    // tail does not already name a global command.
    if !import_all.is_empty() {
        for name in registry.command_names() {
            let Some((ns, tail)) = name.rsplit_once("::") else {
                continue;
            };
            if tail.is_empty() || registry.get(tail).is_some() {
                continue;
            }
            let ns = ns.trim_start_matches(':');
            let Some(off) = import_all
                .iter()
                .filter(|(n, _)| n == ns)
                .map(|(_, o)| *o)
                .min()
            else {
                continue;
            };
            if registry.get(name).is_some_and(|s| s.is_namespace_exported) {
                record(tail.to_string(), name.to_string(), off);
            }
        }
    }
    // Single-name imports.
    for (ns, name, off) in &import_one {
        if registry.get(name).is_some() {
            continue;
        }
        let qualified = format!("{ns}::{name}");
        if registry
            .get(&qualified)
            .is_some_and(|s| s.is_namespace_exported)
        {
            record(name.clone(), qualified, *off);
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_for(src: &str) -> HeadIdentityMap {
        let registry = tcl_registry::registry_for_dialect("tcl");
        command_head_identities(src, "tcl", registry)
    }

    /// Offset just past the first line of `src`, i.e. "after statement 1".
    fn after_first_line(src: &str) -> u32 {
        u32::try_from(src.find('\n').map_or(src.len(), |i| i + 1)).unwrap_or(0)
    }

    #[test]
    fn static_rename_moves_the_identity_and_clears_the_old_name() {
        let src = "rename format origfmt\norigfmt %d 7\nformat x\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("origfmt", at), HeadIdentity::Command("format"));
        // The `::`-qualified spelling of the new name resolves alike.
        assert_eq!(
            map.resolve("::origfmt", at),
            HeadIdentity::Command("format")
        );
        // The old name is gone from the rename onwards …
        assert_eq!(map.resolve("format", at), HeadIdentity::Rebound);
        // … but calls *before* it still see the built-in.
        let src = "format {%08x} 42\nrename format origfmt\n";
        let map = map_for(src);
        assert_eq!(map.resolve("format", 0), HeadIdentity::Command("format"));
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            HeadIdentity::Rebound
        );
    }

    #[test]
    fn interp_alias_aliases_only_the_argument_preserving_form() {
        let src = "interp alias {} myfmt {} format\n";
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(map.resolve("myfmt", at), HeadIdentity::Command("format"));
        assert_eq!(map.resolve("::myfmt", at), HeadIdentity::Command("format"));

        // Pre-bound arguments shift every index — the layout cannot be reused.
        let src = "interp alias {} pad {} format %08x\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("pad", after_first_line(src)),
            HeadIdentity::Rebound
        );
    }

    #[test]
    fn dynamic_bindings_abstain() {
        for src in [
            "rename $old myfmt\n",
            "interp alias {} $n {} format\n",
            "interp alias {} myfmt {} $t\n",
        ] {
            let map = map_for(src);
            let at = after_first_line(src);
            assert_eq!(
                map.resolve("myfmt", at),
                HeadIdentity::Command("myfmt"),
                "dynamic binding must not state a fact: {src}"
            );
            // A dynamic `rename` must not claim the *old* name is gone either.
            assert_eq!(map.resolve("format", at), HeadIdentity::Command("format"));
        }
    }

    #[test]
    fn a_child_interpreter_alias_changes_nothing_here() {
        let src = "interp alias slave myfmt {} format\n";
        let map = map_for(src);
        assert!(map.is_empty(), "a foreign srcPath states no fact here");
    }

    /// FP guard — a **safe / sub-interpreter** command table is not this
    /// document's own, so hiding or exposing a command there states nothing
    /// about a bare call here.
    ///
    /// tclsh-proof (9.0.4 / 8.6.16): `interp create -safe s; interp hide s
    /// format` leaves the parent's `format %d 7` answering `7`, while
    /// `s eval {format %d 7}` fails `invalid command name "format"`.
    #[test]
    fn a_safe_interpreters_hidden_command_states_nothing_here() {
        for src in [
            "interp create -safe s\n",
            "interp hide s format\n",
            "interp expose s format\n",
            "interp invokehidden s format %d 7\n",
        ] {
            let map = map_for(src);
            assert!(
                map.is_empty(),
                "a child interpreter's command table must state nothing here: {src}"
            );
        }
    }

    /// TN — a malformed call states nothing rather than half a fact.
    #[test]
    fn malformed_binding_calls_abstain() {
        for src in [
            // Arity too short for a rename / an alias creation.
            "rename\n",
            "rename format\n",
            "interp alias\n",
            "interp alias {}\n",
            // Not the alias subcommand at all (`aliases` merely queries).
            "interp aliases {}\n",
            // `proc` with no name.
            "proc\n",
        ] {
            let map = map_for(src);
            assert_eq!(
                map.resolve("format", u32::MAX),
                HeadIdentity::Command("format"),
                "a malformed binding must not disturb `format`: {src}"
            );
        }
    }

    #[test]
    fn a_foreign_target_path_rebinds_without_aliasing() {
        let src = "interp alias {} myfmt slave format\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("myfmt", after_first_line(src)),
            HeadIdentity::Rebound
        );
    }

    #[test]
    fn a_top_level_proc_shadows_the_builtin_it_names() {
        let src = "proc format {args} { return USER }\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", after_first_line(src)),
            HeadIdentity::Rebound
        );
        // A proc that shadows nothing states nothing.
        let src = "proc mything {args} { return 1 }\n";
        assert!(map_for(src).is_empty());
        // A qualified proc defines a different command entirely.
        let src = "proc ::ns::format {args} { return 1 }\n";
        assert!(map_for(src).is_empty());
    }

    #[test]
    fn a_nested_binding_is_not_a_document_wide_fact() {
        // Neither the conditional rename nor the namespaced proc is an
        // unconditional top-level statement.
        for src in [
            "if {$x} { rename format origfmt }\n",
            "namespace eval ::n { proc format {a} { return 1 } }\n",
            "proc p {} { rename format origfmt }\n",
            "eval { rename format origfmt }\n",
            "uplevel #0 { rename format origfmt }\n",
            "interp eval $child { rename format origfmt }\n",
        ] {
            assert!(map_for(src).is_empty(), "nested binding leaked: {src}");
        }
    }

    #[test]
    fn a_qualified_mutator_head_states_the_same_fact() {
        // C Tcl resolves `::rename` to `::rename` — the mutator's own spelling
        // must not be a false negative either (issue #1185).
        let src = "::rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            HeadIdentity::Command("format")
        );
    }

    #[test]
    fn the_latest_applicable_fact_wins() {
        let src = "rename format origfmt\norigfmt %d 7\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        let after_all = u32::try_from(src.len()).unwrap_or(0);
        // Between the `rename` and the `proc`, `origfmt` *is* the built-in …
        assert_eq!(
            map.resolve("origfmt", after_first_line(src)),
            HeadIdentity::Command("format")
        );
        // … and after the `proc` takes the name back it is a user command,
        // even though `origfmt` is not itself a registry name.
        assert_eq!(map.resolve("origfmt", after_all), HeadIdentity::Rebound);
    }

    /// Issue #1275's "chained bindings do not compose" residual.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical):
    ///
    /// ```tcl
    /// interp alias {} a {} format ; rename a b ; b %08x 42   ;# 0000002a
    /// info commands a                                        ;# (empty)
    /// rename lindex li1 ; rename li1 li2 ; li2 {x y z} 2      ;# z
    /// rename lsort mysort ; interp alias {} sorter {} mysort
    /// sorter {c a b}                                          ;# a b c
    /// ```
    #[test]
    fn chained_bindings_compose() {
        // alias → rename
        let src = "interp alias {} a {} format\nrename a b\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("b", end), HeadIdentity::Command("format"));
        assert_eq!(map.resolve("::b", end), HeadIdentity::Command("format"));
        // The intermediate name is gone once the rename moves it.
        assert_eq!(map.resolve("a", end), HeadIdentity::Rebound);

        // rename → rename
        let src = "rename lindex li1\nrename li1 li2\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("li2", end), HeadIdentity::Command("lindex"));

        // rename → alias
        let src = "rename lsort mysort\ninterp alias {} sorter {} mysort\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("sorter", end), HeadIdentity::Command("lsort"));
    }

    /// A chain must not inherit a *broken* binding: the rename moves the user
    /// proc, not the built-in it shadowed, so the new name gets no grammar.
    #[test]
    fn a_chain_through_a_shadowed_builtin_stays_rebound() {
        let src = "proc format {args} { return USER }\nrename format myfmt\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("myfmt", end), HeadIdentity::Rebound);
        assert_eq!(map.resolve("::myfmt", end), HeadIdentity::Rebound);
    }

    /// An alias whose target is an ordinary user proc still *takes over* the
    /// name it binds — C Tcl lets an alias shadow an existing command
    /// (tclsh 8.6.16 / 9.0.4: `proc myproc …; interp alias {} lindex {} myproc`
    /// makes `lindex {a b c} 1` answer `MINE`).
    #[test]
    fn an_alias_over_a_builtin_rebinds_it() {
        let src = "proc myproc {args} { return MINE }\ninterp alias {} lindex {} myproc\n";
        let map = map_for(src);
        let end = u32::try_from(src.len()).unwrap_or(0);
        assert_eq!(map.resolve("lindex", end), HeadIdentity::Rebound);
    }

    #[test]
    fn an_unpositioned_read_abstains_when_the_facts_disagree() {
        // One fact — the unpositioned read matches the positioned one.
        let src = "rename format origfmt\n";
        let map = map_for(src);
        assert_eq!(
            map.resolve_unpositioned("origfmt"),
            HeadIdentity::Command("format")
        );
        assert_eq!(map.resolve_unpositioned("format"), HeadIdentity::Rebound);
        // A head nothing binds keeps its own spelling.
        assert_eq!(
            map.resolve_unpositioned("lindex"),
            HeadIdentity::Command("lindex")
        );

        // Two facts that disagree — without a position, neither can be chosen.
        let src = "rename format origfmt\nproc origfmt {args} { return 1 }\n";
        let map = map_for(src);
        assert_eq!(map.resolve_unpositioned("origfmt"), HeadIdentity::Rebound);
    }

    #[test]
    fn the_shared_empty_map_binds_nothing() {
        let map = HeadIdentityMap::none();
        assert!(map.is_empty());
        assert_eq!(map.resolve("format", 0), HeadIdentity::Command("format"));
        assert_eq!(
            map.resolve_unpositioned("format"),
            HeadIdentity::Command("format")
        );
    }

    #[test]
    fn a_rebound_head_resolves_to_no_registry_spec() {
        // The `Rebound` sentinel must be unresolvable, since that is what makes
        // every registry query answer "unknown" without a variant check.
        let registry = tcl_registry::registry_for_dialect("tcl");
        assert!(registry.get(HeadIdentity::Rebound.spec_name()).is_none());
    }
}
