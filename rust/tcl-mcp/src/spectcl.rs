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

//! `spectcl_check` — the structured parse report for a `.tclspec` spec pack.
//!
//! The CLI-side `tcl spec check` of `docs/design/spec-packs.md`, exposed to an
//! agent instead of a terminal: a pack's source text in, a machine-readable
//! account of what the loader made of it out. It is what the `spec-author`
//! skill calls to validate a pack it has just written.
//!
//! **There is exactly one parser.** Every fact below comes from
//! [`tcl_spec_studio::spectcl::load_pack`] — the same loader the studio and the
//! equivalence gate use — and from the [`Draft`] it seeds through the ordinary
//! [`draft::from_command_spec`]. This module reads that result and renders it;
//! it never looks at the pack text itself, with the single, documented
//! exception of the hook-body `ctx` scan under
//! [`shape_cacheability`](fn@shape_cacheability), which inspects body text the
//! loader deliberately carries verbatim.
//!
//! Four things are reported:
//!
//! 1. **Commands** — the name, and which draft fields the pack actually set,
//!    computed by diffing the pack's draft against
//!    [`draft::default_command_draft`]. A field is "set" exactly when it
//!    differs from a brand-new command's value, so the list answers "what did
//!    this declaration actually say" rather than "which keys exist".
//! 2. **Notices** — every [`Notice`] the loader raised, as
//!    `{line, context, reason}`. A notice is always a *degradation*: the named
//!    declaration was dropped and the rest of the pack loaded. An unknown word
//!    surfaces here and nowhere else, which is what makes this report the way
//!    to find a typo'd trait or a property this server is too old to know.
//! 3. **Hooks** — every declared hook with its family, what it hangs off, and
//!    whether it is **shape-cacheable** (see below).
//! 4. **Collisions** — commands whose name the shipped registry already
//!    defines for the target dialect. Shipped wins unless the declaration says
//!    `-override`, so an unintended collision is a silently dead command.
//!
//! ## A note on cost
//!
//! [`load_pack`](tcl_spec_studio::spectcl::load_pack) installs a pack for the
//! process's life by leaking its strings and specs — a `CommandSpec` is a
//! `&'static`-shaped record. Checking a pack therefore leaks it, by design of
//! the loader rather than of this tool. That is fine for an MCP server driven
//! by an authoring session (a few packs per session, kilobytes each) and is
//! noted here so nobody wires this handler into a hot loop.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};
use tcl_registry::CommandRegistry;
use tcl_registry::spec::CommandSpec;
use tcl_spec_studio::draft::{self, Draft, UNRENDERABLE_KEY};
use tcl_spec_studio::spectcl::{self, HookDecl, HookFamily, HookOwner, HookSource, PackCommand};

/// `ctx` keys that are *part of* the `(command, word-shape)` memo key
/// `docs/design/spec-packs.md` assumes for its 24.5 ns/call figure.
///
/// `command`, `subcommand`, and `nwords` are components of the key itself;
/// `kinds` is the word shape. The option-arity family's three extra keys are
/// all functions of the call's word vector — the option word, its index, and
/// the index after it — so they are in the shape too.
const SHAPE_CTX_KEYS: &[&str] = &[
    "command",
    "subcommand",
    "nwords",
    "kinds",
    "option",
    "option-index",
    "option-value-start",
];

/// `ctx` keys *outside* the shape key. Reading one is the "declares broader
/// dependencies" case: fully legal, and uncacheable.
///
/// `tcl-version` and `dialect` vary per profile and `in-event-body` per call
/// site, and none of the three is a function of the words — so a body that
/// consults any of them cannot be answered from a word-shape memo.
const BROADER_CTX_KEYS: &[&str] = &["tcl-version", "dialect", "in-event-body"];

