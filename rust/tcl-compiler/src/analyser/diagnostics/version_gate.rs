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
use tcl_dialect::model::{Family, SpecProvider};
use tcl_lexer::Span;
use tcl_registry::deprecation::{DeprecationFixContext, DeprecationFixTarget, DeprecationFixWord};
use tcl_registry::lifecycle::{Lifecycle, LifecycleState};

use super::super::state::Analyser;
use super::super::types::{Diagnostic, Severity};
use tcl_dialect::model::SpecSurface;

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
    dialect: String,
}

/// A proven **W147** option conflict whose [`OptionRelation`] is
/// version-gated, held until the whole-file floor is known.
///
/// A relationship such as `option_conflict {-a -b} -introduced 2.0` does not
/// exist below 2.0, so enforcing it against a file pinned to 1.x would report
/// a rule that release does not have. A constraint with an
/// [`Lifecycle::UNSPECIFIED`] lifecycle never reaches this buffer: it holds in
/// every release and is queued inline onto [`Analyser::pending_arity`] at the
/// dispatch site.
///
/// The fields after `lifecycle` are the `pending_arity` tuple this becomes
/// once [`Analyser::flush_gated_option_conflicts`] decides the relationship
/// exists — the version gate is a filter in front of the ordinary queue, not
/// a second reporting path.
///
/// [`OptionRelation`]: tcl_registry::OptionRelation
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

/// Retained selected source signature whose window depends on a later floor.
/// Each window recounts the same effective operands under its own count axis.
#[derive(Debug, Clone)]
pub(in crate::analyser) struct GatedArityCall {
    pub(in crate::analyser) axis: Option<VersionGateAxis>,
    pub(in crate::analyser) windows: &'static [tcl_registry::arity::ArityWindow],
    pub(in crate::analyser) fallback: tcl_registry::Arity,
    pub(in crate::analyser) display_name: String,
    pub(in crate::analyser) original:
        super::super::diagnostic_registry::OriginalDiagnosticInvocation,
    pub(in crate::analyser) synopsis: Option<&'static str>,
}

/// A bare ensemble with original head geometry and selected parent windows.
#[derive(Debug)]
pub(in crate::analyser) struct GatedBareEnsemble {
    pub(in crate::analyser) axis: Option<VersionGateAxis>,
    pub(in crate::analyser) windows: &'static [tcl_registry::arity::ArityWindow],
    pub(in crate::analyser) fallback: tcl_registry::Arity,
    pub(in crate::analyser) display_name: String,
    pub(in crate::analyser) original:
        super::super::diagnostic_registry::OriginalDiagnosticInvocation,
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

    /// The model axis this gate axis names — the §5.4 declared-target
    /// sets are keyed by [`tcl_dialect::model::VersionAxisId`].
    fn version_axis(self) -> tcl_dialect::model::VersionAxisId {
        match self {
            Self::Package(package) => tcl_dialect::model::VersionAxisId::package(package),
            Self::TclCore => {
                tcl_dialect::model::VersionAxisId::core(tcl_dialect::model::Family::Tcl)
            }
        }
    }
}

/// A §5.4 **mask-gated** range site (W150): an item whose availability is
/// spelled as `SpecSurface` ladder bits rather than a lifecycle, admitted
/// at the primary target but missing from part of the declared target
/// set. Buffered during the walk and decided post-walk beside the
/// lifecycle sites so one word still draws one diagnostic.
#[derive(Debug)]
pub(in crate::analyser) struct RangeGateSite {
    /// Span the diagnostic anchors to.
    span: Span,
    /// Human phrase naming the gated syntax (`item_phrase` form).
    what: String,
    /// The gate's own version floor ("8.6"), when it spells one — the
    /// "requires Tcl 8.6" half of the message.
    requires: Option<String>,
    /// The declared targets the gate does not cover.
    uncovered: tcl_dialect::model::VersionSet,
    /// Typed source ownership, when this is an original Registry use.
    subject: Option<crate::analyser::DiagnosticSubject>,
}

