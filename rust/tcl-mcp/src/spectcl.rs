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
//! `.tclspec` spec pack.
//!
//! The CLI-side `tcl spec check` / `tcl spec export` of
//! `docs/design/registry/spec-packs.md`, exposed to an agent instead of a terminal: a
//! pack's source text in, a machine-readable account of what the loader made
//! of it out, and — for a pack written as a *program* — its expansion back
//! out as canonical source. They are what the `spec-author` skill calls to
//! validate a pack it has just written, and to read what its own loop
//! actually registered.
//!
//! **There is exactly one loader** — true of the tree, not just of this
//! module. Every fact below comes from
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
use tcl_dialect::model::WorkspaceTrust;
use tcl_registry::CommandRegistry;
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::spec::CommandSpec;
use tcl_registry::value_transfer::{
    DeclaredEvaluation, DeclaredInput, DeclaredSemantics, SemanticsDeclaration,
};
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
/// Use the engine's [`HookInputs`] rule so this report and cache allocation
/// agree on which declared inputs are shape-only.
fn ctx_key_in_shape(key: &str) -> bool {
    HookInputs::parse(&[key]).shape_only()
}

/// The MCP `spectcl_check` handler.
///
/// The pack is **evaluated**, not walked: a program written by
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
/// the `tier`/`trust` provenance preview below.
///
/// The pack is always **evaluated** as trusted — the file is the author's
/// own, and the authority ruling makes its declarative facts authoritative
/// regardless of trust, so every command, notice, and hook the pack declares
/// is reported whatever `tier`/`trust` name. `tier` (`bundled`, `user`,
/// `workspace`, or `studio-override`; default `workspace`, the tier a
/// workspace `.tclspec` file actually installs at) and `trust` (`trusted` or
/// `untrusted`; default `trusted`) instead choose which install this report
/// *previews*, over that one evaluated snapshot rather than a second,
/// possibly-discarding evaluation: `untrusted_tier_refusal` is the E-R2
/// refusal `tier` would draw from an untrusted install (never for the
/// bundled or user tier, which no trust state makes untrusted), and
/// `dormant_hooks` is every hook body that stays dormant under the
/// `(tier, trust)` pair's provenance (redesign O4) — empty when that
/// provenance is itself untrusted and refuses the pack, since a pack that
/// does not load holds nothing dormant. `stamp_refusals` is, on the same
/// terms, every codegen-axis stamp that install drops under the stamp
/// rejection rule ([`tcl_spectcl::stamps`]): all of them outside a bundled
/// pack, and a bundled pack's too unless its command's `alias_of` names the
/// shipped builtin that carries the stamp.
pub fn spectcl_check(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let dialect = crate::tools::declared_dialect(args);
    let hook_config = tcl_lexer::LexerConfig::for_profile(Some(
        crate::environment::profile_for_dialect(&dialect),
    ));
    let registry = tcl_spectcl::bundled::registry_for_dialect(&dialect);
    let tier = declared_tier(args);
    let trust = declared_trust(args);

    let pack = evaluate_pack(source);

    let defaults = draft::default_command_draft();
    let sub_defaults = draft::default_subcommand_draft();

    let commands: Vec<Value> = pack
        .commands
        .iter()
        .map(|c| command_json(c, &defaults, &sub_defaults, hook_config))
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
    // pack lands at `tier` (default the workspace tier a `.tclspec` file
    // actually installs at), where E-R2 refuses a `-override` on a compiled
    // name, a `dialect` block, or a reserved `environment` name outright.
    // Reporting it here is the difference between finding that out now and
    // finding it out when the pack silently fails to load in an editor.
    let tier_refusal = tcl_spectcl::provenance_violation(&pack, tier);
    if let Some((line, message)) = &tier_refusal {
        notices.push(json!({
            "line": line,
            "context": "pack",
            "class": "provenance",
            "reason": message,
        }));
    }
    let provenance = tcl_spectcl::PackEnvironmentTier::of(tier, trust).provenance();
    let InstallPreview {
        dormant_hooks,
        stamp_refusals,
    } = install_preview(&pack, provenance, tier_refusal.is_some());
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
        .filter(|h| declaration_conflict(h, &CtxScan::of(h, hook_config)).is_some())
        .count();
    let evaluate_finding_count: usize = pack
        .commands
        .iter()
        .flat_map(|c| c.hooks.iter().map(move |h| evaluate_findings(h, c.spec)))
        .map(|findings| findings.len())
        .sum();
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
        "untrusted_tier_refusal": tier_refusal.as_ref().map(|(_, message)| message.clone()),
        "provenance": tcl_registry::model::provenance_label(provenance),
        "dormant_hooks": dormant_hooks,
        "stamp_refusals": stamp_refusals,
        "summary": {
            "commands": pack.commands.len(),
            "notices": notice_count,
            "hooks": hook_count,
            "uncacheable_hooks": uncacheable,
            "declaration_conflicts": conflicts,
            "evaluate_findings": evaluate_finding_count,
            "collisions": collisions.len(),
            "shadowed_commands": shadowed,
            "dormant_hooks": dormant_hooks.len(),
            "stamp_refusals": stamp_refusals.len(),
        },
    })
}

