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
//! A command, subcommand, option, or literal argument value carries a
//! [`Lifecycle`] on either its owning package's version axis (Tk, a tcllib
//! package, `argparse`, …) or the core Tcl axis: the introducing release, the
//! deprecating release, and the retiring release. The resolved floor — the
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

/// Version axes supported by the generic lifecycle consumer.
#[derive(Debug, Clone, Copy)]
enum VersionGateAxis {
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
    Option {
        command: String,
        option: String,
    },
    ArgumentValue {
        command: String,
        subcommand: String,
        value: String,
    },
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
        VersionGateItem::ArgumentValue {
            command,
            subcommand,
            value,
        } => format!("Argument value '{value}' on '{command} {subcommand}'"),
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
        sub: &tcl_registry::SubCommand,
    ) {
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
        for gate in sub.versioned_arg_values {
            let arg_idx = 1usize + usize::from(gate.index);
            let (Some(arg), Some(tok)) = (
                invocation.args.get(arg_idx),
                invocation.arg_tokens.get(arg_idx),
            ) else {
                continue;
            };
            if arg != gate.value || matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
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
                gate.lifecycle,
                VersionGateItem::ArgumentValue {
                    command: invocation.command.to_owned(),
                    subcommand: sub.name.to_owned(),
                    value: gate.value.to_owned(),
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
        let axis = self
            .profile
            .keyed_pin_for(spec)
            .map(|pin| pin.package)
            .or_else(|| spec.owning_package())
            .map(VersionGateAxis::Package)
            // A lifecycle on an otherwise unowned Tcl command/subcommand is
            // governed by the selected core Tcl profile, not by a fictitious
            // `package require`. This keeps every core command generic while
            // allowing an explicit `package require Tcl` to raise the floor.
            .or_else(|| {
                spec.supports_dialect(DialectSet::ALL_TCL)
                    .then_some(VersionGateAxis::TclCore)
            });
        let Some(axis) = axis else {
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

        // Option-level gates.  Resolve subcommand-scoped options when the first
        // argument names a subcommand.
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| {
                let first = args.first().map(String::as_str).unwrap_or_default();
                spec.resolve_subcommand(first)
            })
            .flatten();

        if let Some(sub) = sub_match {
            self.record_subcommand_version_sites(invocation, sub);
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
            let floor_and_guarantee = match site.axis {
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
            };
            let Some((floor, guarantee)) = floor_and_guarantee else {
                continue;
            };
            let Some((code, message)) = version_gate_diagnostic(&site, &floor, &guarantee) else {
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
}