/// The MCP `spectcl_check` handler.
pub fn spectcl_check(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let dialect = crate::tools::declared_dialect(args);
    let registry = tcl_registry::registry_for_dialect(&dialect);

    let pack = spectcl::load_pack(source);

    let defaults = draft::default_command_draft();
    let sub_defaults = draft::default_subcommand_draft();

    let commands: Vec<Value> = pack
        .commands
        .iter()
        .map(|c| command_json(c, &defaults, &sub_defaults))
        .collect();
    let notices: Vec<Value> = pack
        .notices
        .iter()
        .map(|n| json!({ "line": n.line, "context": n.context, "reason": n.message }))
        .collect();
    let collisions: Vec<Value> = pack
        .commands
        .iter()
        .filter_map(|c| collision_json(c, registry, &dialect))
        .collect();

    let hook_count: usize = pack.commands.iter().map(|c| c.hooks.len()).sum();
    let uncacheable = pack
        .commands
        .iter()
        .flat_map(|c| c.hooks.iter())
        .filter(|h| shape_cacheability(h).0 == Some(false))
        .count();
    let shadowed = collisions
        .iter()
        .filter(|c| c["effect"] == "shipped-spec-wins")
        .count();

    json!({
        "pack": pack.name,
        "dsl_version": pack.dsl_version,
        "dialect": dialect,
        "commands": commands,
        "notices": notices,
        "collisions": collisions,
        "summary": {
            "commands": pack.commands.len(),
            "notices": pack.notices.len(),
            "hooks": hook_count,
            "uncacheable_hooks": uncacheable,
            "collisions": collisions.len(),
            "shadowed_commands": shadowed,
        },
    })
}

// ── Commands ──────────────────────────────────────────────────────────

/// One command: its name, the draft fields the declaration set, its
/// subcommands, and its hooks.
fn command_json(cmd: &PackCommand, defaults: &Draft, sub_defaults: &Draft) -> Value {
    let d = cmd.draft();
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
        "hook_families": families.iter().collect::<Vec<_>>(),
        "hooks": hooks,
    })
}

