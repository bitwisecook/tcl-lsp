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

//! `spectcl_check` and `spectcl_expand` — the AI author's two views of a
//! `.tclspec` spec pack (design E §15.3, E-R13).
//!
//! The CLI-side `tcl spec check` / `tcl spec export` of
//! `docs/design/spec-packs.md`, exposed to an agent instead of a terminal: a
//! pack's source text in, a machine-readable account of what the loader made
//! of it out, and — for a pack written as a *program* — its expansion back
//! out as canonical source. They are what the `spec-author` skill calls to
//! validate a pack it has just written, and to read what its own loop
//! actually registered.
//!
//! **There is exactly one loader** — and since the `one-loader` lane that is
//! true of the tree, not just of this module. Every fact below comes from
//! [`tcl_spectcl::evaluate_pack`], which is now the only door from `.tclspec`
//! text to a `Pack`: the LSP's discovery and reload path reaches the same
//! function through `tcl_spectcl::evaluate_pack_cached` (the cache, not a
//! second reading), and so do the Spec Studio and the `tcl` CLI. Facts also
//! come from the [`Draft`] the studio seeds from that result through the
//! ordinary [`draft::from_command_spec`], so a field this report calls "set"
//! is set in exactly the sense the Spec Studio's form means. This module reads that result and renders it;
//! it never looks at the pack text itself, with the single, documented
//! exception of the hook-body `ctx` scan under
//! [`declaration_conflict`](fn@declaration_conflict), which inspects body text
//! the loader deliberately carries verbatim.
//!
//! Cacheability and that scan are deliberately kept apart. Whether a hook is
//! cached is decided by its `-inputs` declaration and nothing else, because
//! that is all the engine consults; the body scan answers the different
//! question of whether the body keeps the promise the declaration made.
//! Folding the two together — reporting a tame-looking body as cached, or a
//! declared-and-cached hook as uncached because its body looks broad —
//! describes an engine that does not exist.
//!
//! Four things are reported:
//!
//! 1. **Commands** — the name, and which draft fields the pack actually set,
//!    computed by diffing the pack's draft against
//!    [`draft::default_command_draft`] (see [`fields_set`](fn@fields_set) for
//!    what "set" means, hooks included). The list answers "what did this
//!    declaration actually say" rather than "which keys exist", which is how a
//!    silently-dropped property becomes visible: it is simply absent.
//! 2. **Notices** — every [`Notice`] the loader raised, as
//!    `{line, context, reason}`. A notice is always a *degradation*: the named
//!    declaration was dropped and the rest of the pack loaded. An unknown word
//!    surfaces here and nowhere else, which is what makes this report the way
//!    to find a typo'd trait or a property this server is too old to know.
//! 3. **Hooks** — every declared hook with its family, what it hangs off,
//!    whether it is **shape-cacheable** (see below), and whether its body
//!    contradicts its own declaration.
//! 4. **Collisions** — commands whose name the shipped registry already
//!    defines for the target dialect. Shipped wins unless the declaration says
//!    `-override`, so an unintended collision is a silently dead command.
//!
//! ## A note on cost
//!
//! Loading a pack installs it for the
//! process's life by leaking its strings and specs — a `CommandSpec` is a
//! `&'static`-shaped record. Checking a pack therefore leaks it, by design of
//! the loader rather than of this tool. That is fine for an MCP server driven
//! by an authoring session (a few packs per session, kilobytes each) and is
//! noted here so nobody wires this handler into a hot loop. Evaluation adds
//! a bounded interpreter run on top: the budgets are the only thing standing
//! between a generated pack's loop and this process, and they are what make
//! running one safe.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};
use tcl_registry::CommandRegistry;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::spec::CommandSpec;
use tcl_spec_studio::draft::{self, Draft, UNRENDERABLE_KEY};
use tcl_spectcl::{HookDecl, HookFamily, HookOwner, HookSource, PackCommand, Tier, evaluate_pack};

/// Every key the hook calling convention puts in `ctx`, exactly as
/// `tcl_spec_hooks`'s `ctx_value` builds it. A `dict get $ctx` on anything
/// else reads empty forever.
///
/// The option-arity trio is present only on an option row's call, but a body
/// naming one is asking for a key the convention defines, so it is not a typo.
const CTX_KEYS: &[&str] = &[
    "command",
    "subcommand",
    "nwords",
    "kinds",
    "tcl-version",
    "dialect",
    "in-event-body",
    "option",
    "option-index",
    "option-value-start",
];

/// Whether reading `key` leaves a body inside the shape key.
///
/// Asked of the engine rather than answered from a list kept here: the same
/// [`HookInputs`] that decides caching at `allocate` time decides it here, so
/// the two cannot drift. They had drifted in both directions — this tool
/// called `tcl-version` and `in-event-body` uncacheable when `ShapeKey`
/// carries both, and called the option trio cacheable when it carries
/// neither.
fn ctx_key_in_shape(key: &str) -> bool {
    HookInputs::parse(&[key]).shape_only()
}

