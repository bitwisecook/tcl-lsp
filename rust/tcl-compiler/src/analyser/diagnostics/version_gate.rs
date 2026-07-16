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
//! A command or option can declare a `min_version` — the lowest version of its
//! owning package (Tk, a tcllib package, `argparse`, …) that provides it.  When
//! the resolved floor — the profile's library pin (§7.1) raised by any
//! versioned `package require` — is *below* the declared minimum, using the
//! command (W135) or option (W136) will fail at runtime.
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

use super::super::state::Analyser;
use super::super::types::{Diagnostic, Severity};

/// A command/option use gated behind a package `min_version`, recorded during
/// the walk and checked post-walk against the resolved `package require` floor.
#[derive(Debug)]
pub(in crate::analyser) struct VersionGateSite {
    /// Span the diagnostic anchors to (command head, or option token).
    span: Span,
    /// The gating package (the spec's [`owning_package`]).
    ///
    /// [`owning_package`]: tcl_registry::CommandSpec::owning_package
    package: &'static str,
    /// The minimum package version the command/option needs.
    min_version: &'static str,
    /// What is gated — a command, or an option on one.
    item: VersionGateItem,
}

/// Payload distinguishing a gated command (W135) from a gated option (W136).
#[derive(Debug)]
enum VersionGateItem {
    Command(String),
    Option { command: String, option: String },
}

impl Analyser {
    /// Buffer version-gated command/option uses at a dispatch site.
    ///
    /// The command's `min_version` (if any) records a W135 candidate at the
    /// command head; each option argument matching a value-gated
    /// [`OptionSpec::min_version`] records a W136 candidate at the option token.
    /// Option scanning mirrors [`Analyser::emit_w004_dialect_invalid_option`]:
    /// it stops at `--`, skips negative-number literals and dynamic
    /// (`Var`/`Cmd`) tokens, and resolves subcommand-scoped options.
    ///
    /// [`OptionSpec::min_version`]: tcl_registry::OptionSpec
    pub(in crate::analyser) fn record_version_gate_sites(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };
        let Some(pkg) = spec.owning_package() else {
            return;
        };

        // Command-level gate.
        if let Some(min) = spec.min_version {
            self.version_gate_sites.push(VersionGateSite {
                span: cmd_tok.span,
                package: pkg,
                min_version: min,
                item: VersionGateItem::Command(cmd_name.to_owned()),
            });
        }

        // Option-level gates.  Resolve subcommand-scoped options when the first
        // argument names a subcommand.
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| {
                let first = args.first().map(String::as_str).unwrap_or_default();
                spec.resolve_subcommand(first)
            })
            .flatten();
        let (options, start_idx) = match sub_match {
            Some(sub) => (sub.options, 1usize),
            None => (spec.options, 0usize),
        };
        if options.is_empty() {
            return;
        }

        let mut i = start_idx;
        while i < args.len() {
            let arg = args[i].as_str();
            if arg == "--" {
                break;
            }
            if !arg.starts_with('-') || arg.len() < 2 {
                i += 1;
                continue;
            }
            // Skip negative-number literals (`-1`, `-1.5`).
            let rest = arg[1..].trim_start_matches('-');
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                i += 1;
                continue;
            }
            // Skip dynamic-value args — a `Var`/`Cmd` token's text is not the
            // literal option name.
            if i < arg_tokens.len() && matches!(arg_tokens[i].kind, TokenType::Var | TokenType::Cmd)
            {
                i += 1;
                continue;
            }
            if let Some(opt) = options.iter().find(|o| o.matches(arg)) {
                if let Some(min) = opt.min_version
                    && i < arg_tokens.len()
                {
                    self.version_gate_sites.push(VersionGateSite {
                        span: arg_tokens[i].span,
                        package: pkg,
                        min_version: min,
                        item: VersionGateItem::Option {
                            command: cmd_name.to_owned(),
                            option: arg.to_owned(),
                        },
                    });
                }
                // Skip the value word(s) this option consumes, so a value that
                // itself looks like a flag (`-textvariable -placeholder`) is not
                // re-tested as an option — the same `value_word_count` skip the
                // W004 dialect-option loop uses (RUST_ISSUE_077).
                i += 1 + opt.value_word_count(args, i);
                continue;
            }
            i += 1;
        }
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
            let Some((floor, source)) = self.package_version_floor(site.package) else {
                continue;
            };
            if tcl_registry::version::meets_min(&floor, site.min_version) {
                continue;
            }
            let guarantee = match source {
                FloorSource::Require => format!("`package require` guarantees only {floor}"),
                FloorSource::ProfilePin => {
                    format!("{} ships {} {floor}", self.profile.name, site.package)
                }
            };
            let (code, message) = match &site.item {
                VersionGateItem::Command(cmd) => (
                    DiagCode::W135,
                    format!(
                        "'{cmd}' requires {} {} but {guarantee}.",
                        site.package, site.min_version
                    ),
                ),
                VersionGateItem::Option { command, option } => (
                    DiagCode::W136,
                    format!(
                        "Option '{option}' on '{command}' requires {} {} but {guarantee}.",
                        site.package, site.min_version
                    ),
                ),
            };
            new_diags.push(Diagnostic {
                code,
                span: site.span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
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

    /// Buffer format/scan %-string DSL uses at a dispatch site — the
    /// registry marks the %-string argument positions with
    /// [`ArgRole::FormatString`] / [`ArgRole::ScanFormat`], so no command
    /// name is matched here.
    ///
    /// [`ArgRole::FormatString`]: tcl_registry::arg_role::ArgRole
    /// [`ArgRole::ScanFormat`]: tcl_registry::arg_role::ArgRole
    pub(in crate::analyser) fn record_dsl_format_sites(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(registry) = self.registry else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        for (role, is_scan) in [(ArgRole::FormatString, false), (ArgRole::ScanFormat, true)] {
            for idx in registry.arg_indices_for_role(cmd_name, &arg_strs, role) {
                let (Some(fmt), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                // A dynamic token's text is not the literal %-string.
                if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
                    continue;
                }
                if is_scan {
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
        // RUST_ISSUE_077: `-placeholder`'s value is itself `-placeholder`. The
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
            .filter(|d| matches!(d.code.as_str(), "W135" | "W136"))
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
