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

//! The Test tab's engine: run a **sample** of Tcl through the real analyser
//! with the pack under edit installed, and explain every word of it.
//!
//! `docs/design/spec-packs.md` asks for a surface where "my stuff is working"
//! is *observed, not asserted*: paste code that uses the built-ins plus the
//! pack, and see exactly what an editor would see. That is what this module
//! computes, and the emphasis is on **exactly**:
//!
//! - The pack is installed into a real [`CommandRegistry`] through
//!   [`tcl_spectcl::install::registry_with_packs`] — the same installer the
//!   language server runs at workspace init, under the same collision policy
//!   (*shipped wins unless the pack says `-override`*). No studio-local merge.
//! - The diagnostics come from [`Analyser::analyse`] pointed at that registry
//!   via [`Analyser::with_pack_overlay`], which is how the server, the CLI and
//!   the MCP server all read a pack. If the pack's arity is wrong, the sample
//!   gets the same arity error a user's editor would show.
//! - Every per-word answer — the role, the subcommand, the option — is asked of
//!   the registry ([`CommandRegistry::arg_indices_for_role`]), never
//!   re-derived. What this module adds on top is **provenance**: *which
//!   declaration in the spec put that word where it is*, which is the question
//!   a spec author actually has ("why is my third argument not a body?").
//!
//! ## What the walk covers, and what it does not
//!
//! Words are the segmenter's words, so the sample is read exactly as the
//! compiler reads it. Braced words whose resolved role is [`ArgRole::Body`] are
//! recursed into — generically, because the *spec* said they are scripts, so a
//! pack command that declares a body argument gets its body inspected for free.
//!
//! Not covered, and deliberately so in this slice: command substitution
//! (`[…]`) inside a word is not walked, and neither is `expr`'s sub-language.
//! Both are their own parsers; the analyser still sees them (its diagnostics
//! cover the whole sample), only the click-a-word view stops at them.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, LineIndex, SourceMap};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::registry::CommandRegistry;
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_spectcl::catalogue;

use crate::store::{Origin, Resolution};

/// How deep the body recursion goes before it stops.
///
/// A sample is a snippet, not a codebase; a runaway depth here would be a
/// pathological input rather than an honest one.
const MAX_DEPTH: usize = 12;

// The words of a sample

/// One word of the sample, with everything the Test tab paints it from.
#[derive(Debug, Clone)]
struct Word {
    /// Byte range in the whole sample.
    start: usize,
    end: usize,
    /// The word's source text, delimiters and all.
    text: String,
    /// Position in its own statement — `0` is the command head.
    index: usize,
    /// The statement's head word.
    head: String,
    /// The other words of the statement, head excluded.
    args: Vec<String>,
    /// How many bodies deep the statement is.
    depth: usize,
}

impl Word {
    /// The argument index this word occupies, or `None` for the head.
    fn arg_index(&self) -> Option<usize> {
        self.index.checked_sub(1)
    }
}

/// Segment `source` into words, recursing into every word the registry says is
/// a script body.
///
/// `base` is `source`'s own offset inside the whole sample, so every emitted
/// span is absolute no matter how deep the recursion went.
fn walk(registry: &CommandRegistry, whole: &str, base: usize, depth: usize, out: &mut Vec<Word>) {
    if depth > MAX_DEPTH {
        return;
    }
    let source = &whole[base..];
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(source, LexerConfig::default());
    for segment in segments_from_document(document, &source_map) {
        let head = segment.texts.first().cloned().unwrap_or_default();
        let args: Vec<String> = segment.texts.iter().skip(1).cloned().collect();
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        for (index, token) in segment.argv.iter().enumerate() {
            let start = base + token.span.start() as usize;
            let end = base + token.span.end() as usize;
            if start >= end || end > whole.len() {
                continue;
            }
            out.push(Word {
                start,
                end,
                text: whole[start..end].to_owned(),
                index,
                head: head.clone(),
                args: args.clone(),
                depth,
            });
        }

        // Recurse into the words the *spec* calls scripts. This is the whole
        // reason a pack command that declares `arg_roles { 2 Body }` gets its
        // body inspected without a line of body-specific code here.
        for arg_index in registry.arg_indices_for_role(&head, &arg_refs, ArgRole::Body) {
            let Some(token) = segment.argv.get(arg_index + 1) else {
                continue;
            };
            let start = base + token.span.start() as usize;
            let end = base + token.span.end() as usize;
            let lead = usize::from(token.content_offset);
            if lead == 0 || end <= start + lead || whole.as_bytes().get(start) != Some(&b'{') {
                continue;
            }
            // The braced word's contents: `{`…`}` minus both delimiters.
            let inner_start = start + lead;
            let inner_end = end - 1;
            if inner_end <= inner_start {
                continue;
            }
            walk(registry, &whole[..inner_end], inner_start, depth + 1, out);
        }
    }
}

