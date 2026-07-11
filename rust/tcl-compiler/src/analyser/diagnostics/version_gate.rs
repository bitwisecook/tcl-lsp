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

//! Version-aware diagnostics (**W135** / **W136**).
//!
//! A command or option can declare a `min_version` — the lowest version of its
//! owning package (Tk, a tcllib package, `argparse`, …) that provides it.  When
//! the document `package require`s that package at a version whose guaranteed
//! floor is *below* the declared minimum, using the command (W135) or option
//! (W136) will fail at runtime.
//!
//! The floor is a whole-file fact: `package require` may appear anywhere, so
//! candidate uses are buffered during the walk
//! ([`Analyser::record_version_gate_sites`]) and decided post-walk
//! ([`Analyser::flush_version_gate_diagnostics`]) once every `package require`
//! is known.  A package required *without* a version is permissive (no floor,
//! every version accepted), and a package not required at all is the domain of
//! W120 (missing `package require`); neither draws a version diagnostic.

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
        let Some(registry) = self.registry.as_ref() else {
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

    /// Emit W135/W136 for each buffered site whose package was `package
    /// require`d at a floor below the required `min_version`.  Sites whose
    /// package has no version floor (required without a version, or not
    /// required — the latter handled by W120) are skipped.
    pub(in crate::analyser) fn flush_version_gate_diagnostics(&mut self) {
        if self.version_gate_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.version_gate_sites);
        let mut new_diags: Vec<Diagnostic> = Vec::new();
        for site in sites {
            let Some(floor) = self.package_version_floor(site.package) else {
                continue;
            };
            if tcl_registry::version::meets_min(floor, site.min_version) {
                continue;
            }
            let (code, message) = match &site.item {
                VersionGateItem::Command(cmd) => (
                    DiagCode::W135,
                    format!(
                        "'{cmd}' requires {} {} but `package require` guarantees only {floor}.",
                        site.package, site.min_version
                    ),
                ),
                VersionGateItem::Option { command, option } => (
                    DiagCode::W136,
                    format!(
                        "Option '{option}' on '{command}' requires {} {} but \
                         `package require` guarantees only {floor}.",
                        site.package, site.min_version
                    ),
                ),
            };
            new_diags.push(Diagnostic::new(code, site.span, message, Severity::Warning));
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// The highest *guaranteed* lower bound among this file's `package require
    /// <pkg> <req>` lines; `None` when `pkg` is not required, or required
    /// without a version (permissive — every version is accepted).
    ///
    /// Conditional requires — an optional probe such as
    /// `catch {package require Tk 8.7}` or a `package require` inside an `if`
    /// arm — are excluded: they do not guarantee the version on every path, so
    /// counting them would raise the floor and wrongly suppress a real W135/W136.
    fn package_version_floor(&self, pkg: &str) -> Option<&str> {
        self.result
            .package_requires
            .iter()
            .filter(|r| r.name == pkg && !r.conditional)
            .filter_map(|r| r.version.as_deref())
            .map(tcl_registry::version::requirement_lower_bound)
            .max_by(|a, b| tcl_registry::version::compare(a, b))
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

    #[test]
    fn ungated_option_is_silent() {
        // `-width` carries no `min_version`, so no version diagnostic.
        let src = "package require Tk 8.6\nentry .e -width 10\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn require_without_version_is_permissive() {
        // No version floor to compare against ⇒ nothing flagged.
        let src = "package require Tk\nentry .e -placeholder hi\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn no_require_draws_no_version_diagnostic() {
        // Missing `package require` is W120's job, not a version diagnostic.
        let src = "entry .e -placeholder hi\n";
        assert!(!fires(src, "W136"), "{:?}", version_diags(src));
    }

    #[test]
    fn command_below_floor_fires_w135() {
        // `ttk::button` needs Tk 8.5; the require guarantees only 8.4.
        let src = "package require Tk 8.4\nttk::button .b\n";
        assert!(fires(src, "W135"), "{:?}", version_diags(src));
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
