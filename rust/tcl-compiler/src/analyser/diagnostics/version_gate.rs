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

//! Version-aware diagnostics (**W135** / **W136**, and the argument-DSL
//! rung **W137** / **W138** / **W200**).
//!
//! A command, subcommand, second-level subcommand, option, or literal
//! argument value carries a [`Lifecycle`] on either its owning package's
//! version axis (Tk, a tcllib package, `argparse`, …) or the core Tcl axis:
//! the introducing release, the deprecating release, and the retiring
//! release. The resolved floor — the
//! profile's library pin raised by any versioned `package require`, or the
//! active Tcl profile raised by `package require Tcl` — is checked against all
//! three:
//!
//! * floor below `introduced` ⇒ not available yet (W135 command/subcommand
//!   /argument value, W136 option);
//! * floor at or past `retired` ⇒ gone (W139) — the boundary is
//!   **exclusive**, so `retired: 10.0.0` means 10.0.0 no longer has it;
//! * floor at or past `deprecated` while still available ⇒ W144.
//!
//! The argument mini-languages get the same treatment one rung deeper
//! (design doc §6): a `string is` class ([`ArgValue::min_tcl`], W137), a
//! `format`/`scan` conversion (W138), or a `binary format`/`scan` size
//! modifier (W200) can need a newer **Tcl core** than the dialect's
//! effective version ([`Analyser::effective_dsl_version`]).
//!
//! Every floor is a whole-file fact: `package require` may appear anywhere,
//! so candidate uses are buffered during the walk and decided post-walk once
//! every `package require` is known.  An unpinned package required *without*
//! a version is permissive, and a package not required at all is the domain
//! of W120 (missing `package require`).
//!
//! A `package require` that states a **range** (`1.0-3.0`) guarantees only
//! that the loaded version is somewhere inside it, so the floor alone can be
//! satisfied while part of the accepted range is not. When the window's far
//! end reaches a retirement the same W139 is emitted with a hedged message —
//! *not available in every version satisfying the requirement* — rather than
//! a new code (see [`Analyser::requirement_straddle_diagnostic`]).
//!
//! One word draws one diagnostic. A subcommand the active profile does not
//! have at all belongs to W002, exactly as a missing `package require`
//! belongs to W120, so the version gate skips it and its inner gates.
//!
//! [`ArgValue::min_tcl`]: tcl_registry::ArgValue

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::ProfileQueries as _;
use tcl_registry::deprecation::{
    DeprecationFixContext, DeprecationFixSafety, DeprecationFixTarget, DeprecationFixWord,
};
use tcl_registry::dialects::DialectSet;
use tcl_registry::lifecycle::{Lifecycle, LifecycleState};

use super::super::state::Analyser;
use super::super::types::{Diagnostic, Severity};

/// A lifecycle-bearing registry use, recorded during the walk and checked
/// post-walk against its package or core-Tcl version floor.
#[derive(Debug)]
pub(in crate::analyser) struct VersionGateSite {
    /// Span the diagnostic anchors to (command head, or option token).
    span: Span,
    /// The version axis that governs this lifecycle declaration.
    axis: VersionGateAxis,
    /// The declared lifecycle on that package's version axis.
    lifecycle: Lifecycle,
    /// What is gated — a command, subcommand, option, or argument value.
    item: VersionGateItem,
    /// Complete generic invocation context for a registry deprecation-fix hook.
    fix_payload: DeprecationFixPayload,
    /// Active profile name, supplied to a context-aware registry hook.
    dialect: &'static str,
}

/// A proven **W147** option conflict whose [`OptionConstraint`] is
/// version-gated, held until the whole-file floor is known.
///
/// A relationship such as `option_conflict {-a -b} -introduced 2.0` does not
/// exist below 2.0, so enforcing it against a file pinned to 1.x would report
/// a rule that release does not have. A constraint with an
/// [`Lifecycle::UNSPECIFIED`] lifecycle never reaches this buffer: it holds in
/// every release and is queued inline onto [`Analyser::pending_arity`] at the
/// dispatch site, exactly as before.
///
/// The fields after `lifecycle` are the `pending_arity` tuple this becomes
/// once [`Analyser::flush_gated_option_conflicts`] decides the relationship
/// exists — the version gate is a filter in front of the ordinary queue, not
/// a second reporting path.
///
/// [`OptionConstraint`]: tcl_registry::OptionConstraint
#[derive(Debug, Clone, PartialEq)]
pub(in crate::analyser) struct GatedOptionConflict {
    /// The axis governing the constraint's lifecycle, or `None` when the
    /// owning spec sits on no version axis at all (permissive).
    pub(in crate::analyser) axis: Option<VersionGateAxis>,
    /// The relationship's declared lifecycle on that axis.
    pub(in crate::analyser) lifecycle: Lifecycle,
    /// Base command name the post-walk arity flush resolves shadowing against.
    pub(in crate::analyser) resolution_name: String,
    /// Call-site command-resolution namespace.
    pub(in crate::analyser) namespace: String,
    /// Whether a shadowing definition must lexically precede the call.
    pub(in crate::analyser) enforce_order: bool,
    /// The W147 diagnostic, already fully formed.
    pub(in crate::analyser) diagnostic: Diagnostic,
}

/// Version axes supported by the generic lifecycle consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::analyser) enum VersionGateAxis {
    /// A package whose version comes from a profile pin and/or `package require`.
    Package(&'static str),
    /// The Tcl core version selected by the active dialect profile.
    TclCore,
}

impl VersionGateAxis {
    const fn name(self) -> &'static str {
        match self {
            Self::Package(package) => package,
            Self::TclCore => "Tcl",
        }
    }
}

/// Owned word fact retained until the whole-file lifecycle floor is known.
#[derive(Debug)]
struct StoredDeprecationFixWord {
    spelling: String,
    literal: bool,
}

/// The generic syntax context a lifecycle-fix hook may target.
#[derive(Debug)]
struct DeprecationFixPayload {
    invocation: Span,
    word_spans: Vec<Option<Span>>,
    words: Vec<StoredDeprecationFixWord>,
    matched_word_index: usize,
}

/// One registry invocation while its lifecycle sites are being recorded.
#[derive(Clone, Copy)]
struct LifecycleInvocation<'a> {
    command: &'a str,
    command_token: Token,
    args: &'a [String],
    arg_tokens: &'a [Token],
    axis: VersionGateAxis,
}

fn deprecation_fix_payload(
    command: &str,
    command_token: Token,
    args: &[String],
    arg_tokens: &[Token],
    matched_word_index: usize,
    source_map: &tcl_lexer::SourceMap<'_>,
) -> DeprecationFixPayload {
    let command_span = super::super::utils::full_word_span_in(source_map, command_token);
    let mut word_spans = vec![Some(command_span)];
    let mut words = vec![StoredDeprecationFixWord {
        spelling: command.to_owned(),
        literal: true,
    }];
    for (index, arg) in args.iter().enumerate() {
        let token = arg_tokens.get(index).copied();
        word_spans
            .push(token.map(|token| super::super::utils::full_word_span_in(source_map, token)));
        words.push(StoredDeprecationFixWord {
            spelling: arg.clone(),
            literal: token
                .is_some_and(|token| !matches!(token.kind, TokenType::Var | TokenType::Cmd)),
        });
    }
    let end = word_spans
        .iter()
        .flatten()
        .last()
        .map_or(command_span.end(), |span| span.end());
    DeprecationFixPayload {
        invocation: Span::new(command_span.start(), end),
        word_spans,
        words,
        matched_word_index,
    }
}

/// Payload distinguishing the package-version gate's syntax granularity.
#[derive(Debug, Clone)]
enum VersionGateItem {
    Command(String),
    Subcommand {
        command: String,
        subcommand: String,
    },
    /// A second-level operation of a two-level ensemble (`info object class`).
    SubSubCommand {
        command: String,
        subcommand: String,
        sub_subcommand: String,
    },
    Option {
        command: String,
        option: String,
    },
    /// A literal positional value. `subcommand` is `None` for a value gated
    /// directly on the command (`CommandSpec::versioned_arg_values` and the
    /// command's own `arg_values`), `Some` for one scoped to a subcommand.
    ArgumentValue {
        command: String,
        subcommand: Option<String>,
        value: String,
    },
}