// Provenance — which declaration put this word here

/// What the spec says about one word: what kind of word it is, the spec
/// property that says so, and a sentence an author can read.
#[derive(Debug, Clone)]
struct Provenance {
    /// `"command"`, `"subcommand"`, `"option"`, `"option-value"`,
    /// `"terminator"`, `"argument"`, or `"unknown"`.
    kind: &'static str,
    /// The spec property that produced this word's meaning, spelled the way
    /// the DSL and the form spell it (`"arg_roles"`, `"options"`, …).
    field: Option<String>,
    /// One line explaining the answer.
    detail: String,
}

/// The subcommand a call selects, if the spec dispatches on its first word.
fn dispatching_subcommand<'s>(spec: &'s CommandSpec, args: &[&str]) -> Option<&'s SubCommand> {
    if spec.subcommands.is_empty() {
        return None;
    }
    args.first().and_then(|word| spec.resolve_subcommand(word))
}

/// Where argument `idx` of a call to `spec` gets its meaning.
///
/// The order mirrors [`CommandRegistry::arg_indices_for_role`] exactly —
/// subcommand dispatch, then the option scan (with its `--` terminator), then
/// the positional tables — because an explanation that disagreed with the
/// answer it explains would be worse than no explanation at all.
fn provenance(spec: &CommandSpec, args: &[&str], idx: usize) -> Provenance {
    let sub = dispatching_subcommand(spec, args);
    if let Some(sub) = sub
        && idx == 0
    {
        return Provenance {
            kind: "subcommand",
            field: Some("subcommands".to_owned()),
            detail: format!(
                "selects the `{}` subcommand of `{}`{}",
                sub.name,
                spec.name,
                if args[0] == sub.name {
                    String::new()
                } else {
                    format!(" (`{}` is an accepted abbreviation)", args[0])
                }
            ),
        };
    }

    let (options, offset) = match sub {
        Some(sub) => (sub.options, 1usize),
        None => (spec.options, 0usize),
    };
    let owner = match sub {
        Some(sub) => format!("subcommand {} / options", sub.name),
        None => "options".to_owned(),
    };
    let mut i = offset;
    while i < args.len() {
        if args[i] == "--" {
            if i == idx {
                return Provenance {
                    kind: "terminator",
                    field: Some(owner),
                    detail: "ends option parsing — every later word is a value".to_owned(),
                };
            }
            break;
        }
        if let Some(option) = options.iter().find(|o| o.matches(args[i])) {
            let values = option.value_indices(args, i);
            if i == idx {
                return Provenance {
                    kind: "option",
                    field: Some(owner),
                    detail: if option.takes_value() {
                        format!(
                            "declared option `{}`{} — takes {} value word(s){}",
                            option.name,
                            if args[i] == option.name {
                                String::new()
                            } else {
                                format!(" (written `{}`)", args[i])
                            },
                            values.len(),
                            describe(option.detail),
                        )
                    } else {
                        format!(
                            "declared flag `{}`{} — takes no value{}",
                            option.name,
                            if args[i] == option.name {
                                String::new()
                            } else {
                                format!(" (written `{}`)", args[i])
                            },
                            describe(option.detail),
                        )
                    },
                };
            }
            if values.contains(&idx) {
                return Provenance {
                    kind: "option-value",
                    field: Some(owner),
                    detail: format!(
                        "the value of `{}`{}{}",
                        option.name,
                        option
                            .value_role()
                            .map(|role| format!(", declared {}", catalogue::variant_name(&role)))
                            .unwrap_or_default(),
                        describe(option.value_hint()),
                    ),
                };
            }
            i += 1 + values.len();
        } else {
            i += 1;
        }
    }

    positional(spec, sub, args, idx, offset)
}