/// The draft keys whose value differs from a brand-new command's — i.e. the
/// ones this declaration actually said something about.
///
/// `name` is excluded because it is reported separately and always differs;
/// [`UNRENDERABLE_KEY`] because it is bookkeeping, not a declared field.
fn fields_set(d: &Map<String, Value>, defaults: &Draft) -> Vec<String> {
    d.iter()
        .filter(|(key, _)| key.as_str() != "name" && key.as_str() != UNRENDERABLE_KEY)
        .filter(|(key, value)| defaults.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .collect()
}

// ── Hooks ─────────────────────────────────────────────────────────────

/// The schema spelling of a hook family — the word an author writes.
fn family_key(family: HookFamily) -> &'static str {
    match family {
        HookFamily::ArgRoleResolver => "arg_role_resolver",
        HookFamily::CommandPrefixResolver => "command_prefix_resolver",
        HookFamily::ConstFold => "const_fold",
        HookFamily::ConstFoldVersioned => "const_fold_versioned",
        HookFamily::TaintSinkGate => "taint_sink_gate",
        HookFamily::ContextGate => "context_gate",
        HookFamily::LiteralArgumentValidator => "literal_argument_validator",
        HookFamily::ClauseShapeCheck => "clause_shape_check",
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
    let (cacheable, reason) = shape_cacheability(hook);
    json!({
        "owner": owner_label(&hook.owner),
        "field": hook.field,
        "family": family_key(hook.family),
        "source": kind,
        "detail": detail,
        "ctx_keys": ctx_keys(hook),
        "shape_cacheable": cacheable,
        "cache_reason": reason,
        "verbs": hook.family.verbs(),
        "silence_means": hook.family.silence(),
        "requires_all_literal": hook.family.requires_all_literal(),
    })
}

/// The `ctx` keys a Tcl hook body reads, as far as a `dict get $ctx KEY` scan
/// can tell. Empty for a native or derived hook, which has no body.
fn ctx_keys(hook: &HookDecl) -> Vec<String> {
    match &hook.source {
        HookSource::Body { body, .. } => ctx_reads(body).0.into_iter().collect(),
        HookSource::Native { .. } | HookSource::Derived { .. } => Vec::new(),
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
    let HookSource::Body { body, .. } = &hook.source else {
        return (
            None,
            "no VM body — native and derived hooks run at native cost".to_owned(),
        );
    };
    if hook.family == HookFamily::ConstFoldVersioned {
        return (
            Some(false),
            "`const_fold_versioned` takes `tcl-version` by family, which is not \
             part of the word shape"
                .to_owned(),
        );
    }
    let (keys, unattributed) = ctx_reads(body);
    let broader: Vec<&str> = BROADER_CTX_KEYS
        .iter()
        .copied()
        .filter(|k| keys.contains(*k))
        .collect();
    if !broader.is_empty() {
        return (
            Some(false),
            format!(
                "reads {} from `ctx`, outside the (command, word-shape) key",
                broader.join(", ")
            ),
        );
    }
    if unattributed {
        return (
            Some(false),
            "uses `$ctx` in a way the check cannot attribute to a literal key, \
             so its inputs are not provably inside the word shape"
                .to_owned(),
        );
    }
    (
        Some(true),
        "reads only the call's words and shape inputs, so the answer memoises \
         by (command, word-shape)"
            .to_owned(),
    )
}

/// Scan a hook body for `dict get $ctx KEY`.
///
/// Returns the literal keys read, and whether any `$ctx` / `${ctx}` reference
/// was *not* part of such a read — the case where the read set is unknown.
///
/// Textual on purpose: the pack itself is parsed exactly once, by the loader,
/// which carries a hook body through as verbatim text because the sandbox that
/// will eventually run it does not exist yet. Until it does there is no
/// compiled form to interrogate, and a heuristic that can only be pessimistic
/// is the honest amount of machinery for a report.
fn ctx_reads(body: &str) -> (BTreeSet<String>, bool) {
    let mut keys = BTreeSet::new();
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
            keys.insert(key);
        }
    }
    // `${ctx}` — the braced spelling reads the same variable.
    let braced = body.matches("${ctx}").count();
    total += braced;

    (keys, attributed < total)
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// The literal key of a `dict get $ctx KEY` whose `$ctx` starts at `at`, when
/// the reference is exactly that shape.
fn dict_get_key(body: &str, at: usize, after: usize) -> Option<String> {
    // Preceded by `dict get` (any whitespace run between and before).
    let before = body[..at].trim_end();
    let before = before.strip_suffix("get")?;
    if before.len() == body[..at].trim_end().len() - "get".len()
        && !before.ends_with(char::is_whitespace)
    {
        return None;
    }
    let before = before.trim_end();
    if !before.ends_with("dict") {
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

/// Resolve a name the way this registry's own dialect rules resolve it.
fn shipped(registry: &'static CommandRegistry, name: &str) -> Option<&'static CommandSpec> {
    match registry.profile() {
        Some(profile) => registry.get_for_dialect(name, profile.availability_mask),
        None => registry.get(name),
    }
}

/// A warning when the target dialect's shipped registry already defines this
/// command name.
///
/// Shipped wins unless the declaration says `-override`
/// (`docs/design/spec-packs.md`, "Loading and tooling"), so a collision without
/// `-override` means the pack's command never reaches a query — the failure
/// worth catching before a user reports "my spec does nothing".
fn collision_json(
    cmd: &PackCommand,
    registry: &'static CommandRegistry,
    dialect: &str,
) -> Option<Value> {
    let shipped = shipped(registry, cmd.spec.name)?;
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
speclib mylib 1.0 {
    command mylib::with_var {
        arity 2 3
        arg 0 -role VarWrite
        arg 1 -role Body
        return_type String
        arg_role_resolver {words ctx} {
            if {[llength $words] >= 2} {
                role 1 Body
            }
        }
    }
}
";

    #[test]
    fn a_valid_pack_reports_its_commands_and_no_notices() {
        let result = check(VALID, "tcl9.0");
        assert_eq!(result["pack"], "mylib", "{result}");
        assert_eq!(result["dsl_version"], "1.0");
        assert_eq!(result["dialect"], "tcl9.0");
        assert_eq!(result["notices"], json!([]), "{result}");
        assert_eq!(result["summary"]["commands"], 1);
        assert_eq!(result["collisions"], json!([]));

        let command = &result["commands"][0];
        assert_eq!(command["name"], "mylib::with_var");
        let set = strings(&command["fields_set"]);
        for field in ["arity", "arg_roles", "return_type", "arg_role_resolver"] {
            assert!(set.contains(&field), "{field} missing from {command}");
        }
        // A field the pack never mentioned stays at its default and is absent.
        assert!(!set.contains(&"traits"), "{command}");
    }

    /// The one hook is an `arg_role_resolver` body reading only `words`, so it
    /// is shape-cacheable under the declared-inputs rule.
    #[test]
    fn a_words_only_resolver_body_is_shape_cacheable() {
        let result = check(VALID, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["family"], "arg_role_resolver", "{hook}");
        assert_eq!(hook["owner"], "command");
        assert_eq!(hook["source"], "body");
        assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
        assert_eq!(hook["verbs"], json!(["role"]));
        assert_eq!(result["summary"]["uncacheable_hooks"], 0);
        assert_eq!(
            strings(&result["commands"][0]["hook_families"]),
            vec!["arg_role_resolver"]
        );
    }

    /// A body reaching outside the word shape is legal and reported as
    /// uncacheable, with the offending `ctx` key named.
    #[test]
    fn a_context_reading_body_is_reported_uncacheable() {
        let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1 1
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
                .contains("in-event-body"),
            "{hook}"
        );
        assert_eq!(hook["silence_means"], "the call is allowed");
        assert_eq!(result["summary"]["uncacheable_hooks"], 1);
    }

    /// A `-native` hook has no VM body, so cacheability does not apply to it.
    #[test]
    fn a_native_hook_is_not_a_cacheability_question() {
        let source = r"
speclib mylib 1.0 {
    command mylib::native {
        arity 1 1
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
        arity 1 2
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

    /// Redeclaring a shipped name without `-override` is the silent-no-op
    /// case, and the report says so.
    #[test]
    fn a_shipped_name_collides_and_the_pack_spec_loses() {
        let source = r"
speclib mylib 1.0 {
    command lsort {
        arity 1 -1
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
        arity 1 -1
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
        arity 0 1
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
        arity 1 -1
        subcommand indices {
            arity 1 1
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

    /// `$ctx` the scan cannot attribute to a literal key downgrades the
    /// answer — the pessimistic direction, never the optimistic one.
    #[test]
    fn an_unattributable_ctx_use_is_conservatively_uncacheable() {
        let source = r"
speclib mylib 1.0 {
    command mylib::dynamic {
        arity 1 1
        arg_role_resolver {words ctx} {
            set key command
            if {[dict get $ctx $key] eq {}} return
            role 0 Body
        }
    }
}
";
        let result = check(source, "tcl9.0");
        let hook = &result["commands"][0]["hooks"][0];
        assert_eq!(hook["shape_cacheable"], json!(false), "{hook}");
        assert!(
            hook["cache_reason"]
                .as_str()
                .unwrap_or_default()
                .contains("attribute"),
            "{hook}"
        );
    }

    /// A source with no `speclib` wrapper loads nothing and says why, rather
    /// than erroring.
    #[test]
    fn a_pack_with_no_speclib_wrapper_reports_a_notice() {
        let result = check("command foo { arity 1 1 }\n", "tcl9.0");
        assert_eq!(result["summary"]["commands"], 0, "{result}");
        assert!(
            result["notices"].as_array().is_some_and(|n| !n.is_empty()),
            "{result}"
        );
    }
}