/// The MCP `spectcl_check` handler.
///
/// The pack is **evaluated** (design E §1), not walked: a program written by
/// a model runs in the deterministic sandbox — no clock, no IO, no network,
/// hard budgets, transactional registration — so checking an untrusted
/// generated pack is safe by construction, and a runaway `foreach` comes
/// back as a budget notice rather than a hung tool.
///
/// A straight-line declarative pack reports exactly what it always did: the
/// golden gate (`tcl-spectcl/tests/golden_packs.rs`) holds every shipped pack
/// to its checked-in snapshot and notices. What evaluation adds is what only
/// evaluation can see — [`load_error`](tcl_spectcl::LoadError) for the four transactional
/// failures (a determinism denial naming its axis, a blown budget naming its
/// axis, a Tcl error, a provenance refusal), the target-dependence flag, and
/// the workspace-tier provenance verdict below.
pub fn spectcl_check(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let dialect = crate::tools::declared_dialect(args);
    let registry = tcl_spectcl::bundled::registry_for_dialect(&dialect);

    let pack = evaluate_pack(source);

    let defaults = draft::default_command_draft();
    let sub_defaults = draft::default_subcommand_draft();

    let commands: Vec<Value> = pack
        .commands
        .iter()
        .map(|c| command_json(c, &defaults, &sub_defaults))
        .collect();
    let mut notices: Vec<Value> = pack
        .notices
        .iter()
        .map(|n| {
            json!({
                "line": n.line,
                "context": n.context,
                "class": n.class.name(),
                "reason": n.message,
            })
        })
        .collect();
    // The provenance verdict the *author's* tier does not raise. Checking
    // evaluates a pack as trusted, because the file is the author's own; a
    // pack lands in a workspace or a Spec Studio override, where E-R2 refuses
    // a `-override` on a compiled name, a `dialect` block, or a reserved
    // `environment` name outright. Reporting it here is the difference
    // between finding that out now and finding it out when the pack silently
    // fails to load in an editor.
    let provenance = tcl_spectcl::provenance_violation(&pack, Tier::Workspace);
    if let Some((line, message)) = &provenance {
        notices.push(json!({
            "line": line,
            "context": "pack",
            "class": "provenance",
            "reason": message,
        }));
    }
    let collisions: Vec<Value> = pack
        .commands
        .iter()
        .filter_map(|c| collision_json(c, &registry, &dialect))
        .collect();

    let hook_count: usize = pack.commands.iter().map(|c| c.hooks.len()).sum();
    let uncacheable = pack
        .commands
        .iter()
        .flat_map(|c| c.hooks.iter())
        .filter(|h| shape_cacheability(h).0 == Some(false))
        .count();
    let conflicts = pack
        .commands
        .iter()
        .flat_map(|c| c.hooks.iter())
        .filter(|h| declaration_conflict(h, &CtxScan::of(h)).is_some())
        .count();
    let shadowed = collisions
        .iter()
        .filter(|c| c["effect"] == "shipped-spec-wins")
        .count();
    let notice_count = notices.len();

    json!({
        "pack": pack.name,
        "dsl_version": pack.dsl_version,
        "dialect": dialect,
        "commands": commands,
        "notices": notices,
        "collisions": collisions,
        "load_error": pack.load_error.as_ref().map(ToString::to_string),
        "target_dependent": pack.target_dependent,
        "untrusted_tier_refusal": provenance.as_ref().map(|(_, message)| message.clone()),
        "summary": {
            "commands": pack.commands.len(),
            "notices": notice_count,
            "hooks": hook_count,
            "uncacheable_hooks": uncacheable,
            "declaration_conflicts": conflicts,
            "collisions": collisions.len(),
            "shadowed_commands": shadowed,
        },
    })
}

// ── Expand ────────────────────────────────────────────────────────────

/// The MCP `spectcl_expand` handler — a pack's snapshot as canonical source.
///
/// The affordance that makes a *programmed* pack reviewable. A model that
/// writes a pack as a loop cannot see what the loop registered without
/// simulating it in its head, which is exactly the opacity design E's frozen
/// snapshot exists to prevent (§1.1). This evaluates the pack and writes the
/// registrations back as straight-line declarations, so the author reads the
/// expansion as a diff against intent and iterates.
///
/// Expansion is total; contraction is never attempted — the program is not
/// recovered from its expansion, and re-exporting a canonical pack is a
/// no-op.
pub fn spectcl_expand(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let pack = evaluate_pack(source);
    let (canonical, losses) = tcl_spectcl::export_pack_reporting(&pack);
    // The same formatter profile the studio's editors and `tcl spec export`
    // use, so a pack reads the same however it was produced.
    let canonical = tcl_spec_studio::format_pack(&canonical);

    json!({
        "pack": pack.name,
        "dsl_version": pack.dsl_version,
        "canonical_source": canonical,
        "target_dependent": pack.target_dependent,
        "load_error": pack.load_error.as_ref().map(ToString::to_string),
        "commands": pack
            .commands
            .iter()
            .map(|c| c.spec.name)
            .collect::<Vec<_>>(),
        "notices": pack
            .notices
            .iter()
            .map(|n| json!({
                "line": n.line,
                "context": n.context,
                "class": n.class.name(),
                "reason": n.message,
            }))
            .collect::<Vec<_>>(),
        "unwritable": losses
            .iter()
            .map(|loss| json!({
                "word": loss.word,
                "line": loss.line,
                "text": loss.text,
            }))
            .collect::<Vec<_>>(),
    })
}

// ── Commands ──────────────────────────────────────────────────────────

/// One command: its name, the draft fields the declaration set, its
/// subcommands, and its hooks.
fn command_json(cmd: &PackCommand, defaults: &Draft, sub_defaults: &Draft) -> Value {
    let d = draft::from_command_spec(cmd.spec);
    let subcommands: Vec<Value> = d
        .get("subcommands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|sub| {
            json!({
                "name": sub.get("name").cloned().unwrap_or(Value::Null),
                "fields_set": fields_set(sub, sub_defaults),
            })
        })
        .collect();
    let hooks: Vec<Value> = cmd.hooks.iter().map(hook_json).collect();
    let families: BTreeSet<&str> = cmd.hooks.iter().map(|h| family_key(h.family)).collect();

    json!({
        "name": cmd.spec.name,
        "override": cmd.overrides_shipped,
        "fields_set": fields_set(&d, defaults),
        "unrenderable": d.get(UNRENDERABLE_KEY).cloned().unwrap_or(Value::Null),
        "subcommands": subcommands,
        "clause_grammar": cmd.clause_grammar.is_some(),
        "hook_families": families,
        "hooks": hooks,
    })
}