/// The per-value lifecycle gates of one `arg_values` table, merged with the
/// positional `versioned_arg_values` gates that apply to the same table.
///
/// Both fields describe the same axis, so a value carrying each is gated by
/// the stricter of the two ([`Lifecycle::intersect`]) and recorded once —
/// exactly what `arg_value_available_for_version` answers. Values with no
/// lifecycle at all are dropped here so the walk only visits real gates.
fn arg_value_gates(
    arg_values: &'static [(u8, &'static [tcl_registry::ArgValue])],
    versioned: &'static [tcl_registry::spec::VersionedArgValue],
) -> Vec<(u8, &'static str, Lifecycle)> {
    // The overwhelmingly common case is a command with no gated values at
    // all; recording runs per dispatch site, so bail before allocating.
    if versioned.is_empty()
        && !arg_values
            .iter()
            .any(|(_, values)| values.iter().any(|v| !v.lifecycle.is_unspecified()))
    {
        return Vec::new();
    }
    let mut gates: Vec<(u8, &'static str, Lifecycle)> = Vec::new();
    for (index, values) in arg_values {
        for value in *values {
            if !value.lifecycle.is_unspecified() {
                gates.push((*index, value.value, value.lifecycle));
            }
        }
    }
    for gate in versioned {
        match gates
            .iter_mut()
            .find(|(index, value, _)| *index == gate.index && *value == gate.value)
        {
            Some((_, _, lifecycle)) => *lifecycle = lifecycle.intersect(gate.lifecycle),
            None => gates.push((gate.index, gate.value, gate.lifecycle)),
        }
    }
    gates
}

fn is_literal_option(arg: &str, token: Option<&Token>) -> bool {
    if !arg.starts_with('-') || arg.len() < 2 {
        return false;
    }
    // Negative-number literals (`-1`, `-1.5`) are positional values, not
    // options.
    let rest = arg[1..].trim_start_matches('-');
    if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }
    // A `Var`/`Cmd` token's text is not a literal option name.
    !token.is_some_and(|tok| matches!(tok.kind, TokenType::Var | TokenType::Cmd))
}

/// Human phrase naming the gated syntax, reused by every lifecycle message.
fn item_phrase(item: &VersionGateItem) -> String {
    match item {
        VersionGateItem::Command(cmd) => format!("'{cmd}'"),
        VersionGateItem::Subcommand {
            command,
            subcommand,
        } => format!("Subcommand '{subcommand}' on '{command}'"),
        VersionGateItem::Option { command, option } => {
            format!("Option '{option}' on '{command}'")
        }
        VersionGateItem::SubSubCommand {
            command,
            subcommand,
            sub_subcommand,
        } => format!("Subcommand '{sub_subcommand}' on '{command} {subcommand}'"),
        VersionGateItem::ArgumentValue {
            command,
            subcommand,
            value,
        } => match subcommand {
            Some(subcommand) => format!("Argument value '{value}' on '{command} {subcommand}'"),
            None => format!("Argument value '{value}' on '{command}'"),
        },
    }
}

/// W136 is the option-granularity twin of W135; every other item uses W135.
fn not_introduced_code(item: &VersionGateItem) -> DiagCode {
    match item {
        VersionGateItem::Option { .. } => DiagCode::W136,
        _ => DiagCode::W135,
    }
}

/// The diagnostic for `site` at the resolved `floor`, or `None` when the
/// lifecycle is satisfied there.
///
/// The three states are independently reportable and share one exclusive
/// retirement rule (`floor >= retired` ⇒ gone), so a retired item never also
/// reports as deprecated.
fn version_gate_diagnostic(
    site: &VersionGateSite,
    floor: &str,
    guarantee: &str,
) -> Option<(DiagCode, String)> {
    let package = site.axis.name();
    let what = item_phrase(&site.item);
    match site.lifecycle.state_at(Some(floor)) {
        LifecycleState::Available => None,
        LifecycleState::NotIntroduced => {
            let version = site.lifecycle.introduced?;
            Some((
                not_introduced_code(&site.item),
                format!("{what} requires {package} {version} but {guarantee}."),
            ))
        }
        LifecycleState::Retired => {
            let version = site.lifecycle.retired?;
            Some((
                DiagCode::W139,
                format!("{what} was removed in {package} {version} but {guarantee}."),
            ))
        }
        LifecycleState::Deprecated => {
            let version = site.lifecycle.deprecated?;
            Some((
                DiagCode::W144,
                format!("{what} is deprecated as of {package} {version}; {guarantee}."),
            ))
        }
    }
}

impl Analyser {
    /// The version axis that governs every [`Lifecycle`] declared on `spec`
    /// or on anything hanging off it — subcommands, options, argument values,
    /// and option *relationships*.
    ///
    /// On a keyed ambient axis (the F5 surfaces) a vendor-own spec needs no
    /// `required_package` to sit on the axis: its pin resolves through the
    /// profile's vendor bit. Otherwise the owning package names the axis, and
    /// a lifecycle on an otherwise unowned Tcl command is governed by the
    /// selected core Tcl profile rather than a fictitious `package require` —
    /// which keeps every core command generic while letting an explicit
    /// `package require Tcl` raise the floor.
    ///
    /// `None` when nothing puts the spec on an axis; every consumer treats
    /// that as permissive, matching the "no version known ⇒ do not gate" rule.
    pub(in crate::analyser) fn lifecycle_axis(
        &self,
        spec: &tcl_registry::CommandSpec,
    ) -> Option<VersionGateAxis> {
        self.profile
            .keyed_pin_for(spec)
            .map(|pin| pin.package)
            .or_else(|| spec.owning_package())
            .map(VersionGateAxis::Package)
            .or_else(|| {
                spec.supports_dialect(DialectSet::ALL_TCL)
                    .then_some(VersionGateAxis::TclCore)
            })
    }

    fn record_lifecycle_site(
        &mut self,
        span: Span,
        axis: VersionGateAxis,
        lifecycle: Lifecycle,
        item: VersionGateItem,
        fix_payload: DeprecationFixPayload,
    ) {
        if lifecycle.is_unspecified() {
            return;
        }
        self.version_gate_sites.push(VersionGateSite {
            span,
            axis,
            lifecycle,
            item,
            fix_payload,
            dialect: self.profile.name,
        });
    }

    fn record_subcommand_version_sites(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        spec: &tcl_registry::CommandSpec,
        sub: &tcl_registry::SubCommand,
    ) {
        // A subcommand the active profile does not have at all is W002's,
        // exactly as a missing `package require` is W120's: one word, one
        // diagnostic. Its inner gates are moot for the same reason.
        if !self.profile.is_subcommand_available(spec, sub) {
            return;
        }
        let sub_is_literal = invocation
            .arg_tokens
            .first()
            .is_some_and(|tok| !matches!(tok.kind, TokenType::Var | TokenType::Cmd));
        if sub_is_literal {
            // Span and payload are computed against one hoisted `SourceMap`
            // and bound to locals before the `&mut self` call below, so the
            // map's immutable borrow of `self.source` has already ended.
            let (span, payload) = {
                let source_map = self.cached_source_map();
                (
                    super::super::utils::full_word_span_in(&source_map, invocation.arg_tokens[0]),
                    deprecation_fix_payload(
                        invocation.command,
                        invocation.command_token,
                        invocation.args,
                        invocation.arg_tokens,
                        1,
                        &source_map,
                    ),
                )
            };
            self.record_lifecycle_site(
                span,
                invocation.axis,
                sub.lifecycle,
                VersionGateItem::Subcommand {
                    command: invocation.command.to_owned(),
                    subcommand: sub.name.to_owned(),
                },
                payload,
            );
        }
        if sub_is_literal && !sub.sub_subcommands.is_empty() {
            self.record_sub_subcommand_version_site(invocation, sub);
        }
        self.record_arg_value_version_sites(
            invocation,
            &arg_value_gates(sub.arg_values, sub.versioned_arg_values),
            1,
            Some(sub.name),
        );
    }