/// What the install a `(tier, trust)` pair describes does to the pack's
/// executable half: the hook bodies it holds dormant and the codegen-axis
/// stamps it drops.
struct InstallPreview {
    dormant_hooks: Vec<Value>,
    stamp_refusals: Vec<Value>,
}

/// The [`InstallPreview`] at `provenance` — the pair's own, which the report
/// names so an author sees which install this preview is of. `dormant_hooks`
/// gates on it (D3.12: only an untrusted workspace holds bodies dormant), and
/// `stamp_refusals` is the stamp rejection rule's verdict under it, the
/// warnings the load would publish on each command's row. When that
/// provenance is itself untrusted and refuses the pack (`refused`), the pack
/// does not load there at all, so both are empty — the report says one thing
/// about one install.
fn install_preview(
    pack: &tcl_spectcl::Pack,
    provenance: tcl_dialect::model::Provenance,
    refused: bool,
) -> InstallPreview {
    if refused && tcl_registry::model::untrusted(provenance) {
        return InstallPreview {
            dormant_hooks: Vec::new(),
            stamp_refusals: Vec::new(),
        };
    }
    let dormant_hooks = tcl_spectcl::hooks::dormant_hooks(&pack.name, &pack.commands, provenance)
        .iter()
        .map(dormant_hook_json)
        .collect();
    let stamp_refusals = pack
        .commands
        .iter()
        .flat_map(|command| {
            tcl_spectcl::stamps::stamp_refusals(
                command.spec,
                provenance,
                None,
                tcl_spectcl::stamps::shipped(),
            )
            .into_iter()
            .map(move |refusal| {
                json!({
                    "command": command.spec.name,
                    "line": command.line,
                    "message": refusal.message(),
                })
            })
        })
        .collect();
    InstallPreview {
        dormant_hooks,
        stamp_refusals,
    }
}

/// The `tier` argument's spelling of [`Tier::StudioOverride`] — the one
/// tier whose own label ("Spec Studio override") is prose rather than a
/// token an argument can carry.
const STUDIO_OVERRIDE_ARGUMENT: &str = "studio-override";

/// The four `tier` argument values the schema advertises: each discovery
/// tier's own [`Tier::label`], and [`STUDIO_OVERRIDE_ARGUMENT`].
pub(crate) const TIER_ARGUMENTS: [&str; 4] = [
    Tier::Bundled.label(),
    Tier::User.label(),
    Tier::Workspace.label(),
    STUDIO_OVERRIDE_ARGUMENT,
];

/// The tier `tier`/`trust` name, from the MCP arguments: the tier a
/// workspace `.tclspec` file actually installs at by default, the other
/// three discovery tiers on request, each by its own label.
fn declared_tier(args: &Value) -> Tier {
    let Some(named) = args.get("tier").and_then(Value::as_str) else {
        return Tier::Workspace;
    };
    if named == STUDIO_OVERRIDE_ARGUMENT {
        return Tier::StudioOverride;
    }
    [Tier::Bundled, Tier::User, Tier::Workspace]
        .into_iter()
        .find(|tier| tier.label() == named)
        .unwrap_or(Tier::Workspace)
}

/// The editor's Workspace Trust state to preview `dormant_hooks` under —
/// trusted by default, since the file is the author's own and that is what
/// checking it has always meant.
fn declared_trust(args: &Value) -> WorkspaceTrust {
    match args.get("trust").and_then(Value::as_str) {
        Some("untrusted") => WorkspaceTrust::Untrusted,
        _ => WorkspaceTrust::Trusted,
    }
}

/// One dormant hook, as `spectcl_check` reports it: the same fact
/// [`tcl_spectcl::PackNotice::dormant`] turns into a load-time notice, keyed
/// so an author can find the row.
fn dormant_hook_json(hook: &tcl_spectcl::hooks::DormantHook) -> Value {
    json!({
        "command": hook.command,
        "field": hook.field,
        "line": hook.line,
        "message": tcl_spectcl::PackNotice::dormant(hook).message,
    })
}

// ── Expand ────────────────────────────────────────────────────────────