/// The names a range diagnostic lists for `set`: the touched ladder
/// releases where the axis has a version-spelled ladder, the
/// requirement spelling otherwise.
fn range_target_names(set: &tcl_dialect::model::VersionSet) -> String {
    let names = tcl_registry::model::ladder_releases_in(set);
    if names.is_empty() {
        tcl_registry::model::requirement_spelling(set)
    } else {
        names.join(", ")
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
    subject: crate::analyser::DiagnosticSubject,
}

/// One retained source invocation while its lifecycle uses are recorded.
#[derive(Clone, Copy)]
struct LifecycleInvocation<'a> {
    original: &'a super::super::diagnostic_registry::OriginalDiagnosticInvocation,
    command: &'a str,
    axis: VersionGateAxis,
}

fn deprecation_fix_payload(
    invocation: LifecycleInvocation<'_>,
    argument: Option<usize>,
) -> Option<DeprecationFixPayload> {
    let original = invocation.original;
    let head = original.head();
    let mut word_spans = vec![Some(head.span())];
    let mut words = vec![StoredDeprecationFixWord {
        spelling: original.command().to_owned(),
        literal: true,
    }];
    for (index, value) in original.words().arguments().iter().enumerate() {
        word_spans.push(original.word(index).map(tcl_lexer::NativeWord::span));
        let literal = value
            .literal_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        words.push(StoredDeprecationFixWord {
            spelling: literal.unwrap_or_default().to_owned(),
            literal: literal.is_some(),
        });
    }
    let end = word_spans
        .iter()
        .flatten()
        .map(|span| span.end())
        .max()
        .unwrap_or(head.span().end());
    Some(DeprecationFixPayload {
        invocation: Span::new(head.span().start(), end),
        word_spans,
        words,
        matched_word_index: argument.map_or(0, |index| index + 1),
        subject: original.subject(
            crate::analyser::RegistrySourceDiagnosticKind::Lifecycle,
            argument,
        )?,
    })
}