    /// Buffer the third-level word of a two-level ensemble (`info object
    /// class`) against its [`SubSubCommand`]'s lifecycle.
    ///
    /// Resolution is the registry's own — exact match or unique prefix — so an
    /// abbreviated `info object prop` is gated exactly as the spelt-out word
    /// is.
    ///
    /// [`SubSubCommand`]: tcl_registry::SubSubCommand
    fn record_sub_subcommand_version_site(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        sub: &tcl_registry::SubCommand,
    ) {
        let (Some(word), Some(tok)) = (invocation.args.get(1), invocation.arg_tokens.get(1)) else {
            return;
        };
        if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
            return;
        }
        let Some(sub_sub) = sub.resolve_sub_subcommand(word) else {
            return;
        };
        // Span and payload are computed against one hoisted `SourceMap` and
        // bound to locals before the `&mut self` call below, so the map's
        // immutable borrow of `self.source` has already ended.
        let (span, payload) = {
            let source_map = self.cached_source_map();
            (
                super::super::utils::full_word_span_in(&source_map, *tok),
                deprecation_fix_payload(
                    invocation.command,
                    invocation.command_token,
                    invocation.args,
                    invocation.arg_tokens,
                    2,
                    &source_map,
                ),
            )
        };
        self.record_lifecycle_site(
            span,
            invocation.axis,
            sub_sub.lifecycle,
            VersionGateItem::SubSubCommand {
                command: invocation.command.to_owned(),
                subcommand: sub.name.to_owned(),
                sub_subcommand: sub_sub.name.to_owned(),
            },
            payload,
        );
    }

    /// Buffer every literal positional word that matches a lifecycle-bearing
    /// declared value.
    ///
    /// `arg_offset` is where the gated positions start in `args` — 0 for a
    /// command-level table, 1 for a subcommand's (whose indices are relative
    /// to the word after the subcommand). `subcommand` names the owning
    /// subcommand for the message, or `None` at command level.
    fn record_arg_value_version_sites(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        gates: &[(u8, &'static str, Lifecycle)],
        arg_offset: usize,
        subcommand: Option<&str>,
    ) {
        for &(index, value, lifecycle) in gates {
            let arg_idx = arg_offset + usize::from(index);
            let (Some(arg), Some(tok)) = (
                invocation.args.get(arg_idx),
                invocation.arg_tokens.get(arg_idx),
            ) else {
                continue;
            };
            if arg != value || matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
                continue;
            }
            let (span, payload) = {
                let source_map = self.cached_source_map();
                (
                    super::super::utils::full_word_span_in(&source_map, *tok),
                    deprecation_fix_payload(
                        invocation.command,
                        invocation.command_token,
                        invocation.args,
                        invocation.arg_tokens,
                        arg_idx + 1,
                        &source_map,
                    ),
                )
            };
            self.record_lifecycle_site(
                span,
                invocation.axis,
                lifecycle,
                VersionGateItem::ArgumentValue {
                    command: invocation.command.to_owned(),
                    subcommand: subcommand.map(ToOwned::to_owned),
                    value: value.to_owned(),
                },
                payload,
            );
        }
    }

    fn record_option_version_sites(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        options: &[tcl_registry::hover::OptionSpec],
        start_idx: usize,
    ) {
        let mut i = start_idx;
        while i < invocation.args.len() {
            let arg = invocation.args[i].as_str();
            if arg == "--" {
                break;
            }
            if !is_literal_option(arg, invocation.arg_tokens.get(i)) {
                i += 1;
                continue;
            }
            if let Some(opt) = options.iter().find(|o| o.matches(arg)) {
                if let Some(tok) = invocation.arg_tokens.get(i) {
                    let (span, payload) = {
                        let source_map = self.cached_source_map();
                        (
                            super::super::utils::full_word_span_in(&source_map, *tok),
                            deprecation_fix_payload(
                                invocation.command,
                                invocation.command_token,
                                invocation.args,
                                invocation.arg_tokens,
                                i + 1,
                                &source_map,
                            ),
                        )
                    };
                    self.record_lifecycle_site(
                        span,
                        invocation.axis,
                        opt.lifecycle,
                        VersionGateItem::Option {
                            command: invocation.command.to_owned(),
                            option: arg.to_owned(),
                        },
                        payload,
                    );
                }
                i += 1 + opt.value_word_count(invocation.args, i);
                continue;
            }
            i += 1;
        }
    }

    /// Buffer package-version-gated syntax uses at a dispatch site.
    ///
    /// The command's [`Lifecycle`] (if declared) records a candidate at the
    /// command head; each option argument matching a lifecycle-bearing
    /// [`OptionSpec`] records one at the option token.
    /// Option scanning mirrors [`Analyser::emit_w004_dialect_invalid_option`]:
    /// it stops at `--`, skips negative-number literals and dynamic
    /// (`Var`/`Cmd`) tokens, and resolves subcommand-scoped options.
    ///
    /// [`OptionSpec`]: tcl_registry::OptionSpec
    pub(in crate::analyser) fn record_version_gate_sites(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        let Some(registry) = self.registry.clone() else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };
        // Command-level gate. On a keyed ambient axis (the F5 surfaces)
        // the effective range applies — an explicit introduction release,
        // or the declared 15.0 baseline, plus any removal release (W139) —
        // and a vendor-own spec needs no `required_package` to sit on the
        // axis (its pin resolves through the profile's vendor bit).
        let keyed = self.profile.keyed_version_range(spec);
        let Some(axis) = self.lifecycle_axis(spec) else {
            return;
        };
        let invocation = LifecycleInvocation {
            command: cmd_name,
            command_token: cmd_tok,
            args,
            arg_tokens,
            axis,
        };
        let effective = keyed.map_or(spec.lifecycle, |(min, max)| Lifecycle {
            introduced: min.or(spec.lifecycle.introduced),
            deprecated: spec.lifecycle.deprecated,
            retired: max.or(spec.lifecycle.retired),
            deprecation_fix: spec.lifecycle.deprecation_fix,
        });
        let (span, payload) = {
            let source_map = self.cached_source_map();
            (
                super::super::utils::full_word_span_in(&source_map, cmd_tok),
                deprecation_fix_payload(cmd_name, cmd_tok, args, arg_tokens, 0, &source_map),
            )
        };
        self.record_lifecycle_site(
            span,
            axis,
            effective,
            VersionGateItem::Command(cmd_name.to_owned()),
            payload,
        );

        // Command-level literal argument values (`HTTP::respond <status>
        // noserver`, a vendor mode word) — the same gate the subcommand arm
        // applies, one level up.
        self.record_arg_value_version_sites(
            invocation,
            &arg_value_gates(spec.arg_values, spec.versioned_arg_values),
            0,
            None,
        );

        // Option-level gates.  Resolve subcommand-scoped options when the first
        // argument names a subcommand.
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| {
                let first = args.first().map(String::as_str).unwrap_or_default();
                spec.resolve_subcommand(first)
            })
            .flatten();

        if let Some(sub) = sub_match {
            self.record_subcommand_version_sites(invocation, spec, sub);
        }
        let (options, start_idx) = match sub_match {
            Some(sub) => (sub.options, 1usize),
            None => (spec.options, 0usize),
        };
        if options.is_empty() {
            return;
        }

        self.record_option_version_sites(invocation, options, start_idx);
    }

    /// Emit W135/W136 for each buffered site whose package's resolved
    /// version floor is below the required `min_version`. The floor comes
    /// from an explicit versioned `package require`, or — for a package the
    /// active profile pins (§7.1 axis C) — from the profile pin (the shipped
    /// Tk on a plain Tcl base, a `Keyed` vendor surface at its D5
    /// oldest-supported default). Sites with no floor at all (unpinned +
    /// required without a version, or not required — the latter handled by
    /// W120) are skipped.
    pub(in crate::analyser) fn flush_version_gate_diagnostics(&mut self) {
        if self.version_gate_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.version_gate_sites);
        let mut new_diags: Vec<Diagnostic> = Vec::new();
        for site in sites {
            let Some((floor, guarantee)) = self.axis_floor(site.axis) else {
                continue;
            };
            // A range that reaches a retirement outranks the floor's own
            // verdict *while the floor is satisfied*: "gone in part of the
            // accepted range" is the stronger fact than "deprecated here".
            // A floor that already fails keeps its own, more specific message.
            let Some((code, message)) = self
                .requirement_straddle_diagnostic(&site, &floor)
                .or_else(|| version_gate_diagnostic(&site, &floor, &guarantee))
            else {
                continue;
            };
            let fixes = (code == DiagCode::W144)
                .then(|| lifecycle_deprecation_fix(&site, &floor))
                .flatten()
                .into_iter()
                .collect();
            new_diags.push(Diagnostic {
                code,
                span: site.span,
                message,
                severity: Severity::Warning,
                fixes,
            });
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// The resolved version floor on `axis`, with the phrase naming what
    /// guarantees it.
    ///
    /// `None` = no floor is resolvable (an unpinned package required without
    /// a version, or a permissive profile with no core version) — the
    /// standing "no version known ⇒ do not gate" case.
    fn axis_floor(&self, axis: VersionGateAxis) -> Option<(String, String)> {
        match axis {
            VersionGateAxis::Package(package) => {
                self.package_version_floor(package).map(|(floor, source)| {
                    let guarantee = match source {
                        FloorSource::Require => {
                            format!("`package require` guarantees only {floor}")
                        }
                        FloorSource::ProfilePin => {
                            format!("{} ships {package} {floor}", self.profile.name)
                        }
                    };
                    (floor, guarantee)
                })
            }
            VersionGateAxis::TclCore => self.effective_dsl_version().map(|version| {
                let floor = version.as_package_version().to_owned();
                (
                    floor.clone(),
                    format!("{} targets Tcl {floor}", self.profile.name),
                )
            }),
        }
    }

    /// Buffer a *proven* option conflict whose [`OptionConstraint`] carries a
    /// lifecycle, to be decided once the whole-file floor is known.
    ///
    /// The caller has already established that the conflict is violated and
    /// built the diagnostic; the only open question is whether the
    /// relationship *exists* at the release this file targets, and that is a
    /// post-walk fact for the same reason every other lifecycle check is one
    /// — `package require` may appear anywhere.
    ///
    /// [`OptionConstraint`]: tcl_registry::OptionConstraint
    pub(in crate::analyser) fn record_gated_option_conflict(
        &mut self,
        resolution_name: &str,
        lifecycle: Lifecycle,
        namespace: String,
        enforce_order: bool,
        diagnostic: Diagnostic,
    ) {
        // The constraint's axis is its owning command's — a relationship
        // between two of that command's options ages on the same axis the
        // options themselves do.
        let axis = self.registry.clone().and_then(|registry| {
            registry
                .get(resolution_name)
                .and_then(|spec| self.lifecycle_axis(spec))
        });
        self.pending_option_conflicts.push(GatedOptionConflict {
            axis,
            lifecycle,
            resolution_name: resolution_name.to_owned(),
            namespace,
            enforce_order,
            diagnostic,
        });
    }

    /// Promote every buffered gated option conflict the resolved floor
    /// actually has onto [`Analyser::pending_arity`], and drop the rest.
    ///
    /// Runs *before* [`Analyser::flush_arity_diagnostics`], so a promoted
    /// conflict goes through exactly the shadowing / definition-order
    /// suppression an ungated one does — the version gate decides whether the
    /// relationship exists, never whether the call resolves to a builtin.
    ///
    /// Permissive in both directions the registry is permissive: a spec on no
    /// version axis, and an axis with no resolvable floor, both report the
    /// conflict. A *deprecated* relationship still exists, so it is reported
    /// too — deprecation is not absence.
    pub(in crate::analyser) fn flush_gated_option_conflicts(&mut self) {
        if self.pending_option_conflicts.is_empty() {
            return;
        }
        for conflict in std::mem::take(&mut self.pending_option_conflicts) {
            let floor = conflict
                .axis
                .and_then(|axis| self.axis_floor(axis))
                .map(|(floor, _guarantee)| floor);
            if !conflict.lifecycle.available_at(floor.as_deref()) {
                continue;
            }
            self.pending_arity.push((
                conflict.resolution_name,
                conflict.namespace,
                conflict.enforce_order,
                conflict.diagnostic,
            ));
        }
    }

    /// The hedged diagnostic for a site whose floor passes but whose
    /// `package require` **range** does not stay inside the lifecycle.
    ///
    /// `package require Foo 1.0-3.0` guarantees only that the loaded Foo is
    /// somewhere in `[1.0, 3.0)`; the floor check asks about `1.0` alone. When
    /// the window's far end reaches a retirement the item is missing from part
    /// of the accepted range, which is a real portability fault the floor
    /// check cannot see — so it is reported with the ordinary
    /// [`DiagCode::W139`] and a message that says *not in every version*
    /// rather than claiming the item is gone.
    ///
    /// Only a stated `a-b` range participates ([`requirement_upper_bound`]);
    /// `a` and `a-` state no ceiling, and a degenerate `a-a` pin is a single
    /// version the floor check already decided.
    ///
    /// [`requirement_upper_bound`]: tcl_registry::version::requirement_upper_bound
    fn requirement_straddle_diagnostic(
        &self,
        site: &VersionGateSite,
        floor: &str,
    ) -> Option<(DiagCode, String)> {
        if !site.lifecycle.state_at(Some(floor)).is_available() {
            return None;
        }
        let package = site.axis.name();
        let retired = site.lifecycle.retired?;
        let (requirement, ceiling) = self.package_requirement_ceiling(package)?;
        // A window that never opens above its floor is a pin, not a range.
        if tcl_registry::version::compare(&ceiling, floor).is_le() {
            return None;
        }
        // The ceiling is exclusive, so the window reaches the retirement only
        // when it extends strictly past it.
        if tcl_registry::version::compare(&ceiling, retired).is_le() {
            return None;
        }
        let what = item_phrase(&site.item);
        Some((
            DiagCode::W139,
            format!(
                "{what} is not available in every version satisfying \
                 requirement `{requirement}`: removed in {package} {retired}."
            ),
        ))
    }

    /// The tightest **stated** upper bound over this file's unconditional
    /// `package require <pkg> <req>` lines, with the requirement that stated
    /// it.
    ///
    /// Several requires must all hold of the one loaded version, so the
    /// accepted window is their intersection and the lowest stated ceiling
    /// wins. Requirements stating no ceiling (`a`, `a-`) contribute nothing.
    /// Conditional probes are excluded for the same reason the floor excludes
    /// them: they guarantee nothing on every path.
    fn package_requirement_ceiling(&self, pkg: &str) -> Option<(String, String)> {
        self.result
            .package_requires
            .iter()
            .filter(|r| r.name == pkg && !r.conditional)
            .filter_map(|r| {
                let requirement = r.version.as_deref()?;
                let ceiling = tcl_registry::version::requirement_upper_bound(requirement)?;
                Some((requirement.to_owned(), ceiling.to_owned()))
            })
            .min_by(|a, b| tcl_registry::version::compare(&a.1, &b.1))
    }

    /// The resolved version floor for `pkg`, and where it came from.
    ///
    /// The base is the active profile's library pin (§7.1: `TracksBase` →
    /// the embedded runtime version, `Pinned` → the shipped version,
    /// `Keyed` → the session override or the D5 oldest-supported default).
    /// The highest *guaranteed* lower bound among this file's
    /// `package require <pkg> <req>` lines can only **raise** that floor —
    /// an explicit require never lowers what the runtime already ships.
    /// `None` when `pkg` is unpinned and not required with a version
    /// (permissive — every version is accepted).
    ///
    /// Conditional requires — an optional probe such as
    /// `catch {package require Tk 8.7}` or a `package require` inside an `if`
    /// arm — are excluded: they do not guarantee the version on every path, so
    /// counting them would raise the floor and wrongly suppress a real W135/W136.
    fn package_version_floor(&self, pkg: &str) -> Option<(String, FloorSource)> {
        let has_unconditional_require = self
            .result
            .package_requires
            .iter()
            .any(|r| r.name == pkg && !r.conditional);
        let require_floor = self
            .result
            .package_requires
            .iter()
            .filter(|r| r.name == pkg && !r.conditional)
            .filter_map(|r| r.version.as_deref())
            .map(tcl_registry::version::requirement_lower_bound)
            .max_by(|a, b| tcl_registry::version::compare(a, b));
        // An **ambient** pin (the F5 surfaces) is part of the runtime — its
        // floor always applies. A **hosted** pin (Tk / Itcl on plain Tcl)
        // floors only once the package is actually in play via a require:
        // the missing-require case stays W120's alone, never double-flagged
        // with a version diagnostic.
        let pin_applies = self
            .profile
            .library_pin(pkg)
            .is_some_and(|pin| pin.ambient || has_unconditional_require);
        let pin_floor = pin_applies
            .then(|| self.profile.library_floor(pkg, &self.library_versions))
            .flatten();
        match (pin_floor, require_floor) {
            (Some(pin), Some(req)) => {
                if tcl_registry::version::compare(req, pin).is_gt() {
                    Some((req.to_owned(), FloorSource::Require))
                } else {
                    Some((pin.to_owned(), FloorSource::ProfilePin))
                }
            }
            (Some(pin), None) => Some((pin.to_owned(), FloorSource::ProfilePin)),
            (None, Some(req)) => Some((req.to_owned(), FloorSource::Require)),
            (None, None) => None,
        }
    }
}