/// The MCP `spectcl_expand` handler — a pack's snapshot as canonical source.
///
/// The affordance that makes a *programmed* pack reviewable. A model that
/// writes a pack as a loop cannot see what the loop registered without
/// simulating it in its head, which is exactly the opacity a frozen
/// snapshot exists to prevent. This evaluates the pack and writes the
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
fn command_json(
    cmd: &PackCommand,
    defaults: &Draft,
    sub_defaults: &Draft,
    config: tcl_lexer::LexerConfig,
) -> Value {
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
    let hooks: Vec<Value> = cmd
        .hooks
        .iter()
        .map(|h| hook_json(h, cmd.spec, config))
        .collect();
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
        HookFamily::Evaluate => "evaluate",
        HookFamily::StateTransitionResolver => "state_transitions.resolver",
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

fn hook_json(hook: &HookDecl, spec: &CommandSpec, config: tcl_lexer::LexerConfig) -> Value {
    let (kind, detail) = match &hook.source {
        HookSource::Body { params, .. } => ("body", params.join(" ")),
        HookSource::Native { id } => ("native", id.clone()),
        HookSource::Derived { keyword } => ("derived", keyword.clone()),
    };
    let scan = CtxScan::of(hook, config);
    let (cacheable, reason) = shape_cacheability(hook);
    json!({
        "owner": owner_label(&hook.owner),
        "field": hook.field,
        "line": hook.line,
        "family": family_key(hook.family),
        "source": kind,
        "detail": detail,
        "ctx_keys": &scan.keys,
        "unknown_ctx_keys": scan.unknown(),
        "shape_cacheable": cacheable,
        "cache_reason": reason,
        "declaration_conflict": declaration_conflict(hook, &scan),
        "evaluate_findings": evaluate_findings(hook, spec),
        "verbs": hook.family.verbs(),
        "silence_means": hook.family.silence(),
        "requires_all_literal": hook.family.requires_all_literal(),
    })
}

/// The declared plan a hook's owner states, when it is a pack's own
/// (`as_declared`) rather than a shipped, compiled-in specialisation.
fn declared_semantics_for<'a>(
    spec: &'a CommandSpec,
    owner: &HookOwner,
) -> Option<&'a DeclaredSemantics> {
    let declaration = match owner {
        HookOwner::Command => spec.semantics,
        HookOwner::Subcommand(name) => {
            spec.subcommands
                .iter()
                .find(|sub| sub.name == name)?
                .semantics
        }
        HookOwner::Option { .. } => return None,
    };
    match declaration {
        SemanticsDeclaration::Declared(semantics) => semantics.as_declared(),
        SemanticsDeclaration::Inherited | SemanticsDeclaration::Declined => None,
    }
}

/// The three declared-implementation findings
/// (`docs/design/compiler/value-evaluation.md` § *The four surfaces, the
/// parity tests, and `spectcl_check`*): each a report over the declaration
/// and — for the two that ask what the body *does* — the same pessimistic,
/// textual style [`ctx_reads`] already uses for the other eleven families,
/// applied to the `fold` / `write` / `preserve` verbs this one emits instead
/// of a `dict get $ctx KEY` read (`evaluate -implementation`'s body takes no
/// `ctx` at all — `tcl_spec_hooks::host::run` binds it only its declared
/// inputs, positionally). Each is a report, never an enforcement: the driver
/// already raises on a write to a non-target and declines a body silent on a
/// declared target at run time (`answer_of`'s three rules); this is what an
/// author sees before a user does.
fn evaluate_findings(hook: &HookDecl, spec: &CommandSpec) -> Vec<String> {
    let mut findings = Vec::new();
    if hook.family != HookFamily::Evaluate {
        return findings;
    }
    let HookSource::Body { body, .. } = &hook.source else {
        return findings;
    };
    let Some(declared) = declared_semantics_for(spec, &hook.owner) else {
        return findings;
    };
    let targets: &[usize] = declared
        .structure
        .stores
        .map_or(&[], |stores| stores.targets);
    let incoming: Vec<usize> = match declared.evaluation {
        DeclaredEvaluation::Implementation(implementation) => implementation
            .capability
            .inputs
            .iter()
            .filter_map(|input| match input {
                DeclaredInput::IncomingTarget { index } => Some(*index),
                DeclaredInput::Operand { .. } | DeclaredInput::OptionValue { .. } => None,
            })
            .collect(),
        DeclaredEvaluation::Route(_) => Vec::new(),
    };

    // An evaluator reads a target it did not declare: `inputs` names
    // `target N incoming` for an `N` the structural plan never declared as
    // one of its own targets, so the value it reads is not a target at all
    // by this declaration's own other half — almost always a typo for
    // `arg N exact`, or a `stores -targets` row an edit forgot to update.
    for index in &incoming {
        if !targets.contains(index) {
            findings.push(format!(
                "reads target {index} incoming, which `semantics`'s own `stores` \
                 row does not declare as one of this evaluator's targets"
            ));
        }
    }

    if !targets.is_empty() {
        let emissions = evaluate_emissions(body);
        // A write names a non-target: reported first, target by target, so a
        // typo'd index is named beside the row that would have accepted it.
        for &index in &emissions.named {
            if !targets.contains(&index) {
                findings.push(format!(
                    "writes or preserves target {index}, which is outside the \
                     structural plan's declared targets {targets:?} — the write \
                     raises at query time, a decline the pack can avoid"
                ));
            }
        }
        // An evaluator is silent on a declared target: the body answers
        // *something* (a fold, or a write/preserve of another target) but
        // never names this one, so rule 3 declines the whole answer and every
        // fact the body did establish for the others is lost with it. Never
        // reported when some write/preserve names an unattributable
        // (computed) target — the body may cover it, and this scan can only
        // ever be pessimistic in the direction of the target it can prove.
        if emissions.any && !emissions.unattributed {
            for &index in targets {
                if !emissions.named.contains(&index) {
                    findings.push(format!(
                        "answers without a `write` or `preserve` for declared \
                         target {index} — silence declines the whole answer, \
                         not just this target"
                    ));
                }
            }
        }
    }
    findings
}