/// The positional half of [`provenance`]: the three tables that can name an
/// argument position, in the registry's own precedence.
fn positional(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: &[&str],
    idx: usize,
    offset: usize,
) -> Provenance {
    let (roles, resolver, repeated) = match sub {
        Some(sub) => (sub.arg_roles, sub.arg_role_resolver, sub.repeated_args),
        None => (spec.arg_roles, spec.arg_role_resolver, spec.repeated_args),
    };
    let Some(local) = idx.checked_sub(offset) else {
        return Provenance {
            kind: "argument",
            field: None,
            detail: "argument position the spec declares no role for".to_owned(),
        };
    };
    let owner = |word: &str| match sub {
        Some(sub) => format!("subcommand {} / {word}", sub.name),
        None => word.to_owned(),
    };

    if let Some(resolve) = resolver {
        let tail: Vec<&str> = args.iter().skip(offset).copied().collect();
        if let Some((_, role)) = resolve(&tail)
            .into_iter()
            .find(|(at, _)| usize::from(*at) == local)
        {
            return Provenance {
                kind: "argument",
                field: Some(owner("arg_role_resolver")),
                detail: format!(
                    "the resolver read this whole call and gave word {local} the role {}",
                    catalogue::variant_name(&role)
                ),
            };
        }
    }
    if let Some((_, role)) = roles.iter().find(|(at, _)| usize::from(*at) == local) {
        return Provenance {
            kind: "argument",
            field: Some(owner("arg_roles")),
            detail: format!(
                "the role table pins position {local} to {}",
                catalogue::variant_name(role)
            ),
        };
    }
    let tail_len = args.len().saturating_sub(offset);
    for layout in repeated {
        if layout.indices(tail_len).contains(&local) {
            return Provenance {
                kind: "argument",
                field: Some(owner("repeated_args")),
                detail: format!(
                    "a repeating tail from position {} every {} word(s) gives it {}",
                    layout.start,
                    layout.stride,
                    catalogue::variant_name(&layout.role)
                ),
            };
        }
    }
    Provenance {
        kind: "argument",
        field: None,
        detail: format!(
            "argument {} of `{}` — the spec declares no role for this position",
            local + 1,
            spec.name
        ),
    }
}

/// `" — text"`, or nothing when the spec had nothing to add.
fn describe(text: &str) -> String {
    if text.trim().is_empty() {
        String::new()
    } else {
        format!(" — {text}")
    }
}

// The bench

/// The Test tab's world: the merged models, plus the live registry they
/// install into.
pub struct Bench<'a> {
    merged: &'a Resolution<'a>,
    registry: Arc<CommandRegistry>,
    overlay: u64,
}

impl<'a> Bench<'a> {
    /// Install `merged`'s pack over its built-ins and hold the result.
    ///
    /// Installation is cached on the pack's content key, so a sample re-run
    /// against an unchanged document costs a hash lookup.
    #[must_use]
    pub fn new(merged: &'a Resolution<'a>) -> Self {
        Self {
            merged,
            registry: merged.registry(),
            overlay: merged.overlay_key(),
        }
    }

    /// The registry an editor would resolve this sample against.
    #[must_use]
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Analyse `sample` and return everything the tab paints: the render
    /// chunks, one entry per word, and the analyser's diagnostics.
    #[must_use]
    pub fn analyse(&self, sample: &str) -> Value {
        let diagnostics = self.diagnostics(sample);
        let mut words = Vec::new();
        walk(&self.registry, sample, 0, 0, &mut words);
        words.sort_by_key(|w| (w.start, w.end));
        words.dedup_by_key(|w| (w.start, w.end));

        let lines = LineIndex::new(sample);
        let severities = severity_by_span(&diagnostics);

        let tokens: Vec<Value> = words
            .iter()
            .map(|word| {
                let position = lines.position_at_utf16(clamp_u32(word.start), sample);
                let (origin, resolved) = self.resolve_head(&word.head);
                let arg_refs: Vec<&str> = word.args.iter().map(String::as_str).collect();
                let roles = resolved
                    .map(|spec| self.roles_at(spec, &word.head, &arg_refs, word.arg_index()))
                    .unwrap_or_default();
                let prov = match (resolved, word.arg_index()) {
                    (Some(spec), Some(idx)) => provenance(spec, &arg_refs, idx),
                    (Some(_), None) => Provenance {
                        kind: "command",
                        field: Some("command".to_owned()),
                        detail: format!("resolves to the {} spec", origin_label(origin)),
                    },
                    (None, _) => Provenance {
                        kind: "unknown",
                        field: None,
                        detail: format!(
                            "`{}` is in neither the pack nor the {} registry",
                            word.head,
                            self.merged.builtins().dialect()
                        ),
                    },
                };
                json!({
                    "start": word.start,
                    "end": word.end,
                    "line": position.line + 1,
                    "column": position.character.get() + 1,
                    "text": word.text,
                    "index": word.index,
                    "depth": word.depth,
                    "command": word.head,
                    "origin": origin.map_or("unknown", Origin::key),
                    "kind": prov.kind,
                    "field": prov.field,
                    "detail": prov.detail,
                    "roles": roles,
                    "severity": worst_severity(&severities, word.start, word.end),
                })
            })
            .collect();

        let heads: BTreeMap<&str, bool> = words
            .iter()
            .filter(|w| w.index == 0)
            .map(|w| (w.head.as_str(), self.resolve_head(&w.head).1.is_some()))
            .collect();
        let from_pack = words
            .iter()
            .filter(|w| w.index == 0)
            .filter(|w| self.resolve_head(&w.head).0.is_some_and(Origin::pack_wins))
            .count();

        json!({
            "dialect": self.merged.builtins().dialect(),
            "pack": self.merged.store().name(),
            "installed": self.overlay != 0,
            "render": render_chunks(sample, &words),
            "tokens": tokens,
            "diagnostics": diagnostics,
            "summary": {
                "words": words.len(),
                "calls": words.iter().filter(|w| w.index == 0).count(),
                "pack_calls": from_pack,
                "unknown_commands": heads.values().filter(|known| !**known).count(),
                "diagnostics": diagnostics.len(),
                "errors": diagnostics
                    .iter()
                    .filter(|d| d["severity"] == json!("error"))
                    .count(),
                "warnings": diagnostics
                    .iter()
                    .filter(|d| d["severity"] == json!("warning"))
                    .count(),
            },
        })
    }