/// Resolve a registry-owned lifecycle deprecation hook to an analyser edit.
///
/// The hook's typed target is mapped only through syntax captured while the
/// generic registry walk recognised the lifecycle-bearing word; there is no
/// command-specific fallback. An unavailable target (for example a malformed
/// call missing the requested companion argument) deliberately abstains.
fn lifecycle_deprecation_fix(
    site: &VersionGateSite,
    floor: &str,
) -> Option<super::super::types::CodeFix> {
    let words: Vec<DeprecationFixWord<'_>> = site
        .fix_payload
        .words
        .iter()
        .map(|word| DeprecationFixWord {
            spelling: &word.spelling,
            literal: word.literal,
        })
        .collect();
    let resolved = site
        .lifecycle
        .deprecation_fix?
        .resolve(DeprecationFixContext {
            words: &words,
            matched_word_index: site.fix_payload.matched_word_index,
            dialect: Some(site.dialect),
            effective_version: Some(floor),
        })?;
    let span = match resolved.target {
        DeprecationFixTarget::Word(index) => site.fix_payload.word_spans.get(index).copied()??,
        DeprecationFixTarget::Invocation => site.fix_payload.invocation,
    };
    let safety = match resolved.safety {
        DeprecationFixSafety::SemanticsEquivalent => {
            crate::irules_checks::FixSafety::SemanticsEquivalent
        }
        DeprecationFixSafety::RequiresReview => crate::irules_checks::FixSafety::RequiresReview,
    };
    Some(super::super::types::CodeFix {
        span,
        new_text: resolved.new_text,
        description: resolved.description,
        safety,
    })
}