/// Payload distinguishing the package-version gate's syntax granularity.
#[derive(Debug, Clone)]
pub(in crate::analyser) enum VersionGateItem {
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
        self.analysis_context()
            .context()
            .keyed_ambient_placement(spec)
            .map(|placement| crate::environment_ingress::interned_package_name(&placement.package))
            .or_else(|| spec.owning_package())
            .map(VersionGateAxis::Package)
            .or_else(|| {
                spec.surface
                    .is_none_or(|rows| {
                        rows.iter()
                            .any(|row| row.provider == SpecProvider::Core(Family::Tcl))
                    })
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
            dialect: self.environment_label(),
        });
    }

    fn record_source_lifecycle_site(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        argument: Option<usize>,
        lifecycle: Lifecycle,
        item: VersionGateItem,
        surface: Option<&'static [SpecSurface]>,
    ) {
        let Some(payload) = deprecation_fix_payload(invocation, argument) else {
            return;
        };
        let span = match argument {
            Some(index) => invocation
                .original
                .word(index)
                .expect("payload retains original operand")
                .span(),
            None => invocation.original.head().span(),
        };
        self.record_range_gate_site_with_subject(
            span,
            &item,
            surface,
            Some(payload.subject.clone()),
        );
        self.record_lifecycle_site(span, invocation.axis, lifecycle, item, payload);
    }

    fn record_subcommand_version_sites(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        spec: &tcl_registry::CommandSpec,
        sub: &tcl_registry::SubCommand,
    ) {
        if !invocation
            .original
            .context()
            .context()
            .subcommand_available(spec, sub)
        {
            return;
        }
        self.record_source_lifecycle_site(
            invocation,
            Some(0),
            sub.lifecycle,
            VersionGateItem::Subcommand {
                command: invocation.command.to_owned(),
                subcommand: sub.name.to_owned(),
            },
            sub.surface.or(spec.surface),
        );
        self.record_sub_subcommand_version_site(invocation, sub);
        self.record_arg_value_version_sites(
            invocation,
            &arg_value_gates(sub.arg_values, sub.versioned_arg_values),
            1,
            Some(sub.name),
        );
    }

    fn record_sub_subcommand_version_site(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        sub: &tcl_registry::SubCommand,
    ) {
        let Some(word) = invocation.original.literal(1) else {
            return;
        };
        let Some(nested) = sub.resolve_sub_subcommand_gated(
            word,
            Some(invocation.original.context().context().authoring_query()),
            None,
        ) else {
            return;
        };
        self.record_source_lifecycle_site(
            invocation,
            Some(1),
            nested.lifecycle,
            VersionGateItem::SubSubCommand {
                command: invocation.command.to_owned(),
                subcommand: sub.name.to_owned(),
                sub_subcommand: nested.name.to_owned(),
            },
            nested.surface.or(sub.surface),
        );
    }

    fn record_arg_value_version_sites(
        &mut self,
        invocation: LifecycleInvocation<'_>,
        gates: &[(u8, &'static str, Lifecycle)],
        argument_offset: usize,
        subcommand: Option<&str>,
    ) {
        for &(index, value, lifecycle) in gates {
            let argument = argument_offset + usize::from(index);
            if invocation.original.literal(argument) != Some(value) {
                continue;
            }
            self.record_source_lifecycle_site(
                invocation,
                Some(argument),
                lifecycle,
                VersionGateItem::ArgumentValue {
                    command: invocation.command.to_owned(),
                    subcommand: subcommand.map(ToOwned::to_owned),
                    value: value.to_owned(),
                },
                None,
            );
        }
    }

    fn record_option_version_sites(&mut self, invocation: LifecycleInvocation<'_>) {
        let Some(scan) = invocation
            .original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        for selected in scan.options {
            let Some(spelling) = invocation.original.literal(selected.argument) else {
                continue;
            };
            self.record_source_lifecycle_site(
                invocation,
                Some(selected.argument),
                selected.option.lifecycle,
                VersionGateItem::Option {
                    command: invocation.command.to_owned(),
                    option: spelling.to_owned(),
                },
                selected.surface,
            );
        }
    }

    /// Buffer command, selector, option and literal-value lifecycle syntax
    /// from one retained source schema. Effective captured ordinals can select
    /// a table, but only genuine written operands can anchor a diagnostic.
    pub(in crate::analyser) fn record_version_gate_sites(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
        command: &str,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(selected) =
            original.with_schema(crate::analyser::diagnostic_registry::source_descriptors)
        else {
            return;
        };
        let spec = selected.command;
        let Some(axis) = self.lifecycle_axis(spec) else {
            return;
        };
        let invocation = LifecycleInvocation {
            original,
            command,
            axis,
        };
        let effective = original
            .context()
            .context()
            .keyed_version_range(spec)
            .map_or(spec.lifecycle, |(min, max)| Lifecycle {
                introduced: min.or(spec.lifecycle.introduced),
                deprecated: spec.lifecycle.deprecated,
                retired: max.or(spec.lifecycle.retired),
                deprecation_fix: spec.lifecycle.deprecation_fix,
            });
        self.record_source_lifecycle_site(
            invocation,
            None,
            effective,
            VersionGateItem::Command(command.to_owned()),
            spec.surface,
        );
        self.record_arg_value_version_sites(
            invocation,
            &arg_value_gates(spec.arg_values, spec.versioned_arg_values),
            0,
            None,
        );
        if let Some(sub) = selected.subcommand {
            self.record_subcommand_version_sites(invocation, spec, sub);
        }
        self.record_option_version_sites(invocation);
    }

    /// Emit W135/W136 for each buffered site whose package's resolved
    /// version floor is below the required `min_version`. The floor comes
    /// from an explicit versioned `package require`, or — for a package the
    /// active profile pins (§7.1 axis C) — from the profile pin (the shipped
    /// Tk on a plain Tcl base, a `Keyed` vendor surface at its
    /// oldest-supported default). Sites with no floor at all (unpinned +
    /// required without a version, or not required — the latter handled by
    /// W120) are skipped.
    pub(in crate::analyser) fn flush_version_gate_diagnostics(&mut self) {
        if self.version_gate_sites.is_empty() && self.range_gate_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.version_gate_sites);
        let mut new_diags: Vec<Diagnostic> = Vec::new();
        // Spans that drew a verdict, so the §5.4 range pass keeps the
        // one-word-one-diagnostic rule: a primary (semantic-floor)
        // verdict always outranks the assistance range warning.
        let mut reported: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for site in sites {
            let floor = self.axis_floor(site.axis);
            // A range that reaches a retirement outranks the floor's own
            // verdict *while the floor is satisfied*: "gone in part of the
            // accepted range" is the stronger fact than "deprecated here".
            // A floor that already fails keeps its own, more specific message.
            let verdict = floor.as_ref().and_then(|(floor, guarantee)| {
                self.requirement_straddle_diagnostic(&site, floor)
                    .or_else(|| version_gate_diagnostic(&site, floor, guarantee))
            });
            if let Some((code, message)) = verdict {
                let (floor, _) = floor.expect("a verdict implies a resolved floor");
                let fixes = (code == DiagCode::W144)
                    .then(|| lifecycle_deprecation_fix(&site, &floor))
                    .flatten()
                    .into_iter()
                    .collect();
                reported.insert((site.span.start(), site.span.end()));
                new_diags.push(
                    crate::analyser::types::Diagnostic::new(
                        code,
                        site.span,
                        message,
                        Severity::Warning,
                    )
                    .with_fixes(fixes)
                    .with_subject(site.fix_payload.subject.clone()),
                );
                continue;
            }
            // §5.4 range mode: the primary floor is satisfied (or none
            // resolves), but a *declared* target can still sit outside
            // the item's lifecycle window — the assistance range warning.
            if let Some(message) = self.range_window_message(&site) {
                reported.insert((site.span.start(), site.span.end()));
                new_diags.push(
                    crate::analyser::types::Diagnostic::new(
                        DiagCode::W150,
                        site.span,
                        message,
                        Severity::Warning,
                    )
                    .with_subject(site.fix_payload.subject.clone()),
                );
            }
        }
        self.result.diagnostics.extend(new_diags);
        self.flush_range_gate_sites(&reported);
    }

    /// The **W150** message for a lifecycle site whose window excludes
    /// part of the declared target set (§5.4), or `None` when no targets
    /// are declared on the site's axis or the window covers them all.
    /// The introduction end outranks the retirement end, mirroring the
    /// floor gate's own precedence; deprecation is not absence and never
    /// range-warns.
    fn range_window_message(&self, site: &VersionGateSite) -> Option<String> {
        let context = self.range_context.as_ref()?;
        let axis = site.axis.version_axis();
        context.declared_targets(&axis)?;
        let what = item_phrase(&site.item);
        let package = site.axis.name();
        if let Some(introduced) = site.lifecycle.introduced
            && let Some(outside) = context.targets_outside_window(&axis, Some(introduced), None)
        {
            return Some(format!(
                "{what} requires {package} {introduced} but the declared targets include {}.",
                range_target_names(&outside)
            ));
        }
        let retired = site.lifecycle.retired?;
        let outside = context.targets_outside_window(&axis, None, Some(retired))?;
        Some(format!(
            "{what} was removed in {package} {retired} but the declared targets include {}.",
            range_target_names(&outside)
        ))
    }

    fn record_range_gate_site_with_subject(
        &mut self,
        span: Span,
        item: &VersionGateItem,
        gate: Option<&'static [SpecSurface]>,
        subject: Option<crate::analyser::DiagnosticSubject>,
    ) {
        let Some(context) = self.range_context.as_ref() else {
            return;
        };
        let Some(uncovered) = context.targets_uncovered_by_gate(gate) else {
            return;
        };
        let requires = gate
            .and_then(tcl_registry::model::core_tcl_floor)
            .map(|version| version.as_package_version().to_owned());
        self.range_gate_sites.push(RangeGateSite {
            span,
            what: item_phrase(item),
            requires,
            uncovered,
            subject,
        });
    }

    /// Emit **W150** for the buffered mask-gated range sites, skipping
    /// any span the lifecycle pass already reported (one word, one
    /// diagnostic).
    fn flush_range_gate_sites(&mut self, reported: &std::collections::HashSet<(u32, u32)>) {
        if self.range_gate_sites.is_empty() {
            return;
        }
        let mut seen = reported.clone();
        for site in std::mem::take(&mut self.range_gate_sites) {
            if !seen.insert((site.span.start(), site.span.end())) {
                continue;
            }
            let names = range_target_names(&site.uncovered);
            // "requires X" is only the right story when the miss is
            // *below* the gate's floor; a run that ends before a target
            // (the removed-in-9.0 shape) gets the neutral phrasing.
            let miss_is_below_floor = site.requires.as_ref().is_some_and(|requires| {
                tcl_dialect::model::VersionSet::from_requirements(
                    site.uncovered.axis().clone(),
                    &[format!("0-{requires}")],
                )
                .is_ok_and(|below| site.uncovered.subset(&below).unwrap_or(false))
            });
            let message = match &site.requires {
                Some(requires) if miss_is_below_floor => format!(
                    "{} requires Tcl {requires} but the declared targets include {names}.",
                    site.what
                ),
                _ => format!(
                    "{} is not available at every declared Tcl target — missing at {names}.",
                    site.what
                ),
            };
            let mut diagnostic = crate::analyser::types::Diagnostic::new(
                DiagCode::W150,
                site.span,
                message,
                Severity::Warning,
            );
            if let Some(subject) = site.subject {
                diagnostic = diagnostic.with_subject(subject);
            }
            self.result.diagnostics.push(diagnostic);
        }
    }

    /// The name a guarantor phrase calls this document's environment.
    ///
    /// The resolved environment's own canonical id, not the interned
    /// [`tcl_dialect::DialectProfile`]'s name. Every compiled environment
    /// shares its name with its profile, so no shipped message moves; a
    /// **pack-declared** environment has no compiled profile and sinks to
    /// the permissive fallback, which would otherwise make its placements
    /// report as `tcl ships Tk 8.6` — naming a profile the author never chose
    /// instead of the shell they declared.
    fn environment_label(&self) -> String {
        let id = self
            .analysis_context()
            .context()
            .environment
            .id
            .as_str()
            .to_owned();
        if id.is_empty() {
            self.profile.name.to_owned()
        } else {
            id
        }
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
                        FloorSource::PackAmbient => {
                            format!("a spec pack declares {package} {floor} ambient here")
                        }
                        FloorSource::ProfilePin => {
                            format!("{} ships {package} {floor}", self.environment_label())
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

    /// Select the whole-file floor, then recount every candidate shape using
    /// the same retained source operands and each window's authored count axis.
    pub(in crate::analyser) fn flush_gated_arity_calls(&mut self) {
        for call in std::mem::take(&mut self.pending_gated_arity) {
            let floor = call
                .axis
                .and_then(|axis| self.axis_floor(axis))
                .map(|(floor, _)| floor);
            let selected = tcl_registry::arity::ArityWindow::select(call.windows, floor.as_deref());
            let arity = selected.map_or(call.fallback, |window| window.arity);
            let Some(count) = call.original.count_for_arity(arity) else {
                continue;
            };
            let Some(diagnostic) = super::validity::source_arity_verdict(
                &call.original,
                &call.display_name,
                arity,
                &count,
                call.synopsis,
            ) else {
                continue;
            };
            let elsewhere = call.windows.iter().find(|window| {
                Some(**window) != selected
                    && call
                        .original
                        .count_for_arity(window.arity)
                        .is_some_and(|other| {
                            !other.indeterminate && window.arity.accepts(other.minimum)
                        })
            });
            let diagnostic = match (elsewhere, floor.as_deref()) {
                (Some(window), Some(floor)) => Self::window_mismatch_diagnostic(
                    &call,
                    *window,
                    floor,
                    selected,
                    usize::from(count.minimum),
                ),
                _ => diagnostic,
            };
            self.queue_source_arity_diagnostic(&call.original, diagnostic);
        }
    }

    /// The selected parent window determines whether a bare ensemble needs
    /// a selector. The original source receipt owns its naming applicability.
    pub(in crate::analyser) fn flush_gated_bare_ensembles(&mut self) {
        for call in std::mem::take(&mut self.pending_gated_bare_ensemble) {
            let floor = call
                .axis
                .and_then(|axis| self.axis_floor(axis))
                .map(|(floor, _)| floor);
            let arity = tcl_registry::arity::ArityWindow::select(call.windows, floor.as_deref())
                .map_or(call.fallback, |window| window.arity);
            if arity.min > 0 {
                self.queue_source_arity_diagnostic(
                    &call.original,
                    Diagnostic::new(
                        DiagCode::E001,
                        call.original.head().span(),
                        format!("'{}' requires a subcommand", call.display_name),
                        Severity::Error,
                    ),
                );
            }
        }
    }

    /// The **W149** message for a call whose count fits `window` rather than
    /// the shape the resolved `floor` selects.
    ///
    /// Names the direction, because the two have opposite fixes: a call
    /// written for a *later* release needs the floor raised (or the call
    /// rewritten), while one written for an *earlier* release needs the call
    /// rewritten (the floor cannot go backwards without abandoning
    /// everything else the file requires).
    fn window_mismatch_diagnostic(
        call: &GatedArityCall,
        window: tcl_registry::arity::ArityWindow,
        floor: &str,
        selected: Option<tcl_registry::arity::ArityWindow>,
        count: usize,
    ) -> Diagnostic {
        let package = call.axis.map_or("the package", VersionGateAxis::name);
        let name = &call.display_name;
        let fits = window.lifecycle.introduced.map_or_else(
            || "another release".to_owned(),
            |at| format!("{package} {at}"),
        );
        let now = selected.map_or_else(
            || format!("the resolved floor {floor} selects the command's default shape"),
            |current| {
                current.lifecycle.introduced.map_or_else(
                    || format!("the resolved floor is {floor}"),
                    |at| format!("the resolved floor {floor} selects the {package} {at} shape"),
                )
            },
        );
        // The same fact as `now`, but as a *noun phrase*: the tail says "write
        // the call for …", which needs the shape named, not the whole clause
        // about which floor selected it.
        let shape = selected.map_or_else(
            || "the command's default shape".to_owned(),
            |current| {
                current.lifecycle.introduced.map_or_else(
                    || format!("the shape at the resolved floor {floor}"),
                    |at| format!("the {package} {at} shape"),
                )
            },
        );
        // Later-release window ⇒ the file could adopt it; earlier-release
        // window ⇒ the call is left over from one.
        let forward = window
            .lifecycle
            .introduced
            .is_some_and(|at| tcl_registry::version::compare(at, floor).is_gt());
        let tail = if forward {
            format!(
                "raise the floor with `package require {package} {}`, or write the call for {shape}",
                window.lifecycle.introduced.unwrap_or(floor)
            )
        } else {
            format!(
                "that shape was valid until {}; write the call for {shape}",
                window.lifecycle.retired.unwrap_or(floor)
            )
        };
        Diagnostic::new(
            DiagCode::W149,
            call.original.invocation_span(),
            format!("{count} arguments to '{name}' matches {fits}, but {now} — {tail}"),
            Severity::Warning,
        )
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
    /// `Keyed` → the session override or the oldest-supported default).
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
        // An **ambient** placement (the F5 surfaces) is part of the
        // runtime — its floor always applies. A **hosted** placement
        // (Tk / Itcl on plain Tcl) floors only once the package is
        // actually in play via a require: the missing-require case stays
        // W120's alone, never double-flagged with a version diagnostic.
        // Both read the resolved context's floor map, whose keyed axes
        // already carry the session pins (`--bigip-version`) or their D5
        // oldest-supported defaults.
        let generation = self.analysis_context();
        let context = generation.context();
        let pin_applies = context
            .placement(pkg)
            .is_some_and(|placement| placement.ambient || has_unconditional_require);
        let pin_floor = pin_applies
            .then(|| {
                context
                    .placement_floor(pkg)
                    .map(|floor| floor.as_str().to_owned())
            })
            .flatten();
        // A pack's `ambient_package` row is ambient by construction — the
        // package comes with the dialect, so like an ambient profile pin its
        // floor applies whether or not the file requires anything.
        let pack_floor = self.pack_ambient_floor(pkg);

        let candidates = [
            require_floor.map(|floor| (floor.to_owned(), FloorSource::Require)),
            pack_floor.map(|floor| (floor.to_owned(), FloorSource::PackAmbient)),
            pin_floor.map(|floor| (floor, FloorSource::ProfilePin)),
        ];
        // Highest floor wins; at equal versions the earliest `FloorSource`
        // wins, since the variants are declared in reporting precedence.
        // `max_by_key` would have kept the last equal one instead, so the
        // comparison is written out.
        candidates.into_iter().flatten().reduce(|best, next| {
            match tcl_registry::version::compare(&next.0, &best.0) {
                std::cmp::Ordering::Greater => next,
                std::cmp::Ordering::Equal if next.1 < best.1 => next,
                _ => best,
            }
        })
    }

    /// The floor a loaded `SpecTcl` pack declared for `pkg` as an ambient
    /// package, if any.
    ///
    /// Reads the overlaid registry rather than the profile: an
    /// `ambient_package` row is authored outside this repository, so there is
    /// no compiled-in [`tcl_dialect::LibraryPin`] to find it in. A session
    /// with no packs never reaches the registry cache at all.
    fn pack_ambient_floor(&self, pkg: &str) -> Option<&'static str> {
        if self.pack_overlay == 0 {
            return None;
        }
        self.analysis_context().context().pack_ambient_floor(pkg)
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
    if site.fix_payload.word_spans.iter().any(Option::is_none) {
        return None;
    }
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
            dialect: Some(&site.dialect),
            effective_version: Some(floor),
        })?;
    let span = match resolved.target {
        DeprecationFixTarget::Word(index) => site.fix_payload.word_spans.get(index).copied()??,
        DeprecationFixTarget::Invocation => site.fix_payload.invocation,
    };
    // Source applicability establishes no entered handler or equivalent
    // rewrite. The hook supplies a proposal for explicit review only.
    let safety = crate::irules_checks::FixSafety::RequiresReview;
    Some(super::super::types::CodeFix {
        span,
        new_text: resolved.new_text,
        description: resolved.description,
        safety,
    })
}