/// The draft keys this declaration actually said something about, sorted.
///
/// Two kinds, because the draft model has two ways of saying "set":
///
/// - a key whose **value** differs from a brand-new command's, and
/// - a key under [`UNRENDERABLE_KEY`], which is precisely the list of fields
///   that *are* set but whose defining Rust expression could not be recovered
///   — every function-pointer field, so every hook. Without these a declared
///   hook would read as unset, which is the opposite of true.
///
/// `name` is excluded because it is reported separately and always differs.
///
/// **One roll-up to know about**: the draft model bubbles a subcommand's
/// unrenderable keys into its command's list, so a hook declared only on a
/// subcommand also appears in the command's `fields_set`. The per-hook `owner`
/// in `hooks` is the authoritative answer to *where* a hook hangs.
fn fields_set(d: &Map<String, Value>, defaults: &Draft) -> Vec<String> {
    let mut set: BTreeSet<String> = d
        .iter()
        .filter(|(key, _)| key.as_str() != "name" && key.as_str() != UNRENDERABLE_KEY)
        .filter(|(key, value)| defaults.get(key.as_str()) != Some(*value))
        .map(|(key, _)| key.to_owned())
        .collect();
    set.extend(
        d.get(UNRENDERABLE_KEY)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
    set.into_iter().collect()
}

// ── Hooks ─────────────────────────────────────────────────────────────

/// The schema spelling of a hook family — the word an author writes.
fn family_key(family: HookFamily) -> &'static str {
    match family {
        HookFamily::ArgRoleResolver => "arg_role_resolver",
        HookFamily::CommandPrefixResolver => "command_prefix_resolver",
        HookFamily::ScriptTimingResolver => "script_timing_resolver",
        HookFamily::ConstFold => "const_fold",
        HookFamily::ConstFoldVersioned => "const_fold_versioned",
        HookFamily::TaintSinkGate => "taint_sink_gate",
        HookFamily::ContextGate => "context_gate",
        HookFamily::LiteralArgumentValidator => "literal_argument_validator",
        HookFamily::ClauseShapeCheck => "clause_shape_check",
        HookFamily::Constraints => "constraints",
        HookFamily::OptionArity => "-arity-hook",
    }
}

/// What the hook hangs off, as one readable phrase.
fn owner_label(owner: &HookOwner) -> String {
    match owner {
        HookOwner::Command => "command".to_owned(),
        HookOwner::Subcommand(name) => format!("subcommand {name}"),
        HookOwner::Option { subcommand, option } => match subcommand {
            Some(sub) => format!("subcommand {sub} option {option}"),
            None => format!("option {option}"),
        },
    }
}

fn hook_json(hook: &HookDecl) -> Value {
    let (kind, detail) = match &hook.source {
        HookSource::Body { params, .. } => ("body", params.join(" ")),
        HookSource::Native { id } => ("native", id.clone()),
        HookSource::Derived { keyword } => ("derived", keyword.clone()),
    };
    let scan = CtxScan::of(hook);
    let (cacheable, reason) = shape_cacheability(hook);
    json!({
        "owner": owner_label(&hook.owner),
        "field": hook.field,
        "family": family_key(hook.family),
        "source": kind,
        "detail": detail,
        "ctx_keys": &scan.keys,
        "unknown_ctx_keys": scan.unknown(),
        "shape_cacheable": cacheable,
        "cache_reason": reason,
        "declaration_conflict": declaration_conflict(hook, &scan),
        "verbs": hook.family.verbs(),
        "silence_means": hook.family.silence(),
        "requires_all_literal": hook.family.requires_all_literal(),
    })
}

/// What a hook body reads out of its `ctx` dict.
#[derive(Debug, Default)]
struct CtxScan {
    /// Literal keys the body reads through `dict get $ctx KEY`.
    keys: BTreeSet<String>,
    /// Whether some `$ctx` reference was *not* one of those reads, so the
    /// read set is not fully known.
    unattributed: bool,
}

impl CtxScan {
    /// Scan a hook's body. A native or derived hook has none, and reads
    /// nothing.
    fn of(hook: &HookDecl) -> Self {
        match &hook.source {
            HookSource::Body { body, .. } => ctx_reads(body),
            HookSource::Native { .. } | HookSource::Derived { .. } => Self::default(),
        }
    }

    /// Keys outside [`CTX_KEYS`] — words the calling convention does not
    /// define, which will read as empty every time. Always a pack bug, and
    /// invisible without this report because the loader carries a body
    /// verbatim rather than checking inside it.
    fn unknown(&self) -> Vec<&str> {
        self.keys
            .iter()
            .map(String::as_str)
            .filter(|k| !CTX_KEYS.contains(k))
            .collect()
    }

    /// The keys read that lie outside the `(command, word-shape)` memo key.
    fn broader(&self) -> Vec<&str> {
        self.keys
            .iter()
            .map(String::as_str)
            .filter(|k| CTX_KEYS.contains(k) && !ctx_key_in_shape(k))
            .collect()
    }
}