/// Where a resolved package-version floor came from — an explicit
/// `package require`, or the active profile's library pin (§7.1).
#[derive(Debug, Clone, Copy)]
enum FloorSource {
    /// A versioned, unconditional `package require` in the file.
    Require,
    /// The profile's [`tcl_dialect::LibraryPin`].
    ProfilePin,
}

/// An argument-DSL use gated behind a Tcl release (design doc §6: a
/// `string is` class, a `format`/`scan` conversion), buffered during the
/// walk and decided post-walk against
/// [`Analyser::effective_dsl_version`] — like [`VersionGateSite`], the
/// deciding floor (`package require Tcl`) is a whole-file fact.
#[derive(Debug)]
pub(in crate::analyser) struct DslGateSite {
    /// Span the diagnostic anchors to.
    pub(in crate::analyser) span: Span,
    /// The W-code to emit (W137 for argument values, W138 for
    /// format/scan conversions).
    pub(in crate::analyser) code: DiagCode,
    /// Fully-formed message minus the version comparison tail.
    pub(in crate::analyser) what: String,
    /// The lowest Tcl release that accepts the feature.
    pub(in crate::analyser) min: tcl_dialect::TclVersion,
}

impl Analyser {
    /// The Tcl version the argument mini-languages validate against
    /// (§6.1): the profile's runtime base, raised to any unconditional
    /// `package require Tcl` floor in the file. `None` = permissive
    /// (the unknown-dialect fallback / non-Tcl profiles) — every DSL
    /// check abstains.
    pub(in crate::analyser) fn effective_dsl_version(&self) -> Option<tcl_dialect::TclVersion> {
        let tcl_floor = self
            .result
            .package_requires
            .iter()
            .filter(|r| r.name == "Tcl" && !r.conditional)
            .filter_map(|r| r.version.as_deref())
            .filter_map(|v| {
                tcl_dialect::TclVersion::from_package_version(
                    tcl_registry::version::requirement_lower_bound(v),
                )
            })
            .max();
        self.profile.effective_tcl_version(tcl_floor)
    }