    /// Everything known about the word covering byte `offset`.
    ///
    /// This is the deep-inspection view: the resolved spec and where it came
    /// from, the argument's role and which declaration produced it, and the
    /// diagnostics that land on the word.
    #[must_use]
    pub fn inspect(&self, sample: &str, offset: usize) -> Option<Value> {
        let mut words = Vec::new();
        walk(&self.registry, sample, 0, 0, &mut words);
        // The innermost word wins: a body's own statements are pushed after
        // the body word that holds them, and are strictly narrower.
        let word = words
            .into_iter()
            .filter(|w| w.start <= offset && offset < w.end)
            .min_by_key(|w| w.end - w.start)?;

        let arg_refs: Vec<&str> = word.args.iter().map(String::as_str).collect();
        let (origin, resolved) = self.resolve_head(&word.head);
        let lines = LineIndex::new(sample);
        let position = lines.position_at_utf16(clamp_u32(word.start), sample);
        let diagnostics: Vec<Value> = self
            .diagnostics(sample)
            .into_iter()
            .filter(|d| {
                let start = json_usize(&d["start"]);
                let end = json_usize(&d["end"]);
                start < word.end && end > word.start
            })
            .collect();

        let prov = match (resolved, word.arg_index()) {
            (Some(spec), Some(idx)) => provenance(spec, &arg_refs, idx),
            (Some(_), None) => Provenance {
                kind: "command",
                field: Some("command".to_owned()),
                detail: format!("resolves to the {} spec", origin_label(origin)),
            },
            (None, _) => Provenance {
                kind: "unknown",
                field: None,
                detail: format!(
                    "`{}` is in neither the pack nor the {} registry",
                    word.head,
                    self.merged.builtins().dialect()
                ),
            },
        };
        let sub = resolved.and_then(|spec| dispatching_subcommand(spec, &arg_refs));

        Some(json!({
            "word": {
                "text": word.text,
                "start": word.start,
                "end": word.end,
                "line": position.line + 1,
                "column": position.character.get() + 1,
                "index": word.index,
                "depth": word.depth,
                "kind": prov.kind,
            },
            "call": {
                "command": word.head,
                "args": word.args,
                "resolved": resolved.is_some(),
            },
            "spec": resolved.map(|spec| self.spec_view(spec, &word.head, origin)),
            "subcommand": sub.map(|sub| json!({
                "name": sub.name,
                "detail": sub.detail,
                "synopsis": sub.synopsis,
                "arity": arity_text(sub.arity),
                "options": sub.options.len(),
            })),
            "role": {
                "roles": resolved
                    .map(|spec| self.roles_at(spec, &word.head, &arg_refs, word.arg_index()))
                    .unwrap_or_default(),
                "field": prov.field,
                "detail": prov.detail,
            },
            "diagnostics": diagnostics,
            "notices": notice_lines(self.merged, &word.head),
            "editable": self.merged.store().draft(&word.head).is_some(),
        }))
    }

    /// The head's origin under the collision policy, and the spec the merged
    /// registry actually answers with.
    fn resolve_head(&self, head: &str) -> (Option<Origin>, Option<&CommandSpec>) {
        (self.merged.origin(head), self.registry.get(head))
    }