/// Whether a hook's answer can be memoised by `(command, word-shape)` — the
/// **declared-inputs rule** of `docs/design/spec-packs.md`'s hot-path budget.
///
/// A hook that depends only on its declared inputs and answers the whole call
/// site in one invocation is cacheable at 24.5 ns/call, indistinguishable from
/// native. One that declares broader dependencies stays fully legal and costs
/// ~28 µs per call site, every call site — so the rule is reported, not
/// enforced.
///
/// Returns `(Some(true) | Some(false) | None, reason)`. `None` is
/// *not applicable*: a native or derived hook runs no VM body at all, so the
/// question does not arise for it.
///
/// ### How "declared inputs" is decided
///
/// `words` and every [`SHAPE_CTX_KEYS`] entry are inside the shape key;
/// [`BROADER_CTX_KEYS`] are outside it. A body is cacheable when it reads no
/// key of the second kind — with two conservative refusals:
///
/// - `const_fold_versioned` takes `tcl-version` *by family*, whether or not
///   the body spells it, so the whole family is uncacheable.
/// - A `$ctx` reference the scan cannot attribute to a literal key (a computed
///   key, or `ctx` passed onward) means the read set is unknown, and unknown
///   is not provably inside the shape.
///
/// The scan is textual and therefore may only ever be *pessimistic*: an
/// unattributed reference downgrades the answer, never upgrades it.
fn shape_cacheability(hook: &HookDecl) -> (Option<bool>, String) {
    let HookSource::Body { inputs, .. } = &hook.source else {
        return (
            None,
            "no VM body — native and derived hooks run at native cost".to_owned(),
        );
    };
    // The declaration decides, and *only* the declaration, because that is
    // what the runtime does: `allocate` stores `HookInputs::shape_only` into
    // the slot's cacheable bit and `dispatch` consults nothing else. A body
    // scan cannot move this answer in either direction without describing an
    // engine that does not exist — what the scan finds is reported as a
    // conflict instead, by [`declaration_conflict`].
    if !inputs.shape_only() {
        return (
            Some(false),
            "declares no `-inputs`, or declares one outside the (command, \
             word-shape) key, so the runtime never caches it — the default, \
             and always correct"
                .to_owned(),
        );
    }
    (
        Some(true),
        "declares only inputs the shape key carries, so its answer memoises by \
         (command, word-shape)"
            .to_owned(),
    )
}

/// Where a body contradicts its own `-inputs` declaration.
///
/// A declaration is a promise the engine acts on: it arms the shape cache and
/// drops the `words` parameter. Nothing checks the body against it — the
/// loader carries a body verbatim — so a body that reads more than it declared
/// is served another call's answer whenever the shapes match. That is the one
/// way a pack can be silently miscompiled, and this report is the only place
/// it is visible before it happens.
///
/// Reading word *content* is not listed here: a shape-only hook is compiled
/// without the `words` parameter, so the sandbox raises on the first call and
/// the host quarantines the hook. Loud already.
///
/// The scan is textual, so it may only ever be *suspicious*, never certain —
/// an unattributable `$ctx` is reported as unproven rather than as a fault.
fn declaration_conflict(hook: &HookDecl, scan: &CtxScan) -> Option<String> {
    let HookSource::Body { inputs, .. } = &hook.source else {
        return None;
    };
    if !inputs.shape_only() {
        // Nothing was promised that the body could break: an unrestricted or
        // words-reading hook may read whatever it likes, and is not cached.
        return None;
    }
    let broader = scan.broader();
    if !broader.is_empty() {
        return Some(format!(
            "declares only shape inputs but reads {} from `ctx`, which the \
             shape key does not carry — the cache will serve one call's answer \
             to another that differs only in {}",
            broader.join(", "),
            broader.join(" / ")
        ));
    }
    scan.unattributed.then(|| {
        "declares only shape inputs and uses `$ctx` in a way this check cannot \
         attribute to a literal key, so the declaration cannot be confirmed \
         against the body"
            .to_owned()
    })
}

/// Scan a hook body for `dict get $ctx KEY`.
///
/// Textual on purpose: the pack itself is parsed exactly once, by the loader,
/// which carries a hook body through as verbatim text because the sandbox that
/// will eventually run it does not exist yet. Until it does there is no
/// compiled form to interrogate, and a heuristic that can only be pessimistic
/// is the honest amount of machinery for a report.
fn ctx_reads(body: &str) -> CtxScan {
    let mut scan = CtxScan::default();
    let mut attributed = 0usize;
    let mut total = 0usize;

    let bytes = body.as_bytes();
    let mut i = 0;
    while let Some(at) = body[i..].find("$ctx").map(|p| p + i) {
        let after = at + "$ctx".len();
        // `$ctxfoo` is a different variable, not a `ctx` reference.
        let is_ctx = bytes.get(after).is_none_or(|b| !is_name_byte(*b));
        i = after;
        if !is_ctx {
            continue;
        }
        total += 1;
        if let Some(key) = dict_get_key(body, at, after) {
            attributed += 1;
            scan.keys.insert(key);
        }
    }
    // `${ctx}` — the braced spelling reads the same variable, and is never
    // part of the `dict get $ctx KEY` shape this scan recognises.
    total += body.matches("${ctx}").count();

    scan.unattributed = attributed < total;
    scan
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// The literal key of a `dict get $ctx KEY` whose `$ctx` starts at `at` and
/// ends at `after`, when the reference is exactly that shape.
fn dict_get_key(body: &str, at: usize, after: usize) -> Option<String> {
    // Preceded by `dict get`, with a whitespace run before `$ctx` and another
    // between the two words.
    let head = body[..at].trim_end_matches(char::is_whitespace);
    if head.len() == at {
        return None; // `get$ctx` is not a `dict get`
    }
    let head = head.strip_suffix("get")?;
    if !head.ends_with(char::is_whitespace) {
        return None;
    }
    let head = head
        .trim_end_matches(char::is_whitespace)
        .strip_suffix("dict")?;
    // `dict` must itself start a word, so `subdict get` is not a match.
    if head
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        return None;
    }
    // Followed by whitespace and a bare word — the key.
    let rest = body.get(after..)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let word = rest.trim_start();
    let end = word
        .find(|c: char| c.is_whitespace() || c == ']' || c == '}')
        .unwrap_or(word.len());
    let key = &word[..end];
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| key.to_owned())
}