    /// Buffer format/scan %-string DSL uses at a dispatch site — the registry
    /// locates the format-string words *and* names the mini-language each is
    /// written in ([`CommandRegistry::format_string_args`]), so no command
    /// name is matched here.
    ///
    /// The family check is load-bearing, not decoration: `clock`'s field
    /// string, `binary`'s cursor spec, and `regsub`'s backreference template
    /// all sit at [`ArgRole::FormatString`] / [`ArgRole::ScanFormat`]
    /// positions too, and none of them is a printf %-string. Running the
    /// sprintf version gate over `clock format $t -format {%b}` would report
    /// a Tcl 8.6 requirement for a conversion that has nothing to do with
    /// `format`'s `%b`. Only [`FormatType::Sprintf`] words are gated here;
    /// the other families have no version-gated conversion table modelled
    /// yet, so they are deliberately left alone rather than guessed at.
    ///
    /// [`ArgRole::FormatString`]: tcl_registry::arg_role::ArgRole
    /// [`ArgRole::ScanFormat`]: tcl_registry::arg_role::ArgRole
    /// [`CommandRegistry::format_string_args`]: tcl_registry::CommandRegistry::format_string_args
    /// [`FormatType::Sprintf`]: tcl_registry::patterns::FormatType
    pub(in crate::analyser) fn record_dsl_format_sites(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        use tcl_registry::patterns::FormatType;
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        for found in registry.format_string_args(cmd_name, &arg_strs) {
            if found.kind != FormatType::Sprintf {
                continue;
            }
            let (Some(fmt), Some(tok)) = (args.get(found.index), arg_tokens.get(found.index))
            else {
                continue;
            };
            // A dynamic token's text is not the literal %-string.
            if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
                continue;
            }
            if found.scan {
                for (_, feature, min) in tcl_syntax::scan::version_gated_uses(fmt) {
                    self.dsl_gate_sites.push(DslGateSite {
                        span: tok.span,
                        code: DiagCode::W138,
                        what: format!("`scan` conversion {feature} in '{cmd_name}'"),
                        min,
                    });
                }
            } else {
                for use_ in tcl_syntax::format::version_gated_uses(fmt) {
                    self.dsl_gate_sites.push(DslGateSite {
                        span: tok.span,
                        code: DiagCode::W138,
                        what: format!("`format` conversion {} in '{cmd_name}'", use_.feature),
                        min: use_.min,
                    });
                }
            }
        }
    }

    /// Emit W137/W138 for each buffered argument-DSL site whose feature
    /// needs a newer Tcl than the file's effective version (§6).
    pub(in crate::analyser) fn flush_dsl_gate_diagnostics(&mut self) {
        if self.dsl_gate_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.dsl_gate_sites);
        let Some(effective) = self.effective_dsl_version() else {
            return; // permissive profile — abstain
        };
        let mut new_diags: Vec<Diagnostic> = Vec::new();
        for site in sites {
            if site.min <= effective {
                continue;
            }
            new_diags.push(Diagnostic {
                code: site.code,
                span: site.span,
                message: format!(
                    "{} requires Tcl {} but {} provides {}.",
                    site.what,
                    site.min.as_package_version(),
                    self.profile.name,
                    effective.as_package_version()
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        self.result.diagnostics.extend(new_diags);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::state::Analyser;

    /// `(code, message)` pairs for the version-gate codes only.
    fn version_diags(source: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W135" | "W136"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn fires(source: &str, code: &str) -> bool {
        version_diags(source).iter().any(|(c, _)| c == code)
    }

    fn count(source: &str, code: &str) -> usize {
        version_diags(source)
            .iter()
            .filter(|(c, _)| c == code)
            .count()
    }

    #[test]
    fn option_value_that_looks_like_a_flag_is_not_retested() {
        // `-placeholder`'s value is itself `-placeholder`. The
        // value word must be skipped, so exactly ONE W136 fires (the option),
        // not two (the option plus its value re-tested as an option).
        let src = "package require Tk 8.6\nentry .e -placeholder -placeholder\n";
        assert_eq!(count(src, "W136"), 1, "{:?}", version_diags(src));
    }

    #[test]
    fn value_word_of_ungated_option_draws_no_spurious_diagnostic() {
        // `-textvariable` consumes the next word `-placeholder` as its VALUE, so
        // `-placeholder` must not be tested as a (gated) option — no W136.
        let src = "package require Tk 8.6\nentry .e -textvariable -placeholder\n";
        assert_eq!(count(src, "W136"), 0, "{:?}", version_diags(src));
    }

    #[test]
    fn option_below_floor_fires_w136() {
        // `-placeholder` needs Tk 8.7; the require guarantees only 8.6.
        let src = "package require Tk 8.6\nentry .e -placeholder hi\n";
        assert!(fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn option_met_by_floor_is_silent() {
        let src = "package require Tk 8.7\nentry .e -placeholder hi\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    /// `(code, message)` version-gate pairs for an arbitrary dialect with
    /// optional keyed library-version pins (§7.1 axis C).
    fn version_diags_for(
        source: &str,
        dialect: &str,
        bigip_version: Option<&str>,
    ) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        a.library_versions.bigip_version = bigip_version.map(str::to_owned);
        a.analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    #[test]
    fn tracksbase_tk_pin_floors_an_unversioned_require() {
        // §7.1: `tcl8.6` ships Tk 8.6, so `package require Tk` *without a
        // version* still guarantees only 8.6 — the 8.7-introduced
        // `-placeholder` draws W136 (the old rule was silent here: an
        // unversioned require yielded no floor at all).
        let src = "package require Tk\nentry .e -placeholder hi\n";
        let diags = version_diags_for(src, "tcl8.6", None);
        assert!(
            diags
                .iter()
                .any(|(c, m)| c == "W136" && m.contains("tcl8.6 ships Tk 8.6")),
            "TracksBase floor names the runtime as the guarantor: {diags:?}"
        );
        // TN: Tk 9.0 (tracking the tcl9.0 base) carries the 8.7 additions.
        assert!(
            version_diags_for(src, "tcl9.0", None).is_empty(),
            "Tk 9.0 meets an 8.7 introduction"
        );
        // An explicit require can only RAISE the pin floor, never lower it:
        // requiring 8.7 on the 8.6 base is satisfied at 8.7.
        let raised = "package require Tk 8.7\nentry .e -placeholder hi\n";
        assert!(version_diags_for(raised, "tcl8.6", None).is_empty());
    }

    #[test]
    fn keyed_bigip_floor_gates_the_f5_surface() {
        // HTTP2::header was introduced in BIG-IP 16.1.0 (the backfilled
        // datum); the iRules profile keys its surface on BigipVersion.
        let src = "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n";
        // TN at the D5 oldest-supported default (16.1.0 meets 16.1.0)…
        assert!(
            version_diags_for(src, "f5-irules", None).is_empty(),
            "the default floor admits the 16.1.0 surface"
        );
        // …TP pinned below the introduction…
        let below = version_diags_for(src, "f5-irules", Some("15.1.0"));
        assert!(
            below.iter().any(|(c, m)| c == "W135"
                && m.contains("requires f5-irules-cmds 16.1.0")
                && m.contains("f5-irules ships f5-irules-cmds 15.1.0")),
            "a 15.1.0 pin exposes the 16.1.0 introduction: {below:?}"
        );
        // …TN pinned above.
        assert!(
            version_diags_for(src, "f5-irules", Some("17.1.0")).is_empty(),
            "a 17.1.0 pin satisfies a 16.1.0 introduction"
        );
    }

    #[test]
    fn bigip_21_1_subcommands_follow_the_configured_floor() {
        for source in [
            "SSL::c3d cert_lifespan 5\n",
            "SSL::c3d cert_start_date override\n",
            "persist mcp persistence_name\n",
        ] {
            let below = version_diags_for(source, "f5-irules", Some("21.0.0"));
            assert!(
                below.iter().any(|(code, message)| code == "W135"
                    && message.contains("requires f5-irules-cmds 21.1.0")),
                "21.0 must reject the 21.1 subcommand: {below:?}"
            );
            assert!(
                version_diags_for(source, "f5-irules", Some("21.1.0")).is_empty(),
                "21.1 must admit {source:?}"
            );
        }
    }

    #[test]
    fn persist_table_mcp_mode_follows_the_configured_floor() {
        for operation in ["add", "lookup", "delete"] {
            let source = format!("persist {operation} mcp key\n");
            let below = version_diags_for(&source, "f5-irules", Some("21.0.0"));
            assert!(
                below.iter().any(|(code, message)| code == "W135"
                    && message.contains("Argument value 'mcp'")
                    && message.contains("requires f5-irules-cmds 21.1.0")),
                "21.0 must reject persist {operation} mcp: {below:?}"
            );
            assert!(
                version_diags_for(&source, "f5-irules", Some("21.1.0")).is_empty(),
                "21.1 must admit persist {operation} mcp"
            );
        }
    }

    /// Version-gate + argument-DSL codes for `source` under `dialect`.
    fn dsl_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W137" | "W138" | "W200"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    #[test]
    fn w138_format_binary_conversion_is_86_gated() {
        // Oracle-verified surface: `%b` was added in Tcl 8.6.
        let src = "format %b 5\n";
        // TP: 8.4/8.5-era runtimes (incl. iRules' embedded 8.4.6).
        for d in ["tcl8.4", "tcl8.5", "f5-irules", "f5-iapps", "f5-tmsh"] {
            let diags = dsl_diags(src, d);
            assert!(
                diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
                "{d}: %b needs 8.6, got {diags:?}"
            );
        }
        // TN: 8.6+ runtimes.
        for d in ["tcl8.6", "tcl9.0", "expect", "bpf", "synopsys-eda-tcl"] {
            assert!(dsl_diags(src, d).is_empty(), "{d}: %b is real on 8.6+");
        }
        // FP-guard: `%%b` is a literal percent + `b`, not the conversion;
        // a dynamic format string abstains.
        assert!(dsl_diags("format %%b 5\n", "tcl8.4").is_empty());
        assert!(dsl_diags("format $fmt 5\n", "tcl8.4").is_empty());
        // The permissive fallback abstains entirely (§8).
        assert!(dsl_diags(src, "tcl").is_empty());
    }

    #[test]
    fn w138_format_unsigned_bignum_is_90_gated() {
        // Oracle-verified: tclsh8.6 raises "unsigned bignum format is
        // invalid" for %llu; tclsh9.0.4 renders it.
        let src = "format %llu 5\n";
        let diags = dsl_diags(src, "tcl8.6");
        assert!(
            diags.iter().any(|(c, m)| c == "W138" && m.contains("%llu")),
            "tcl8.6: %llu needs 9.0, got {diags:?}"
        );
        assert!(dsl_diags(src, "tcl9.0").is_empty(), "9.0 renders %llu");
        // Plain %lld is fine everywhere the ladder models.
        assert!(dsl_diags("format %lld 5\n", "tcl8.6").is_empty());
    }

    #[test]
    fn w138_scan_binary_conversion_is_86_gated() {
        let src = "scan 101 %b x\n";
        let diags = dsl_diags(src, "tcl8.5");
        assert!(
            diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
            "tcl8.5: scan %b needs 8.6, got {diags:?}"
        );
        assert!(dsl_diags(src, "tcl8.6").is_empty());
    }

    #[test]
    fn w137_string_is_class_follows_the_effective_version() {
        // `string is dict` — oracle-verified 9.0-only (tclsh8.6: bad
        // class; tclsh9.0: works).
        let src = "string is dict {a 1}\n";
        for d in ["tcl8.4", "tcl8.6", "f5-iapps", "f5-tmsh"] {
            let diags = dsl_diags(src, d);
            assert!(
                diags
                    .iter()
                    .any(|(c, m)| c == "W137" && m.contains("'dict'")),
                "{d}: string is dict needs 9.0, got {diags:?}"
            );
        }
        for d in ["tcl9.0", "tcl9.1", "bpf"] {
            assert!(dsl_diags(src, d).is_empty(), "{d}: dict class is real");
        }
        // entier is 8.6+; wideinteger is 8.5+.
        assert!(
            dsl_diags("string is entier 5\n", "tcl8.5")
                .iter()
                .any(|(c, _)| c == "W137"),
            "entier needs 8.6"
        );
        assert!(dsl_diags("string is entier 5\n", "tcl8.6").is_empty());
        assert!(
            dsl_diags("string is wideinteger 5\n", "tcl8.4")
                .iter()
                .any(|(c, _)| c == "W137"),
            "wideinteger needs 8.5"
        );
        assert!(dsl_diags("string is wideinteger 5\n", "tcl8.5").is_empty());
        // FP-guards: an always-available class, a dynamic class, and the
        // unique-prefix abbreviation of an ungated class stay silent.
        assert!(dsl_diags("string is alpha abc\n", "tcl8.4").is_empty());
        assert!(dsl_diags("string is $cls abc\n", "tcl8.4").is_empty());
        assert!(dsl_diags("string is xd abc\n", "tcl8.4").is_empty());
    }

    #[test]
    fn dsl_gates_honour_a_package_require_tcl_floor() {
        // §6.1: `package require Tcl 9.0` raises the effective version
        // above the ambient tcl8.6 dialect — the file validates as 9.0.
        let src = "package require Tcl 9.0\nformat %llu 5\nstring is dict {a 1}\n";
        assert!(
            dsl_diags(src, "tcl8.6").is_empty(),
            "a 9.0 core floor admits 9.0 DSL features"
        );
    }

    #[test]
    fn w200_binary_modifiers_follow_the_effective_version() {
        // TIP 275: binary format/scan u/s modifiers are 8.5+.
        let src = "binary format cu 5\n";
        for d in ["tcl8.4", "f5-irules"] {
            let diags = dsl_diags(src, d);
            assert!(
                diags.iter().any(|(c, _)| c == "W200"),
                "{d}: binary u modifier needs 8.5, got {diags:?}"
            );
        }
        // The old hardcoded list wrongly flagged f5-iapps — its host is a
        // real Tcl 8.5.13 where the modifiers work (FP fixed).
        for d in ["f5-iapps", "tcl8.5", "tcl8.6", "f5-tmsh"] {
            assert!(
                dsl_diags(src, d).is_empty(),
                "{d}: binary u modifier is real on 8.5+"
            );
        }
    }

    #[test]
    fn baseline_floor_declares_the_f5_surface_15_0_plus() {
        // M9: F5 specs with no explicit introduction inherit the declared
        // 15.0 baseline. TN at the 16.1 default and any 15.0+ pin…
        let src = "when HTTP_REQUEST {\n  pool p\n  HTTP::uri\n}\n";
        for pin in [None, Some("15.0.0"), Some("17.1.0")] {
            let mut a = Analyser::new();
            a.library_versions.bigip_version = pin.map(str::to_owned);
            let w135: Vec<String> = a
                .analyse(src, "f5-irules")
                .diagnostics
                .iter()
                .filter(|d| d.code.as_str() == "W135")
                .map(|d| d.message.clone())
                .collect();
            assert!(
                w135.is_empty(),
                "pin {pin:?}: baseline is met, got {w135:?}"
            );
        }
        // …TP below the baseline: the whole modelled surface is declared
        // 15.0+, so a 14.x target flags it.
        let mut a = Analyser::new();
        a.library_versions.bigip_version = Some("14.1.0".to_owned());
        let diags = a.analyse(src, "f5-irules").diagnostics;
        assert!(
            diags
                .iter()
                .any(|d| d.code.as_str() == "W135" && d.message.contains("15.0.0")),
            "a pre-baseline pin flags the declared floor: {:?}",
            diags
                .iter()
                .filter(|d| d.code.as_str() == "W135")
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
        // Explicit data still wins over the baseline: HTTP2::header is
        // 16.1.0-introduced, so a 15.1 pin flags it while `pool` is fine.
        let mut a = Analyser::new();
        a.library_versions.bigip_version = Some("15.1.0".to_owned());
        let diags = a
            .analyse(
                "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
                "f5-irules",
            )
            .diagnostics;
        assert!(
            diags
                .iter()
                .any(|d| d.code.as_str() == "W135" && d.message.contains("16.1.0")),
            "explicit introduction data outranks the baseline"
        );
    }

    #[test]
    fn event_version_clause_follows_the_declared_range() {
        // Known event at the default target: clean (baseline 15.0 ≤ 16.1).
        let mut a = Analyser::new();
        let diags = a
            .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
            .diagnostics;
        assert!(
            !diags
                .iter()
                .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("target release")),
            "events at the default target stay clean"
        );
        // Pinned below the baseline: the event's declared range excludes it.
        let mut a = Analyser::new();
        a.library_versions.bigip_version = Some("14.1.0".to_owned());
        let diags = a
            .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
            .diagnostics;
        assert!(
            diags
                .iter()
                .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("15.0.0")),
            "a pre-baseline target flags the event's declared range: {:?}",
            diags.iter().map(|d| d.code.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ambient_f5_surface_never_draws_missing_require() {
        // HTTP2::header carries `required_package: f5-irules-cmds`, but the
        // profile ships that surface ambiently (§7.1) — no W120, and the
        // command stays resolved (no W123/W002 either).
        let mut a = Analyser::new();
        let result = a.analyse(
            "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
            "f5-irules",
        );
        let noisy: Vec<&str> = result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .filter(|c| matches!(*c, "W120" | "W123" | "W002"))
            .collect();
        assert!(
            noisy.is_empty(),
            "ambient vendor surface must not draw require/unknown codes: {noisy:?}"
        );
    }

    #[test]
    fn ungated_option_is_silent() {
        // `-width` carries no `min_version`, so no version diagnostic.
        let src = "package require Tk 8.6\nentry .e -width 10\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn require_without_version_is_permissive_for_unpinned_packages() {
        // No version floor to compare against ⇒ nothing flagged. This is
        // the contract for a package the profile does NOT pin — under the
        // permissive fallback profile (`"tcl"`, no library pins) an
        // unversioned require yields no floor. On a pinned host the
        // shipped version floors it instead (§7.1 — see
        // `tracksbase_tk_pin_floors_an_unversioned_require`).
        let src = "package require Tk\nentry .e -placeholder hi\n";
        assert!(
            version_diags_for(src, "tcl", None).is_empty(),
            "{:?}",
            version_diags_for(src, "tcl", None)
        );
    }

    #[test]
    fn no_require_draws_no_version_diagnostic() {
        // Missing `package require` is W120's job, not a version diagnostic.
        let src = "entry .e -placeholder hi\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn command_below_floor_fires_w135() {
        // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
        // 8.4 (TracksBase, §7.1) and a `require Tk 8.4` cannot raise it —
        // W135, named after the runtime guarantor.
        let src = "package require Tk 8.4\nttk::button .b\n";
        let diags = version_diags_for(src, "tcl8.4", None);
        assert!(
            diags
                .iter()
                .any(|(c, m)| c == "W135" && m.contains("tcl8.4 ships Tk 8.4")),
            "{diags:?}"
        );
        // On a tcl8.6 host the same source is FINE: `package require Tk
        // 8.4` states a minimum, it does not downgrade the shipped Tk 8.6 —
        // the old require-only floor drew a false positive here.
        assert!(
            version_diags_for(src, "tcl8.6", None).is_empty(),
            "the shipped Tk 8.6 satisfies an 8.5 introduction"
        );
    }

    #[test]
    fn command_met_by_floor_is_silent() {
        let src = "package require Tk 8.6\nttk::button .b\n";
        assert!(!fires(src, "W135"), "{:?}", version_diags(src));
    }

    #[test]
    fn conditional_probe_does_not_raise_floor() {
        // An optional `catch {package require Tk 8.7}` does not guarantee 8.7;
        // the guaranteed floor is the unconditional 8.6, so W136 still fires.
        let src = "package require Tk 8.6\n\
                   catch {package require Tk 8.7}\n\
                   entry .e -placeholder hi\n";
        assert!(fires(src, "W136"), "{:?}", version_diags(src));
    }

    /// Every version-gate code for `source` under `dialect`, including the
    /// retirement (W139) and deprecation (W144) rungs the narrower helpers
    /// above filter out.
    fn lifecycle_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139" | "W144"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    #[test]
    fn command_level_argument_value_follows_the_core_floor() {
        // `close chan ?read|write?` — the half-close direction word is TIP
        // 332, Tcl 8.6.  It sits at a command-level `arg_values` position
        // that is deliberately *not* a closed value set, so W137 never sees
        // it; the command-level version gate is what reports it.
        let src = "close $s read\n";
        for dialect in ["tcl8.4", "tcl8.5"] {
            let diags = lifecycle_diags(src, dialect);
            assert!(
                diags.iter().any(|(code, message)| code == "W135"
                    && message.contains("Argument value 'read' on 'close'")
                    && message.contains("requires Tcl 8.6")),
                "{dialect}: half-close needs 8.6, got {diags:?}"
            );
        }
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                lifecycle_diags(src, dialect).is_empty(),
                "{dialect}: half-close is real from 8.6"
            );
        }
        // FP-guards: the plain one-argument form has no direction word at
        // all, and a dynamic direction is not a literal value.
        assert!(lifecycle_diags("close $s\n", "tcl8.5").is_empty());
        assert!(lifecycle_diags("close $s $dir\n", "tcl8.5").is_empty());
    }

    #[test]
    fn sub_subcommand_follows_the_core_floor() {
        // `info object properties` / `info class definitionnamespace` are the
        // 9.0 TclOO introspection additions (TIP 558 / TIP 524).
        for src in [
            "info object properties $o\n",
            "info class definitionnamespace $c\n",
        ] {
            let diags = lifecycle_diags(src, "tcl8.6");
            assert!(
                diags
                    .iter()
                    .any(|(code, message)| code == "W135" && message.contains("requires Tcl 9.0")),
                "8.6 must reject {src:?}: {diags:?}"
            );
            assert!(
                lifecycle_diags(src, "tcl9.0").is_empty(),
                "9.0 must admit {src:?}"
            );
        }
        // The 8.6 operations of the same ensembles stay silent, and a unique
        // prefix of a gated operation is gated exactly as the full word is.
        assert!(lifecycle_diags("info object class $o\n", "tcl8.6").is_empty());
        assert!(
            lifecycle_diags("info class definitionn $c\n", "tcl8.6")
                .iter()
                .any(|(code, _)| code == "W135"),
            "an abbreviation resolves to the same gated operation"
        );
    }

    #[test]
    fn deprecated_subcommand_reports_once_on_the_axis_that_still_has_it() {
        // `trace variable` is deprecated from 8.4 and removed in 9.0.
        let src = "trace variable v w handler\n";
        let diags = lifecycle_diags(src, "tcl8.6");
        assert!(
            diags.iter().any(|(code, message)| code == "W144"
                && message.contains("Subcommand 'variable' on 'trace'")
                && message.contains("deprecated as of Tcl 8.4")),
            "8.6 still has the legacy form, deprecated: {diags:?}"
        );
        // On 9.0 the form is gone from the profile entirely — that is W002's
        // word, and the version gate must not flag it a second time.
        assert!(
            lifecycle_diags(src, "tcl9.0").is_empty(),
            "a dialect-absent subcommand stays W002's alone"
        );
    }

    #[test]
    fn a_require_range_that_reaches_a_retirement_is_hedged() {
        // The floor (8.6) still has `trace variable`, but the range admits
        // every version up to 9.1 — including the 9.0 that removed it.
        let src = "package require Tcl 8.5-9.1\ntrace variable v w handler\n";
        let diags = lifecycle_diags(src, "tcl8.6");
        assert!(
            diags.iter().any(|(code, message)| code == "W139"
                && message
                    .contains("not available in every version satisfying requirement `8.5-9.1`")
                && message.contains("removed in Tcl 9.0")),
            "a straddling range is hedged, not asserted: {diags:?}"
        );
        // A range that stops below the retirement keeps the ordinary
        // floor verdict …
        let inside = "package require Tcl 8.5-8.7\ntrace variable v w handler\n";
        assert!(
            lifecycle_diags(inside, "tcl8.6")
                .iter()
                .all(|(code, _)| code == "W144"),
            "a range inside the window reports only the deprecation"
        );
        // … and so does an open-ended or bare requirement, which states no
        // ceiling at all.
        for req in ["8.5-", "8.5"] {
            let src = format!("package require Tcl {req}\ntrace variable v w handler\n");
            assert!(
                lifecycle_diags(&src, "tcl8.6")
                    .iter()
                    .all(|(code, _)| code == "W144"),
                "`{req}` states no ceiling"
            );
        }
        // A conditional probe guarantees nothing, so it cannot widen the
        // window either.
        let probed = "package require Tcl 8.5\n\
                      catch {package require Tcl 8.5-9.1}\n\
                      trace variable v w handler\n";
        assert!(
            lifecycle_diags(probed, "tcl8.6")
                .iter()
                .all(|(code, _)| code == "W144"),
            "a conditional require is not a guarantee"
        );
    }

    /// A synthetic package whose two options conflict **only from 2.0**.
    ///
    /// Nothing shipped declares a lifecycle on an `option_conflict` yet — a
    /// `.tclspec` pack is the first thing that will (`option_conflict {-a -b}
    /// -introduced 2.0` is loader vocabulary 1.1) — so the gate is exercised
    /// through an overlay registry, which is exactly the route a workspace's
    /// packs reach the analyser by.
    mod gated_option_conflict {
        use super::super::super::super::state::Analyser;

        /// Overlay key for the synthetic pack below. Any non-zero value works;
        /// the cache keys `(profile, overlay)` on it.
        const OVERLAY: u64 = 0x7e57_c0f1;

        const OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
            tcl_registry::hover::OptionSpec {
                name: "-alpha",
                ..tcl_registry::hover::OptionSpec::DEFAULT
            },
            tcl_registry::hover::OptionSpec {
                name: "-beta",
                ..tcl_registry::hover::OptionSpec::DEFAULT
            },
        ];

        const CONSTRAINTS: &[tcl_registry::OptionConstraint] = &[tcl_registry::OptionConstraint {
            options: &["-alpha", "-beta"],
            lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("2.0"),
            ..tcl_registry::OptionConstraint::DEFAULT
        }];

        /// The same relationship with no lifecycle at all — the zero-change
        /// control for the inline path.
        const UNGATED_CONSTRAINTS: &[tcl_registry::OptionConstraint] =
            &[tcl_registry::OptionConstraint {
                options: &["-alpha", "-beta"],
                ..tcl_registry::OptionConstraint::DEFAULT
            }];

        fn spec(
            name: &'static str,
            constraints: &'static [tcl_registry::OptionConstraint],
        ) -> tcl_registry::CommandSpec {
            tcl_registry::CommandSpec {
                name,
                required_package: Some("Fauxpkg"),
                arity: tcl_registry::Arity::any(),
                options: OPTIONS,
                option_constraints: constraints,
                ..tcl_registry::CommandSpec::DEFAULT
            }
        }

        /// Every W147 message `source` draws under `tcl8.6`, with the
        /// synthetic pack installed.
        fn conflicts(source: &str) -> Vec<String> {
            let profile = tcl_dialect::DialectProfile::by_name("tcl8.6");
            let _registry =
                tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |registry| {
                    registry.insert(spec("fauxgated", CONSTRAINTS));
                    registry.insert(spec("fauxplain", UNGATED_CONSTRAINTS));
                });
            Analyser::new()
                .with_pack_overlay(OVERLAY)
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .filter(|d| d.code.as_str() == "W147")
                .map(|d| d.message.clone())
                .collect()
        }

        #[test]
        fn a_conflict_introduced_later_is_silent_below_its_floor() {
            // The relationship does not exist in Fauxpkg 1.x, so enforcing it
            // would report a rule that release has not got.
            let src = "package require Fauxpkg 1.0\nfauxgated -alpha -beta\n";
            assert!(conflicts(src).is_empty(), "{:?}", conflicts(src));
        }

        #[test]
        fn a_conflict_introduced_later_fires_once_the_floor_reaches_it() {
            let src = "package require Fauxpkg 2.0\nfauxgated -alpha -beta\n";
            let diags = conflicts(src);
            assert!(
                diags
                    .iter()
                    .any(|m| m.contains("-alpha, -beta") && m.contains("fauxgated")),
                "{diags:?}"
            );
        }

        #[test]
        fn no_resolvable_floor_stays_permissive() {
            // An unversioned require on an unpinned package resolves no floor
            // at all, and "no version known ⇒ do not gate" is the standing
            // registry rule — so the conflict is reported.
            let src = "package require Fauxpkg\nfauxgated -alpha -beta\n";
            assert_eq!(conflicts(src).len(), 1, "{:?}", conflicts(src));
        }

        #[test]
        fn an_unversioned_constraint_is_untouched_by_the_gate() {
            // The zero-behaviour-change control: a constraint with no
            // lifecycle keeps the inline path, floor or no floor.
            for src in [
                "package require Fauxpkg 1.0\nfauxplain -alpha -beta\n",
                "package require Fauxpkg 2.0\nfauxplain -alpha -beta\n",
                "package require Fauxpkg\nfauxplain -alpha -beta\n",
            ] {
                assert_eq!(conflicts(src).len(), 1, "{src:?}: {:?}", conflicts(src));
            }
        }

        #[test]
        fn a_retired_conflict_stops_being_enforced_but_a_deprecated_one_does_not() {
            // Retirement is exclusive: the relationship is gone at 3.0…
            const RETIRED: &[tcl_registry::OptionConstraint] = &[tcl_registry::OptionConstraint {
                options: &["-alpha", "-beta"],
                lifecycle: tcl_registry::lifecycle::Lifecycle::UNSPECIFIED
                    .retired_from("3.0")
                    .deprecated_from("2.0"),
                ..tcl_registry::OptionConstraint::DEFAULT
            }];
            const KEY: u64 = 0x7e57_c0f2;
            let profile = tcl_dialect::DialectProfile::by_name("tcl8.6");
            let _registry =
                tcl_registry::registry_for_profile_with_overlay(profile, KEY, |registry| {
                    registry.insert(spec("fauxretired", RETIRED));
                });
            let count = |source: &str| {
                Analyser::new()
                    .with_pack_overlay(KEY)
                    .analyse(source, "tcl8.6")
                    .diagnostics
                    .iter()
                    .filter(|d| d.code.as_str() == "W147")
                    .count()
            };
            assert_eq!(
                count("package require Fauxpkg 3.0\nfauxretired -alpha -beta\n"),
                0,
                "a retired relationship is not enforced"
            );
            // … but a *deprecated* one still exists, and deprecation is not
            // absence.
            assert_eq!(
                count("package require Fauxpkg 2.0\nfauxretired -alpha -beta\n"),
                1,
                "a deprecated relationship still holds"
            );
        }

        #[test]
        fn a_shadowing_user_proc_still_suppresses_a_gated_conflict() {
            // The gate decides whether the relationship exists; it must not
            // bypass the `pending_arity` shadowing suppression an ungated
            // conflict goes through.
            let src = "package require Fauxpkg 2.0\n\
                       proc fauxgated args {}\n\
                       fauxgated -alpha -beta\n";
            assert!(conflicts(src).is_empty(), "{:?}", conflicts(src));
        }

        #[test]
        fn a_require_inside_the_file_gates_a_conflict_in_a_proc_body() {
            // The floor is a whole-file fact and the `package require` may
            // follow the call: buffering is what makes this work.
            let below = "proc use {} { fauxgated -alpha -beta }\n\
                         package require Fauxpkg 1.0\n";
            assert!(conflicts(below).is_empty(), "{:?}", conflicts(below));
            let met = "proc use {} { fauxgated -alpha -beta }\n\
                       package require Fauxpkg 2.0\n";
            assert_eq!(conflicts(met).len(), 1, "{:?}", conflicts(met));
        }
    }
}