    /// Every role the registry gives argument `arg_index` of this call.
    ///
    /// Asked one role at a time because that is the registry's own question
    /// shape; a position can legitimately answer to two (a `VarWrite` that is
    /// also a `VarRead`), and both are reported.
    fn roles_at(
        &self,
        _spec: &CommandSpec,
        head: &str,
        args: &[&str],
        arg_index: Option<usize>,
    ) -> Vec<Value> {
        let Some(index) = arg_index else {
            return Vec::new();
        };
        ArgRole::ALL
            .iter()
            .filter(|role| {
                self.registry
                    .arg_indices_for_role(head, args, **role)
                    .contains(&index)
            })
            .map(|role| {
                let key = catalogue::variant_name(role);
                json!({
                    "role": key,
                    "doc": catalogue::ARG_ROLES
                        .iter()
                        .find(|v| v.key == key)
                        .map_or("", |v| v.doc),
                })
            })
            .collect()
    }

    /// The resolved spec, as much of it as a reader needs at a glance.
    fn spec_view(&self, spec: &CommandSpec, name: &str, origin: Option<Origin>) -> Value {
        let hover = spec.hover;
        json!({
            "name": spec.name,
            "origin": origin.map_or("unknown", Origin::key),
            "source": origin_label(origin),
            "dialect": self.merged.builtins().dialect(),
            "summary": hover.map_or("", |h| h.summary),
            // Studio preview of one spec, with no document behind it — nothing
            // resolves a package-version floor here.
            "synopsis": spec.primary_synopsis(None).unwrap_or(""),
            "arity": arity_text(spec.arity),
            "subcommands": spec.subcommands.len(),
            "options": spec.options.len(),
            "required_package": spec.required_package,
            "return_type": spec
                .return_type
                .map(|t| catalogue::variant_name(&t))
                .unwrap_or_default(),
            "fields_set": self
                .merged
                .store()
                .draft(name)
                .map(crate::store::fields_set_of)
                .unwrap_or_default(),
        })
    }

    /// The analyser's diagnostics for `sample`, as JSON.
    ///
    /// The pack rides in on [`Analyser::with_pack_overlay`], which is the one
    /// and only way the server, the CLI and the MCP tools read a pack — so a
    /// diagnostic here is a diagnostic there.
    fn diagnostics(&self, sample: &str) -> Vec<Value> {
        let dialect = self.merged.builtins().dialect();
        let mut analyser = Analyser::new().with_pack_overlay(self.overlay);
        let result = analyser.analyse(sample, dialect);
        let lines = LineIndex::new(sample);
        let mut out: Vec<Value> = result
            .diagnostics
            .iter()
            .map(|d| {
                let start = lines.position_at_utf16(d.span.start(), sample);
                let end = lines.position_at_utf16(d.span.end(), sample);
                json!({
                    "code": d.code.to_string(),
                    "severity": d.severity.as_str(),
                    "message": d.message,
                    "line": start.line + 1,
                    "column": start.character.get() + 1,
                    "end_line": end.line + 1,
                    "end_column": end.character.get() + 1,
                    "start": d.span.start(),
                    "end": d.span.end(),
                })
            })
            .collect();
        out.sort_by_key(|d| {
            (
                d["start"].as_u64().unwrap_or(0),
                d["end"].as_u64().unwrap_or(0),
            )
        });
        out
    }
}

// Helpers