/// Where a resolved package-version floor came from — an explicit
/// `package require`, a pack's `ambient_package` declaration, or the active
/// profile's library pin (§7.1).
///
/// The order of the variants is the **reporting** precedence at equal
/// versions, and it is deliberate. All three are lower bounds and compose by
/// taking the greatest; when two are equal the message should name the one
/// closest to the author's own control, because that is the one they can act
/// on. A `package require` is in the file being read; an `ambient_package` is
/// in a pack the workspace owns; a profile pin is compiled into this server
/// and is the only one the reader cannot change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FloorSource {
    /// A versioned, unconditional `package require` in the file.
    Require,
    /// A `SpecTcl` pack's `ambient_package` row for this package.
    PackAmbient,
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
    /// written in ([`ResolvedInvocation::authored_source_format_arguments`]), so no command
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
    /// [`ResolvedInvocation::authored_source_format_arguments`]: crate::analyser::diagnostic_registry::source_format_arguments
    /// [`FormatType::Sprintf`]: tcl_registry::patterns::FormatType
    pub(in crate::analyser) fn record_dsl_format_sites(
        &mut self,
        cmd_name: &str,
        templates: &[crate::analyser::commands::OriginalFormatTemplate],
    ) {
        use tcl_registry::patterns::FormatType;
        // Written command names are reporting labels only. The template's
        // kind, effective ordinal and literal bytes come from the same sealed
        // source projection as the binary-field and editor consumers.
        for template in templates {
            if template.format.kind != FormatType::Sprintf {
                continue;
            }
            let Ok(fmt) = std::str::from_utf8(&template.bytes) else {
                continue;
            };
            if template.format.scan {
                for (_, feature, min) in tcl_syntax::scan::version_gated_uses(fmt) {
                    self.dsl_gate_sites.push(DslGateSite {
                        span: template.span,
                        code: DiagCode::W138,
                        what: format!("`scan` conversion {feature} in '{cmd_name}'"),
                        min,
                    });
                }
            } else {
                for use_ in tcl_syntax::format::version_gated_uses(fmt) {
                    self.dsl_gate_sites.push(DslGateSite {
                        span: template.span,
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
                // §5.4 range mode: the feature is fine at the effective
                // (primary) version, but a *declared* core target can
                // still predate it — the assistance range warning.
                if let Some(context) = self.range_context.as_ref()
                    && let Some(outside) = context.targets_outside_window(
                        &VersionGateAxis::TclCore.version_axis(),
                        Some(site.min.as_package_version()),
                        None,
                    )
                {
                    new_diags.push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W150,
                        site.span,
                        format!(
                            "{} requires Tcl {} but the declared targets include {}.",
                            site.what,
                            site.min.as_package_version(),
                            range_target_names(&outside)
                        ),
                        Severity::Warning,
                    ));
                }
                continue;
            }
            new_diags.push(crate::analyser::types::Diagnostic::new(
                site.code,
                site.span,
                format!(
                    "{} requires Tcl {} but {} provides {}.",
                    site.what,
                    site.min.as_package_version(),
                    self.profile.name,
                    effective.as_package_version()
                ),
                Severity::Warning,
            ));
        }
        self.result.diagnostics.extend(new_diags);
    }
}

#[cfg(test)]
mod tests;