/// What an `evaluate -implementation` body's `fold` / `write` / `preserve`
/// verb calls state, scanned the same pessimistic, textual way
/// [`ctx_reads`] scans for `dict get $ctx KEY`.
#[derive(Debug, Default)]
struct EmissionScan {
    /// Whether any of the three verbs was called at all.
    any: bool,
    /// Target indices a `write` or `preserve` names with a literal integer.
    named: BTreeSet<usize>,
    /// Whether some `write` / `preserve` names its target with something
    /// other than a literal integer, so `named` is not the full set.
    unattributed: bool,
}

fn evaluate_emissions(body: &str) -> EmissionScan {
    let mut scan = EmissionScan::default();
    let bytes = body.as_bytes();
    for verb in ["fold", "write", "preserve"] {
        let mut i = 0;
        while let Some(at) = body[i..].find(verb).map(|p| p + i) {
            let after = at + verb.len();
            i = after;
            // A whole word: not `unfold`, not `writeback`.
            let before_ok = at == 0 || !is_name_byte(bytes[at - 1]);
            let after_ok = bytes.get(after).is_none_or(|b| !is_name_byte(*b));
            if !before_ok || !after_ok {
                continue;
            }
            scan.any = true;
            if verb == "fold" {
                continue;
            }
            let rest = body[after..].trim_start();
            let end = rest
                .find(|c: char| c.is_whitespace() || c == ']' || c == '}' || c == ';')
                .unwrap_or(rest.len());
            match rest[..end].parse::<usize>() {
                Ok(index) => {
                    scan.named.insert(index);
                }
                Err(_) if rest[..end].is_empty() => {}
                Err(_) => scan.unattributed = true,
            }
        }
    }
    scan
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
    fn of(hook: &HookDecl, config: tcl_lexer::LexerConfig) -> Self {
        match &hook.source {
            HookSource::Body { body, .. } => ctx_reads(body, config),
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
/// **declared-inputs rule** of `docs/design/registry/spec-packs.md`'s hot-path budget.
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

/// Bounded textual hook-context advice; opaque uses remain unattributed.
/// This report cannot establish an executed hook or compiled context read.
fn ctx_reads(body: &str, config: tcl_lexer::LexerConfig) -> CtxScan {
    let mut scan = CtxScan::default();
    let bytes = body.as_bytes();
    let mut offset = 0;
    while let Some(at) = bytes[offset..]
        .iter()
        .position(|byte| *byte == b'$')
        .map(|relative| offset + relative)
    {
        offset = at + 1;
        let Ok(Some(reference)) = tcl_lexer::scan_var_ref(bytes, at, config) else {
            continue;
        };
        if reference.name != b"ctx" {
            continue;
        }
        if reference.index.is_none()
            && let Some(key) = dict_get_key(body, at, reference.next)
        {
            scan.keys.insert(key);
            continue;
        }
        scan.unattributed = true;
    }
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
/// The collision check here is a bit-mask intersection; properly it should
/// be `targets ⊆ applicable` over the declaration's version sets, which is
/// a model change rather than a simple port.
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
/// (`docs/design/registry/spec-packs.md`, "Loading and tooling"), so a collision without
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
mod tests;