/// A byte offset as the line index takes it.
///
/// Saturating rather than wrapping: a sample past 4 GiB is not a thing a
/// browser tab holds, and clamping keeps the position honest (the end) instead
/// of wrapping it to the start.
fn clamp_u32(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// A JSON number read back as an offset.
fn json_usize(value: &Value) -> usize {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

/// A phrase naming which of the two models answered.
fn origin_label(origin: Option<Origin>) -> &'static str {
    match origin {
        Some(Origin::Pack) => "pack's own",
        Some(Origin::Override) => "pack's (it overrides the shipped command of that name)",
        Some(Origin::Shadowed) => "shipped (the pack's declaration is shadowed — no `-override`)",
        Some(Origin::Builtin) => "shipped registry's",
        None => "unknown",
    }
}

/// `1..3`, `2..`, `0` — the arity as a spec author writes it in the DSL.
fn arity_text(arity: tcl_registry::arity::Arity) -> String {
    if arity.is_unlimited() {
        format!("{}..", arity.min)
    } else if arity.min == arity.max {
        format!("{}", arity.min)
    } else {
        format!("{}..{}", arity.min, arity.max)
    }
}

/// The pack's load notices that mention `name`, as plain lines.
fn notice_lines(merged: &Resolution<'_>, name: &str) -> Vec<Value> {
    merged
        .store()
        .notices_for(name)
        .iter()
        .map(|n| json!({ "line": n.line, "context": n.context, "reason": n.message }))
        .collect()
}

/// Every diagnostic's span and severity, for the per-word underline.
fn severity_by_span(diagnostics: &[Value]) -> Vec<(usize, usize, &str)> {
    diagnostics
        .iter()
        .filter_map(|d| {
            Some((
                json_usize(&d["start"]),
                json_usize(&d["end"]),
                d["severity"].as_str()?,
            ))
        })
        .collect()
}

/// The most severe diagnostic overlapping `[start, end)`, or `null`.
fn worst_severity(spans: &[(usize, usize, &str)], start: usize, end: usize) -> Value {
    let rank = |s: &str| match s {
        "error" => 4,
        "warning" => 3,
        "info" => 2,
        "suggestion" => 1,
        _ => 0,
    };
    spans
        .iter()
        .filter(|(s, e, _)| *s < end && *e > start)
        .max_by_key(|(_, _, sev)| rank(sev))
        .map_or(Value::Null, |(_, _, sev)| json!(sev))
}

/// A word and the words written inside it — a `{ … }` body holds the
/// statements of its own script.
struct Nest {
    token: usize,
    start: usize,
    end: usize,
    children: Vec<Nest>,
}

/// Arrange the (start-sorted) words into containment order.
///
/// Word spans nest strictly — a body's statements lie inside the braced word
/// that holds them — so one stack is enough.
fn nest(words: &[Word]) -> Vec<Nest> {
    let mut roots: Vec<Nest> = Vec::new();
    let mut stack: Vec<Nest> = Vec::new();
    let close = |stack: &mut Vec<Nest>, roots: &mut Vec<Nest>, before: usize| {
        while stack.last().is_some_and(|top| top.end <= before) {
            let done = stack.pop().expect("checked");
            match stack.last_mut() {
                Some(parent) => parent.children.push(done),
                None => roots.push(done),
            }
        }
    };
    for (token, word) in words.iter().enumerate() {
        close(&mut stack, &mut roots, word.start);
        stack.push(Nest {
            token,
            start: word.start,
            end: word.end,
            children: Vec::new(),
        });
    }
    close(&mut stack, &mut roots, usize::MAX);
    roots
}

/// The sample split into render chunks: every byte of it, in order, each
/// either plain text or a word.
///
/// The front-end paints this list straight through, so it never has to convert
/// a Rust byte offset into a JavaScript string index — a conversion that is
/// wrong the moment the sample contains a non-ASCII character.
///
/// A word that holds other words contributes the bytes its children do not:
/// the braces and whitespace of a body belong to the body word (click them and
/// the inspector explains *why this argument is a script*), while the
/// statements inside it are their own clickable words.
fn render_chunks(sample: &str, words: &[Word]) -> Value {
    /// Emit `text` as `token`, never letting one chunk span a line.
    ///
    /// A browser lays a `<button>` out as one box even when told to be inline,
    /// so a chunk carrying a newline — a `{ … }` body's opening brace and the
    /// indentation after it — would render as a two-line box the surrounding
    /// text then aligns on a single baseline, scrambling the view. Splitting on
    /// the newline here keeps every clickable box to one line and leaves the
    /// line breaks as ordinary text, which is a property of the data rather
    /// than a stylesheet's promise.
    fn push(text: &str, token: Option<usize>, chunks: &mut Vec<Value>) {
        if text.is_empty() {
            return;
        }
        let token = match token {
            Some(index) => json!(index),
            None => Value::Null,
        };
        let mut first = true;
        for line in text.split('\n') {
            if !first {
                chunks.push(json!({ "text": "\n", "token": Value::Null }));
            }
            first = false;
            if !line.is_empty() {
                chunks.push(json!({ "text": line, "token": token }));
            }
        }
    }

    fn walk(sample: &str, node: &Nest, chunks: &mut Vec<Value>) {
        if node.children.is_empty() {
            push(&sample[node.start..node.end], Some(node.token), chunks);
            return;
        }
        let mut cursor = node.start;
        for child in &node.children {
            if child.start > cursor {
                push(&sample[cursor..child.start], Some(node.token), chunks);
            }
            walk(sample, child, chunks);
            cursor = child.end;
        }
        if cursor < node.end {
            push(&sample[cursor..node.end], Some(node.token), chunks);
        }
    }

    let mut chunks: Vec<Value> = Vec::new();
    let mut cursor = 0usize;
    for root in nest(words) {
        if root.start > cursor {
            push(&sample[cursor..root.start], None, &mut chunks);
        }
        walk(sample, &root, &mut chunks);
        cursor = root.end;
    }
    if cursor < sample.len() {
        push(&sample[cursor..], None, &mut chunks);
    }
    Value::Array(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Builtins, PackStore};

    const PACK: &str = "speclib demo 1.0 {\n\
                        command greet {\n\
                        \x20   arity 1..2\n\
                        \x20   hover {\n\
                        \x20       summary {Say hello to someone.}\n\
                        \x20   }\n\
                        \x20   arg 0 -role Name\n\
                        \x20   option -loud -detail {Shout it.}\n\
                        }\n\
                        command withbody {\n\
                        \x20   arity 1\n\
                        \x20   arg 0 -role Body\n\
                        }\n\
                        }\n";

    fn bench_on(pack: &str, body: impl FnOnce(&Bench<'_>)) {
        let store = PackStore::from_source(pack);
        let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);
        let bench = Bench::new(&merged);
        body(&bench);
    }

    #[test]
    fn the_pack_is_installed_into_the_registry_the_analyser_reads() {
        bench_on(PACK, |bench| {
            assert!(
                bench.registry().get("greet").is_some(),
                "the pack's command must be in the merged registry"
            );
            // And the shipped commands are all still there.
            assert!(bench.registry().get("lappend").is_some());
        });
    }

    #[test]
    fn a_call_into_the_pack_is_not_an_unknown_command() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("greet world\n");
            let diagnostics = report["diagnostics"].as_array().expect("diagnostics");
            assert!(
                !diagnostics.iter().any(|d| d["code"] == json!("W123")),
                "a pack command must not report as unknown: {diagnostics:?}"
            );
            assert_eq!(report["summary"]["pack_calls"], json!(1));
            assert_eq!(report["summary"]["unknown_commands"], json!(0));
        });
    }

    #[test]
    fn a_call_that_breaks_the_packs_arity_is_reported() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("greet a b c\n");
            let diagnostics = report["diagnostics"].as_array().expect("diagnostics");
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d["message"].as_str().is_some_and(|m| m.contains("greet"))),
                "the pack's own arity must be enforced: {diagnostics:?}"
            );
        });
    }

    #[test]
    fn an_unknown_command_is_still_unknown() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("notacommand 1\n");
            assert_eq!(report["summary"]["unknown_commands"], json!(1));
        });
    }

    #[test]
    fn a_word_reports_the_spec_field_that_produced_it() {
        bench_on(PACK, |bench| {
            let sample = "greet -loud world\n";
            let report = bench.analyse(sample);
            let tokens = report["tokens"].as_array().expect("tokens");
            let head = &tokens[0];
            assert_eq!(head["kind"], json!("command"));
            assert_eq!(head["origin"], json!("pack"));

            let option = &tokens[1];
            assert_eq!(option["text"], json!("-loud"));
            assert_eq!(option["kind"], json!("option"));
            assert_eq!(option["field"], json!("options"));

            // Positions are counted over the words as written, options
            // included — so with `-loud` present the declared role at index 0
            // lands on the flag, and the tab shows exactly that rather than
            // what the author hoped. Without it, the role is where it reads.
            let report = bench.analyse("greet world\n");
            let tokens = report["tokens"].as_array().expect("tokens");
            let value = &tokens[1];
            assert_eq!(value["text"], json!("world"));
            assert_eq!(value["field"], json!("arg_roles"));
            let roles: Vec<&str> = value["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .filter_map(|r| r["role"].as_str())
                .collect();
            assert!(roles.contains(&"Name"), "{roles:?}");
        });
    }

    #[test]
    fn inspecting_a_word_names_the_resolved_spec_and_its_origin() {
        bench_on(PACK, |bench| {
            let sample = "greet world\n";
            let view = bench.inspect(sample, 0).expect("the head word");
            assert_eq!(view["spec"]["name"], json!("greet"));
            assert_eq!(view["spec"]["origin"], json!("pack"));
            assert_eq!(view["spec"]["arity"], json!("1..2"));
            assert_eq!(view["spec"]["summary"], json!("Say hello to someone."));
            assert!(view["editable"].as_bool().expect("editable flag"));

            let view = bench.inspect(sample, 7).expect("the argument word");
            assert_eq!(view["word"]["text"], json!("world"));
            assert_eq!(view["role"]["field"], json!("arg_roles"));
        });
    }

    #[test]
    fn a_body_argument_is_walked_into() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("withbody { greet inner }\n");
            let tokens = report["tokens"].as_array().expect("tokens");
            let inner: Vec<&str> = tokens
                .iter()
                .filter(|t| t["depth"] == json!(1))
                .filter_map(|t| t["text"].as_str())
                .collect();
            assert_eq!(
                inner,
                vec!["greet", "inner"],
                "the pack's declared body must be walked into: {tokens:?}"
            );
        });
    }

    #[test]
    fn a_shipped_body_argument_is_walked_into_too() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("proc p {} {\n    greet you\n}\n");
            let tokens = report["tokens"].as_array().expect("tokens");
            assert!(
                tokens
                    .iter()
                    .any(|t| t["text"] == json!("greet") && t["depth"] == json!(1)),
                "{tokens:?}"
            );
        });
    }

    #[test]
    fn the_render_chunks_reassemble_the_sample_exactly() {
        bench_on(PACK, |bench| {
            let sample = "greet «wörld»\nproc p {} {\n    greet x\n}\n";
            let report = bench.analyse(sample);
            let rebuilt: String = report["render"]
                .as_array()
                .expect("render")
                .iter()
                .filter_map(|c| c["text"].as_str())
                .collect();
            assert_eq!(rebuilt, sample);
        });
    }

    /// The bug the browser caught: a body's own statements have to be
    /// *rendered*, not swallowed by the braced word that holds them.
    #[test]
    fn a_nested_statement_gets_its_own_render_chunk() {
        bench_on(PACK, |bench| {
            let sample = "withbody {\n    greet inner\n}\n";
            let report = bench.analyse(sample);
            let tokens = report["tokens"].as_array().expect("tokens");
            let chunks = report["render"].as_array().expect("render");
            let inner = tokens
                .iter()
                .position(|t| t["text"] == json!("greet") && t["depth"] == json!(1))
                .expect("the nested call is a token");
            assert!(
                chunks.iter().any(|c| c["token"] == json!(inner)),
                "the nested word must be clickable in its own right: {chunks:?}"
            );
            let rebuilt: String = chunks.iter().filter_map(|c| c["text"].as_str()).collect();
            assert_eq!(rebuilt, sample);
            // And the body word still owns its braces, so clicking one explains
            // why the argument is a script at all.
            let body = tokens
                .iter()
                .position(|t| t["index"] == json!(1) && t["depth"] == json!(0))
                .expect("the body word");
            assert!(
                chunks.iter().any(|c| {
                    c["token"] == json!(body)
                        && c["text"].as_str().is_some_and(|t| t.starts_with('{'))
                }),
                "{chunks:?}"
            );
        });
    }

    #[test]
    fn a_shipped_command_reports_its_own_declaration() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("set x 1\n");
            let tokens = report["tokens"].as_array().expect("tokens");
            assert_eq!(tokens[0]["origin"], json!("builtin"));
            let roles: Vec<&str> = tokens[1]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .filter_map(|r| r["role"].as_str())
                .collect();
            assert!(roles.contains(&"VarWrite"), "{roles:?}");
            // `set` computes its roles dynamically (a one-argument `set` reads
            // the variable, a two-argument one writes it), and the report says
            // so rather than pointing at a static table that is not there.
            assert_eq!(tokens[1]["field"], json!("arg_role_resolver"));
        });
    }

    #[test]
    fn a_subcommand_word_is_named_as_one() {
        bench_on(PACK, |bench| {
            let report = bench.analyse("string length abc\n");
            let tokens = report["tokens"].as_array().expect("tokens");
            assert_eq!(tokens[1]["text"], json!("length"));
            assert_eq!(tokens[1]["kind"], json!("subcommand"));
            assert_eq!(tokens[1]["field"], json!("subcommands"));
        });
    }

    #[test]
    fn an_empty_pack_analyses_against_the_plain_registry() {
        bench_on("speclib empty 1.0 {\n}\n", |bench| {
            let report = bench.analyse("set x 1\n");
            assert_eq!(report["installed"], json!(false));
            assert_eq!(report["summary"]["unknown_commands"], json!(0));
        });
    }

    #[test]
    fn a_shadowed_declaration_resolves_to_the_shipped_spec() {
        // No `-override`, so the shipped `lsort` is what an editor uses — and
        // the Test tab has to say so rather than showing the pack's.
        bench_on(
            "speclib demo 1.0 {\ncommand lsort {\n    arity 9\n}\n}\n",
            |bench| {
                let view = bench.inspect("lsort {a b}\n", 0).expect("the head word");
                assert_eq!(view["spec"]["origin"], json!("shadowed"));
                assert_ne!(view["spec"]["arity"], json!("9"));
            },
        );
    }
}