// ── Registry collisions ───────────────────────────────────────────────

/// Resolve a name the way the **target dialect** resolves it.
///
/// The mask comes from the threaded dialect name through the one ingress
/// seam ([`crate::environment::analyser_point_for_dialect`]) rather than
/// being read back off the generation's profile *stamp*, so this no longer
/// depends on the stamp surviving. Same answer: the store handed in here is
/// the one built for that same name.
///
/// T6 (P2): the *payload* this ledger row retires is the bit test itself —
/// a collision is properly `targets ⊆ applicable` over the declaration's
/// version sets, not a mask intersection. That is a model change, not a
/// port, and stays open.
fn shipped<'r>(
    registry: &'r CommandRegistry,
    name: &str,
    dialect: &str,
) -> Option<&'r CommandSpec> {
    registry.get_for_surface(
        name,
        Some(crate::environment::analyser_point_for_dialect(dialect)),
    )
}

/// A warning when the target dialect's shipped registry already defines this
/// command name.
///
/// Shipped wins unless the declaration says `-override`
/// (`docs/design/spec-packs.md`, "Loading and tooling"), so a collision without
/// `-override` means the pack's command never reaches a query — the failure
/// worth catching before a user reports "my spec does nothing".
fn collision_json(cmd: &PackCommand, registry: &CommandRegistry, dialect: &str) -> Option<Value> {
    let shipped = shipped(registry, cmd.spec.name, dialect)?;
    let (effect, message) = if cmd.overrides_shipped {
        (
            "pack-spec-wins",
            format!(
                "`{}` is already shipped for {dialect}; the declaration says \
                 `-override`, so the pack's spec replaces it",
                cmd.spec.name
            ),
        )
    } else {
        (
            "shipped-spec-wins",
            format!(
                "`{}` is already shipped for {dialect}; the shipped spec wins \
                 and this declaration has no effect — rename it, or add \
                 `-override` if replacing it is intended",
                cmd.spec.name
            ),
        )
    };
    Some(json!({
        "command": cmd.spec.name,
        "dialect": dialect,
        "override": cmd.overrides_shipped,
        "effect": effect,
        "shipped_subcommands": shipped.subcommands.len(),
        "message": message,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::dispatch;

    fn check(source: &str, dialect: &str) -> Value {
        dispatch(
            "spectcl_check",
            &json!({ "source": source, "dialect": dialect }),
        )
        .expect("spectcl_check tool")
    }

    fn strings(value: &Value) -> Vec<&str> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect()
    }

    /// A pack whose every word the loader knows: no notices, the declared
    /// fields show up as set, and the hook is reported with its family.
    const VALID: &str = r"
speclib mylib 2.1 {
    command mylib::with_var {
        arity 2..3
        arg 0 -role VarWrite
        arg 1 -role Body
        return_type String
        arg_role_resolver -inputs {nwords} {ctx} {
            if {[dict get $ctx nwords] >= 2} {
                role 1 Body
            }
        }
        arg_role_resolver_roles Body
    }
}
";

    #[test]
    fn a_valid_pack_reports_its_commands_and_no_notices() {
        let result = check(VALID, "tcl9.0");
        assert_eq!(result["pack"], "mylib", "{result}");
        assert_eq!(result["dsl_version"], "2.1");
        assert_eq!(result["dialect"], "tcl9.0");
        assert_eq!(result["notices"], json!([]), "{result}");
        assert_eq!(result["summary"]["commands"], 1);
        assert_eq!(result["collisions"], json!([]));

        let command = &result["commands"][0];
        assert_eq!(command["name"], "mylib::with_var");
        let set = strings(&command["fields_set"]);
        for field in [
            "arity",
            "arg_roles",
            "return_type",
            "arg_role_resolver",
            "arg_role_resolver_roles",
        ] {
            assert!(set.contains(&field), "{field} missing from {command}");
        }
        // A field the pack never mentioned stays at its default and is absent.
        assert!(!set.contains(&"traits"), "{command}");
    }

    /// The one hook declares `nwords` and reads it out of `ctx`: every input
    /// it named is carried by the shape key, so the engine caches it.
    #[test]
    fn a_shape_only_resolver_body_is_shape_cacheable() {
        let result = check(VALID, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["family"], "arg_role_resolver", "{hook}");
        assert_eq!(hook["owner"], "command");
        assert_eq!(hook["source"], "body");
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
        assert_eq!(hook["verbs"], json!(["role"]));
        assert_eq!(result["summary"]["uncacheable_hooks"], 0);
        assert_eq!(result["summary"]["declaration_conflicts"], 0);
        assert_eq!(
            strings(&result["commands"][0]["hook_families"]),
            vec!["arg_role_resolver"]
        );
    }

    #[test]
    fn script_timing_resolver_is_reported_by_its_schema_family() {
        let source = r"
speclib timed 1.2 {
    command timed::schedule {
        arity 1
        arg 0 -role Body
        script_timing_resolver {words ctx} { timing 0 Deferred }
    }
}
";
        let result = check(source, "tcl9.0");
        assert_eq!(result["notices"], json!([]), "{result}");
        assert_eq!(
            strings(&result["commands"][0]["hook_families"]),
            vec!["script_timing_resolver"]
        );
        assert_eq!(
            result["commands"][0]["hooks"][0]["family"],
            "script_timing_resolver"
        );
    }

    /// A body that declares nothing may read anything, and is never cached —
    /// the safe default, and the reason a pack pays 28 µs per call site until
    /// it opts in. The keys it reads are still reported.
    #[test]
    fn an_undeclared_body_is_uncacheable_whatever_it_reads() {
        let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate {words ctx} {
            if {![dict get $ctx in-event-body]} return
            reject {mylib::guarded is only valid in an event body}
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["family"], "context_gate", "{hook}");
        assert_eq!(hook["ctx_keys"], json!(["in-event-body"]), "{hook}");
        assert_eq!(hook["shape_cacheable"], json!(false), "{hook}");
        assert!(
            hook["cache_reason"]
                .as_str()
                .unwrap_or_default()
                .contains("-inputs"),
            "{hook}"
        );
        // Nothing was promised, so nothing is contradicted.
        assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
        assert_eq!(hook["silence_means"], "the call is allowed");
        assert_eq!(result["summary"]["uncacheable_hooks"], 1);
    }

    /// `in-event-body` and `tcl-version` are *in* the shape key — `ShapeKey`
    /// carries both — so declaring and reading them is cacheable. This is the
    /// pair this tool used to call uncacheable while the engine cached them.
    #[test]
    fn the_shape_key_extends_past_the_words() {
        let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate -inputs {in-event-body tcl-version} {ctx} {
            if {[dict get $ctx in-event-body]} return
            if {[dict get $ctx tcl-version] eq {8.4}} return
            reject {mylib::guarded is only valid in an event body}
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
        assert_eq!(result["summary"]["uncacheable_hooks"], 0);
    }

    /// The one silent miscompile a pack can cause: the declaration arms the
    /// cache, the body reads something the key does not carry, and nothing at
    /// runtime checks one against the other. `dialect` is outside the key, so
    /// two documents of different dialects share an answer.
    #[test]
    fn a_body_reading_past_its_declaration_is_flagged_as_a_conflict() {
        let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate -inputs {nwords} {ctx} {
            if {[dict get $ctx dialect] eq {irules}} return
            reject {mylib::guarded is iRules-only}
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        // The engine caches it — saying otherwise would describe an engine
        // that does not exist.
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        let conflict = hook["declaration_conflict"].as_str().unwrap_or_default();
        assert!(conflict.contains("dialect"), "{hook}");
        assert_eq!(result["summary"]["declaration_conflicts"], 1, "{result}");
    }

    /// A `-native` hook has no VM body, so cacheability does not apply to it.
    #[test]
    fn a_native_hook_is_not_a_cacheability_question() {
        let source = r"
speclib mylib 1.0 {
    command mylib::native {
        arity 1
        const_fold -native Concat
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["source"], "native", "{hook}");
        assert_eq!(hook["detail"], "Concat");
        assert_eq!(hook["shape_cacheable"], Value::Null, "{hook}");
        assert_eq!(result["summary"]["uncacheable_hooks"], 0);
    }

    /// Unknown words are dropped with a notice each, and the rest of the
    /// command still loads — the tolerance rule, made visible.
    #[test]
    fn unknown_words_surface_as_notices_without_losing_the_command() {
        let source = r"
speclib mylib 1.0 {
    command mylib::typo {
        arity 1..2
        traits {NOT_PROC_FACTORY WOBBLY_TRAIT}
        frobnicate yes
        return_type String
    }
}
";
        let result = check(source, "tcl9.0");
        let notices = result["notices"].as_array().expect("notices");
        assert!(notices.len() >= 2, "{result}");
        for notice in notices {
            assert_eq!(notice["context"], "command mylib::typo", "{notice}");
            assert!(notice["line"].as_u64().unwrap_or(0) > 0, "{notice}");
            assert!(
                !notice["reason"].as_str().unwrap_or_default().is_empty(),
                "{notice}"
            );
        }
        let reasons: Vec<&str> = notices
            .iter()
            .filter_map(|n| n["reason"].as_str())
            .collect();
        assert!(
            reasons.iter().any(|r| r.contains("WOBBLY_TRAIT")),
            "{reasons:?}"
        );
        assert!(
            reasons.iter().any(|r| r.contains("frobnicate")),
            "{reasons:?}"
        );

        // The command survived, with its known fields intact.
        let command = &result["commands"][0];
        assert_eq!(command["name"], "mylib::typo");
        let set = strings(&command["fields_set"]);
        assert!(set.contains(&"arity"), "{command}");
        assert!(set.contains(&"return_type"), "{command}");
        assert!(set.contains(&"traits"), "{command}");
    }

    #[test]
    fn contradictory_option_semantics_surface_as_degradation_notices() {
        let source = r"
speclib mylib 1.2 {
    command mylib::bad {
        option -timing -takes value -role Value -script-timing Deferred
        option -input -takes script -role Body -script-timing SameInvocation \
            -callback-taint-inputs {%P}
        option -scope -takes value -role Value -variable-scope Global
        option -taint -takes variable -role VarRead -taints-var-write
    }
}
";
        let result = check(source, "tcl9.0");
        let reasons: Vec<&str> = result["notices"]
            .as_array()
            .expect("notices")
            .iter()
            .filter_map(|notice| notice["reason"].as_str())
            .collect();
        for fragment in [
            "non-executable value",
            "not a deferred executable",
            "non-variable value",
            "without a VarWrite role",
        ] {
            assert!(
                reasons.iter().any(|reason| reason.contains(fragment)),
                "missing `{fragment}`: {result}"
            );
        }
        assert_eq!(result["summary"]["notices"], 4, "{result}");
        assert_eq!(result["summary"]["commands"], 1, "{result}");
    }

    /// Redeclaring a shipped name without `-override` is the silent-no-op
    /// case, and the report says so.
    #[test]
    fn a_shipped_name_collides_and_the_pack_spec_loses() {
        let source = r"
speclib mylib 1.0 {
    command lsort {
        arity 1..
    }
}
";
        let result = check(source, "tcl9.0");
        let collision = &result["collisions"][0];
        assert_eq!(collision["command"], "lsort", "{result}");
        assert_eq!(collision["dialect"], "tcl9.0");
        assert_eq!(collision["override"], json!(false));
        assert_eq!(collision["effect"], "shipped-spec-wins");
        assert!(
            collision["message"]
                .as_str()
                .unwrap_or_default()
                .contains("-override"),
            "{collision}"
        );
        assert_eq!(result["summary"]["shadowed_commands"], 1);
    }

    /// The same collision with `-override` is a deliberate replacement, not a
    /// dead declaration.
    #[test]
    fn an_override_collision_says_the_pack_spec_wins() {
        let source = r"
speclib mylib 1.0 {
    command lsort -override {
        arity 1..
    }
}
";
        let result = check(source, "tcl9.0");
        let collision = &result["collisions"][0];
        assert_eq!(collision["effect"], "pack-spec-wins", "{result}");
        assert_eq!(collision["override"], json!(true));
        assert_eq!(result["summary"]["shadowed_commands"], 0);
        assert_eq!(result["summary"]["collisions"], 1);
    }

    /// A dialect-only command collides under its own dialect and not under
    /// plain Tcl — the collision check is per target dialect, not global.
    #[test]
    fn collisions_are_scoped_to_the_target_dialect() {
        let source = r"
speclib mylib 1.0 {
    command HTTP::uri {
        arity 0..1
    }
}
";
        let irules = check(source, "f5-irules");
        assert_eq!(irules["summary"]["collisions"], 1, "{irules}");
        let plain = check(source, "tcl9.0");
        assert_eq!(plain["summary"]["collisions"], 0, "{plain}");
    }

    /// Subcommands are reported with their own set-field lists.
    #[test]
    fn subcommands_report_their_own_fields() {
        let source = r"
speclib mylib 1.0 {
    command mylib::ensemble {
        arity 1..
        subcommand indices {
            arity 1
            return_type List
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let sub = &result["commands"][0]["subcommands"][0];
        assert_eq!(sub["name"], "indices", "{result}");
        let set = strings(&sub["fields_set"]);
        assert!(set.contains(&"arity"), "{sub}");
        assert!(set.contains(&"return_type"), "{sub}");
    }

    /// `$ctx` the scan cannot attribute to a literal key leaves a declaration
    /// unconfirmable. The scan is textual, so it reports the doubt rather than
    /// claiming a fault — and never touches the cacheability answer, which the
    /// declaration alone decides.
    #[test]
    fn an_unattributable_ctx_use_leaves_the_declaration_unconfirmed() {
        let source = r"
speclib mylib 1.0 {
    command mylib::dynamic {
        arity 1
        arg_role_resolver -inputs {nwords} {ctx} {
            set key command
            if {[dict get $ctx $key] eq {}} return
            role 0 Body
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        assert!(
            hook["declaration_conflict"]
                .as_str()
                .unwrap_or_default()
                .contains("attribute"),
            "{hook}"
        );
    }

    /// A `ctx` key the calling convention does not define reads as empty
    /// forever. The loader carries a body verbatim and never looks inside it,
    /// so this report is the only place that typo shows up.
    #[test]
    fn a_ctx_key_outside_the_calling_convention_is_flagged() {
        let source = r"
speclib mylib 1.0 {
    command mylib::typo {
        arity 1
        arg_role_resolver -inputs {nwords} {ctx} {
            if {[dict get $ctx subcomand] eq {}} return
            role 0 Body
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["unknown_ctx_keys"], json!(["subcomand"]), "{hook}");
        // Reading an undefined key is a pack bug, not a caching cost: an
        // always-empty read is a function of nothing at all, so it neither
        // blocks the cache nor contradicts the declaration.
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
    }

    // ── The evaluation-only classes (design E §15.3) ──────────────────

    /// A pack that reaches for a clock fails closed, and the report names
    /// the determinism axis rather than describing a half-loaded pack.
    #[test]
    fn a_determinism_denial_is_a_load_error_naming_its_axis() {
        let result = check(
            "speclib clocky 2.0 {\n    command fine { arity 1 }\n    \
             set now [clock seconds]\n}\n",
            "tcl9.0",
        );
        let error = result["load_error"].as_str().unwrap_or_default();
        assert!(error.contains("determinism axis: clock/time"), "{result}");
        // Registration is transactional: the command declared *before* the
        // denial is not loaded either.
        assert_eq!(result["summary"]["commands"], 0, "{result}");
        let reasons: Vec<&str> = result["notices"]
            .as_array()
            .expect("notices")
            .iter()
            .filter_map(|n| n["reason"].as_str())
            .collect();
        assert!(
            reasons.iter().any(|r| r.contains("transactional")),
            "{reasons:?}"
        );
    }

    /// A runaway loop is a notice, not a hung tool — which is the whole
    /// reason an AI-generated program can be checked at all.
    #[test]
    fn a_runaway_loop_is_a_budget_notice_naming_its_axis() {
        let result = check(
            "speclib hungry 2.0 {\n    command fine { arity 1 }\n    \
             set i 0\n    while {1} { incr i }\n}\n",
            "tcl9.0",
        );
        let error = result["load_error"].as_str().unwrap_or_default();
        assert!(
            error.contains("budget")
                && (error.contains("command steps") || error.contains("wall clock")),
            "{result}"
        );
        assert_eq!(result["summary"]["commands"], 0, "{result}");
    }

    /// `available?` makes the answer depend on the analysis target, and the
    /// report says so — the E-R1 trap, named where an author will see it.
    #[test]
    fn a_target_dependent_pack_is_flagged_with_its_notice() {
        let result = check(
            "speclib trap 2.0 {\n    default available {tcl 8.6-}\n    \
             command base { arity 1 }\n    if {[available? {tcl 8.6-}]} {\n        \
             command extra { arity 1 }\n    }\n}\n",
            "tcl9.0",
        );
        assert_eq!(result["target_dependent"], json!(true), "{result}");
        assert_eq!(result["summary"]["commands"], 2, "{result}");
        let reasons: Vec<&str> = result["notices"]
            .as_array()
            .expect("notices")
            .iter()
            .filter_map(|n| n["reason"].as_str())
            .collect();
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("target-dependent registration")),
            "{reasons:?}"
        );
    }

    /// The provenance class: the pack loads for its author and would be
    /// refused from a workspace, and the report says which and why.
    #[test]
    fn a_workspace_tier_refusal_is_reported_without_failing_the_check() {
        let result = check(
            "speclib sneaky 2.0 {\n    command lsort -override {\n        arity 1..\n    }\n}\n",
            "tcl9.0",
        );
        // The check itself still reports the command and its collision.
        assert_eq!(result["summary"]["commands"], 1, "{result}");
        assert_eq!(result["load_error"], Value::Null, "{result}");
        let refusal = result["untrusted_tier_refusal"]
            .as_str()
            .unwrap_or_default();
        assert!(refusal.contains("design E-R2"), "{result}");
        assert!(refusal.contains("workspace"), "{result}");
        let classes: Vec<&str> = result["notices"]
            .as_array()
            .expect("notices")
            .iter()
            .filter_map(|n| n["class"].as_str())
            .collect();
        assert!(classes.contains(&"provenance"), "{result}");
    }

    /// A pack that touches nothing reserved carries no refusal.
    #[test]
    fn an_ordinary_pack_carries_no_tier_refusal() {
        let result = check(VALID, "tcl9.0");
        assert_eq!(result["untrusted_tier_refusal"], Value::Null, "{result}");
        assert_eq!(result["target_dependent"], json!(false), "{result}");
        assert_eq!(result["load_error"], Value::Null, "{result}");
    }

    // ── spectcl_expand ────────────────────────────────────────────────

    fn expand(source: &str) -> Value {
        dispatch("spectcl_expand", &json!({ "source": source })).expect("spectcl_expand tool")
    }

    const PROGRAM: &str = r"speclib fleet 2.0 {
    proc fleet-command {name arity} {
        command math::fleet::$name {
            arity $arity
            traits {PURE}
            option -verbose -detail {Report each step as it runs.}
        }
    }
    foreach {name arity} {alpha 2 beta 1 gamma 3} {
        fleet-command $name $arity
    }
}
";

    /// The affordance: a loop in, one literal declaration per iteration out.
    #[test]
    fn expand_writes_a_programmed_packs_registrations_as_canonical_source() {
        let result = expand(PROGRAM);
        assert_eq!(result["pack"], "fleet", "{result}");
        assert_eq!(
            result["commands"],
            json!([
                "math::fleet::alpha",
                "math::fleet::beta",
                "math::fleet::gamma"
            ]),
            "{result}"
        );
        assert_eq!(result["target_dependent"], json!(false));
        assert_eq!(result["load_error"], Value::Null);
        let canonical = result["canonical_source"].as_str().expect("source");
        assert!(
            canonical.contains("command math::fleet::alpha {"),
            "{canonical}"
        );
        assert!(canonical.contains("arity 3"), "{canonical}");
        for word in ["proc ", "foreach ", "$name", "$arity"] {
            assert!(!canonical.contains(word), "{canonical}");
        }
        // And the expansion is a pack the checker accepts, with the same
        // commands — the loop the author is meant to close.
        let checked = check(canonical, "tcl9.0");
        assert_eq!(checked["summary"]["commands"], 3, "{checked}");
        assert_eq!(checked["notices"], json!([]), "{checked}");
    }

    /// A pack that is already canonical expands to itself, modulo layout:
    /// the same commands, the same notices, nothing invented.
    #[test]
    fn expand_is_a_no_op_on_a_canonical_pack() {
        let once = expand(VALID);
        let source = once["canonical_source"].as_str().expect("source");
        let twice = expand(source);
        assert_eq!(
            twice["canonical_source"], once["canonical_source"],
            "expansion is not idempotent"
        );
        assert_eq!(once["commands"], json!(["mylib::with_var"]), "{once}");
        // The hook body survives verbatim — a `CommandSpec` could not carry
        // it, and the record does.
        assert!(source.contains("arg_role_resolver -inputs"), "{source}");
        assert!(source.contains("role 1 Body"), "{source}");
    }

    /// A pack whose evaluation failed expands to nothing, and says why.
    #[test]
    fn expand_reports_a_pack_that_registered_nothing() {
        let result = expand("speclib clocky 2.0 {\n    set now [clock seconds]\n}\n");
        assert!(
            result["load_error"]
                .as_str()
                .unwrap_or_default()
                .contains("determinism axis"),
            "{result}"
        );
        assert_eq!(result["commands"], json!([]), "{result}");
    }

    /// A source with no `speclib` wrapper loads nothing and says why, rather
    /// than erroring.
    #[test]
    fn a_pack_with_no_speclib_wrapper_reports_a_notice() {
        let result = check("command foo { arity 1 }\n", "tcl9.0");
        assert_eq!(result["summary"]["commands"], 0, "{result}");
        assert!(
            result["notices"].as_array().is_some_and(|n| !n.is_empty()),
            "{result}"
        );
    }
}
