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

//! Analyser-level iRules event-context checks.
//!
//! These checks run per command from
//! [`super::commands::Analyser::emit_dispatch_site_diagnostics`] and
//! are all gated on the `f5-irules` dialect.  Several of them also
//! consult the enclosing `when EVENT` block via
//! [`super::state::Analyser::current_event`] — set during the body
//! walk of a `when` command.
//!
//! Diagnostic codes:
//!
//! - **IRULE1002** (WARNING): unknown iRules event name.
//! - **IRULE1003** (WARNING): deprecated iRules event.
//! - **IRULE1004** (HINT): `when` block missing explicit `priority`.
//! - **IRULE2003** (ERROR): unsafe iRules command (context escalation).
//! - **IRULE3102** (WARNING): `HTTP::path`/`uri`/`query` getter without `-normalized`.
//! - **IRULE2101** (HINT): heavy `regexp` in a hot event.
//! - **IRULE4001** (WARNING): write to `static::` outside `RULE_INIT`.
//! - **IRULE4003** (HINT): variable scoping concern across events.
//! - **IRULE5001** (HINT): ungated `log` in a high-frequency event.
//! - **IRULE5003** (HINT): `while {$v != 0}` decrement loop can miss zero.
//! - **IRULE5006** (WARNING): top-level-only command in a nested body.
//! - **IRULE5007** (WARNING): event-context command used at top level.
//! - **IRULE6001** (WARNING): global namespace variable usage.

use std::sync::OnceLock;
use tcl_core_types::DiagCode;

use tcl_lexer::{Token, TokenType};
use tcl_registry::events::{EventProps, EventRegistry, EventRequires};
use tcl_registry::profiles::ProfileRegistry;

use super::state::Analyser;
use super::types::{CodeFix, Diagnostic, Severity};

/// Process-wide cached iRules event registry.  The data is static, so
/// building it once and sharing it avoids rebuilding the table on every
/// command (`EventRegistry::build` walks the whole event-props table).
fn event_registry() -> &'static EventRegistry {
    static REGISTRY: OnceLock<EventRegistry> = OnceLock::new();
    REGISTRY.get_or_init(EventRegistry::build)
}

/// Process-wide cached F5 profile registry.  Like [`event_registry`], the
/// data is static, so building it once and sharing it avoids rebuilding the
/// profile / protocol-namespace tables on every IRULE1001 check.
fn profile_registry() -> &'static ProfileRegistry {
    static REGISTRY: OnceLock<ProfileRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ProfileRegistry::build)
}

/// `profile_info_description` — an informational hint string when a command's
/// `required` profiles are not confirmed present on the file, or `None` when
/// there is no requirement or the file's profile stack already covers it.
fn profile_info_description(
    required: &[&str],
    file_profiles: &[&str],
    profiles: &ProfileRegistry,
) -> Option<String> {
    if required.is_empty() || profiles.stack_satisfies(required, file_profiles) {
        return None;
    }
    let mut sorted: Vec<&str> = required.to_vec();
    sorted.sort_unstable();
    Some(format!(
        "assumes profile {} on the virtual server",
        sorted.join(" or ")
    ))
}

/// A `; `-joined human-readable list of the protocol-stack requirements
/// an `event` fails to satisfy for a command.
fn missing_requirements_description(
    props: &EventProps,
    requires: &EventRequires,
    profiles: &ProfileRegistry,
) -> String {
    if requires.init_only {
        return "only valid in RULE_INIT".to_string();
    }
    let mut reasons: Vec<String> = Vec::new();
    if requires.flow && !props.flow {
        reasons.push("no active traffic flow (non-flow event)".to_string());
    }
    if requires.client_side && !props.client_side {
        reasons.push("no client-side connection".to_string());
    }
    if requires.server_side && !props.server_side {
        reasons.push("no server-side connection".to_string());
    }
    if let Some(t) = requires.transport
        && !props.transport.contains(&t)
    {
        let actual = if props.transport.is_empty() {
            "none".to_string()
        } else {
            props.transport.join("/")
        };
        reasons.push(format!("transport is {actual}, needs {t}"));
    }
    if !requires.profiles.is_empty()
        && !profiles.stack_satisfies(requires.profiles, props.implied_profiles)
    {
        let mut sorted: Vec<&str> = requires.profiles.to_vec();
        sorted.sort_unstable();
        reasons.push(format!("requires profile {}", sorted.join(" or ")));
    }
    reasons.join("; ")
}

fn is_hot_event(event: &str) -> bool {
    event_registry().get_props(event).is_some_and(|p| p.hot)
}

fn is_deprecated_event(event: &str) -> bool {
    event_registry()
        .get_props(event)
        .is_some_and(|p| p.deprecated)
}

/// Maps a command name to the argument index that names the written
/// variable.
fn var_write_index(cmd_name: &str) -> Option<usize> {
    match cmd_name {
        "append" | "const" | "global" | "incr" | "lappend" | "ledit" | "lpop" | "lset" | "set"
        | "unset" | "variable" => Some(0),
        "array" | "gets" => Some(1),
        _ => None,
    }
}

/// Return the `static::` variable name a command
/// writes, or `None`.
fn static_var_from_set<'a>(cmd_name: &str, args: &'a [String]) -> Option<&'a str> {
    if cmd_name == "set" && args.first().is_some_and(|f| f.starts_with("static::")) {
        return Some(args[0].as_str());
    }
    if cmd_name == "array" && args.len() >= 2 && args[0] == "set" && args[1].starts_with("static::")
    {
        return Some(args[1].as_str());
    }
    None
}

/// Return the `::`-qualified global variable
/// name a command writes, or `None`.
fn global_var_from_command<'a>(cmd_name: &str, args: &'a [String]) -> Option<&'a str> {
    let idx = var_write_index(cmd_name)?;
    let var = args.get(idx)?;
    if var.starts_with("::") {
        Some(var.as_str())
    } else {
        None
    }
}

/// A plain variable name that is
/// implicitly global when written in `RULE_INIT`.
fn implicit_global_var_from_command<'a>(cmd_name: &str, args: &'a [String]) -> Option<&'a str> {
    let idx = var_write_index(cmd_name)?;
    // `set var` with one arg is a read, not a write.
    if cmd_name == "set" && args.len() < 2 {
        return None;
    }
    // `unset` destroys variables; it doesn't create implicit globals.
    if cmd_name == "unset" {
        return None;
    }
    // `array` only creates implicit globals via `array set`.
    if cmd_name == "array" && args.first().map(String::as_str) != Some("set") {
        return None;
    }
    let var = args.get(idx)?;
    if !var.starts_with("::") && !var.starts_with("static::") {
        Some(var.as_str())
    } else {
        None
    }
}

impl Analyser {
    /// Run every analyser-level iRules event check for one command.
    /// No-op outside the `f5-irules` dialect.  Called from
    /// `emit_dispatch_site_diagnostics`.
    pub(super) fn emit_irules_event_checks(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        if self.dialect() != "f5-irules" {
            return;
        }
        let event = self.current_event.clone();
        let event_ref = event.as_deref();
        if let Some(ev) = event_ref {
            self.emit_irule1001_command_event_validity(cmd_name, cmd_tok, ev);
        }
        self.emit_irule1002_unknown_event(cmd_name, args, arg_tokens);
        self.emit_irule1003_deprecated_event(cmd_name, args, arg_tokens);
        self.emit_irule1004_when_missing_priority(cmd_name, args, cmd_tok);
        self.emit_irule2003_unsafe_command(cmd_name, cmd_tok);
        self.emit_irule3102_unnormalised_getter(cmd_name, args, cmd_tok);
        self.emit_irule2101_heavy_regex(cmd_name, cmd_tok, event_ref);
        self.emit_irule5001_ungated_log(cmd_name, cmd_tok, event_ref);
        self.emit_irule4001_static_write(cmd_name, args, cmd_tok, event_ref);
        self.emit_irule4003_var_scope(cmd_name, args, cmd_tok, event_ref);
        self.emit_irule6001_global_var(cmd_name, args, arg_tokens, cmd_tok, event_ref);
        self.emit_irule5006_top_level_only(cmd_name, cmd_tok);
        self.emit_irule5007_event_context(cmd_name, cmd_tok, event_ref);
        self.emit_irule5003_loop_bound_inequality(cmd_name, args, arg_tokens);
    }

    /// Compute and cache the file-level profile stack for IRULE1001's
    /// informational hint.  No-op outside
    /// the `f5-irules` dialect.  Called once from [`Self::analyse`] after the
    /// registry is built; the result is read immutably by
    /// [`Self::emit_irule1001_command_event_validity`].
    pub(super) fn compute_irules_file_profiles(&mut self) {
        if self.dialect() != "f5-irules" {
            return;
        }
        self.irules_file_profiles = Some(tcl_registry::profiles::compute_file_profiles(
            &self.source,
            event_registry(),
            profile_registry(),
        ));
    }

    /// **IRULE1001.** A command used in an iRules event where it is invalid or
    /// ineffective — registry-legality-matrix driven:
    ///
    /// - **Warning** when the command is illegal in the event: either
    ///   explicitly excluded (`'X' cannot be used in EVENT. Available in: …`)
    ///   or its protocol-stack requirements are unmet (`'X' may not work in
    ///   EVENT: <reasons>`).
    /// - **Hint** when the command is legal but assumes a profile the file
    ///   does not confirm (`'X' assumes profile Y on the virtual server`),
    ///   driven either by the command's protocol namespace (`HTTP::` ⇒ HTTP)
    ///   or its own `event_requires.profiles`.
    ///
    /// Only fires inside a `when EVENT` block; `emit_irules_event_checks`
    /// supplies the enclosing `event`.
    fn emit_irule1001_command_event_validity(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        event: &str,
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let Some(spec) = ({
            use tcl_registry::ProfileQueries;
            tcl_dialect::DialectProfile::irules().resolve_command(registry, cmd_name)
        }) else {
            return;
        };
        let events = event_registry();
        let profiles = profile_registry();

        if registry.is_irules_command_legal_in_event(cmd_name, event, events, profiles) {
            // Legal — only an informational profile hint may fire.  Namespace
            // profiles (`HTTP::respond` ⇒ HTTP) take precedence over the
            // command's own `event_requires.profiles`.
            let file_profiles: Vec<&str> = self
                .irules_file_profiles
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(String::as_str)
                .collect();
            let hint = cmd_name
                .split_once("::")
                .and_then(|(prefix, _)| profiles.get_namespace(prefix))
                .and_then(|ns| profile_info_description(ns.profiles, &file_profiles, profiles))
                .or_else(|| {
                    spec.event_requires.as_ref().and_then(|req| {
                        profile_info_description(req.profiles, &file_profiles, profiles)
                    })
                });
            if let Some(info) = hint {
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::Irule1001,
                    span: cmd_tok.span,
                    message: format!("'{cmd_name}' {info}."),
                    severity: Severity::Hint,
                    fixes: Vec::new(),
                });
            }
            return;
        }

        // Illegal — produce a detailed warning.
        if spec.excluded_events.contains(&event) {
            let valid = registry.irules_events_for_command(cmd_name, events, profiles);
            let mut hint = String::new();
            if !valid.is_empty() {
                let shown: Vec<&str> = valid.iter().take(5).copied().collect();
                hint = format!(" Available in: {}", shown.join(", "));
                if valid.len() > 5 {
                    use std::fmt::Write as _;
                    let _ = write!(hint, " (+{} more)", valid.len() - 5);
                }
                hint.push('.');
            }
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule1001,
                span: cmd_tok.span,
                message: format!("'{cmd_name}' cannot be used in {event}.{hint}"),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
            return;
        }

        // Layer-based requirements — produce a reasoned warning.  An unknown
        // event has no props to explain, so emit the generic form.
        if let Some(req) = spec.event_requires.as_ref() {
            let message = match events.get_props(event) {
                None => format!("'{cmd_name}' may not work in {event}."),
                Some(props) => {
                    let desc = missing_requirements_description(props, req, profiles);
                    format!("'{cmd_name}' may not work in {event}: {desc}.")
                }
            };
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule1001,
                span: cmd_tok.span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **IRULE5003.** `while {$var != 0} { incr var -1 }` can miss zero.
    fn emit_irule5003_loop_bound_inequality(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if cmd_name != "while" || args.len() < 2 {
            return;
        }
        let Some(tok) = arg_tokens.first() else {
            return;
        };
        let Some(var_name) = find_loop_ne_zero(&args[0]) else {
            return;
        };
        if !body_decrements(&args[1], &var_name) {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule5003,
            span: tok.span,
            message: format!(
                "Loop condition '${var_name} != 0' can miss zero if decremented past it. \
                 Consider '${var_name} > 0'."
            ),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **IRULE5006.** A top-level-only command (`when` / `proc` /
    /// `priority` / `timing`) used inside a nested body.
    fn emit_irule5006_top_level_only(&mut self, cmd_name: &str, cmd_tok: Token) {
        if self.body_depth == 0 {
            return;
        }
        let is_top_level = self
            .registry
            .as_ref()
            .is_some_and(|r| r.is_irules_top_level_only(cmd_name));
        if !is_top_level {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule5006,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is only valid at the top level of an iRule."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE5007.** An event-context command (one with event
    /// requirements) used at the top level, outside any `when` block.
    fn emit_irule5007_event_context(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        event: Option<&str>,
    ) {
        if self.body_depth != 0 || event.is_some() {
            return;
        }
        let Some(registry) = self.registry else {
            return;
        };
        if registry.is_irules_top_level_only(cmd_name) {
            return;
        }
        let requires_event = registry
            .get(cmd_name)
            .is_some_and(|s| s.event_requires.is_some());
        if !requires_event {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule5007,
            span: cmd_tok.span,
            message: format!(
                "'{cmd_name}' requires an event context — use it inside a `when` block."
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE1002.** `when` references an unknown iRules event name.
    /// Only literal event names are validated (`$var` / `[cmd]` skipped).
    fn emit_irule1002_unknown_event(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if cmd_name != "when" {
            return;
        }
        let (Some(event_name), Some(tok)) = (args.first(), arg_tokens.first()) else {
            return;
        };
        if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
            return;
        }
        if event_registry().is_known(event_name) {
            // Known event — check the declared BIG-IP version range
            // (explicit data, or the 15.0 axis baseline) against the
            // session's target release (the D5 oldest-supported default
            // when nothing pins it).
            let target = self.library_versions.bigip_version.clone().or_else(|| {
                tcl_dialect::VersionKey::BigipVersion
                    .default_version()
                    .map(str::to_owned)
            });
            if let Some(target) = target
                && !event_registry().event_available_at(event_name, &target)
            {
                let (min, max) = event_registry()
                    .event_version_range(event_name)
                    .unwrap_or((None, None));
                let range = match (min, max) {
                    (Some(min), Some(max)) => format!("BIG-IP {min} through {max}"),
                    (Some(min), None) => format!("BIG-IP {min} and later"),
                    (None, Some(max)) => format!("BIG-IP releases up to {max}"),
                    (None, None) => "an unknown BIG-IP range".to_owned(),
                };
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::Irule1002,
                    span: tok.span,
                    message: format!(
                        "iRules event '{event_name}' exists in {range}, but the target \
                         release is {target}."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule1002,
            span: tok.span,
            message: format!("Unknown iRules event '{event_name}'. Check the event name spelling."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE2003.** Unsafe iRules command (context escalation).
    fn emit_irule2003_unsafe_command(&mut self, cmd_name: &str, cmd_tok: Token) {
        let is_unsafe = self
            .registry
            .as_ref()
            .is_some_and(|r| r.is_unsafe(cmd_name));
        if !is_unsafe {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule2003,
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is unsafe in iRules and may allow context escalation"),
            severity: Severity::Error,
            fixes: Vec::new(),
        });
    }

    /// **IRULE3102.** `HTTP::path` / `HTTP::uri` / `HTTP::query` getter
    /// used without `-normalized`.  Analyser-level per-command check that
    /// fires in `tcl diag` and the LSP (the flow-path getter scan in
    /// `irules_checks.rs` is the separate PyO3-bridge surface).  Reuses
    /// the shared getter-form predicate so the trigger stays
    /// single-sourced.
    fn emit_irule3102_unnormalised_getter(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
    ) {
        let fires = self
            .registry
            .as_ref()
            .is_some_and(|r| crate::irules_checks::is_unnormalised_getter(r, cmd_name, args));
        if !fires {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule3102,
            span: cmd_tok.span,
            message: crate::irules_checks::format_message(cmd_name),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE1003.** Deprecated iRules event referenced by `when`.
    fn emit_irule1003_deprecated_event(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if cmd_name != "when" {
            return;
        }
        let (Some(event_name), Some(tok)) = (args.first(), arg_tokens.first()) else {
            return;
        };
        if !is_deprecated_event(event_name) {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule1003,
            span: tok.span,
            message: format!("'{event_name}' event is deprecated."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE1004.** `when` block missing an explicit `priority`.
    fn emit_irule1004_when_missing_priority(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
    ) {
        if cmd_name != "when" {
            return;
        }
        // when EVENT { body }            → args = [EVENT, body]
        // when EVENT priority N { body } → args = [EVENT, "priority", N, body]
        if args.len() >= 2 && args[1] == "priority" {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule1004,
            span: cmd_tok.span,
            message:
                "'when' missing an explicit priority. Add 'priority <N>' to control execution order."
                    .to_string(),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **IRULE2101.** Heavy `regexp` in a high-frequency event.
    fn emit_irule2101_heavy_regex(&mut self, cmd_name: &str, cmd_tok: Token, event: Option<&str>) {
        if cmd_name != "regexp" {
            return;
        }
        let Some(event) = event else { return };
        if !is_hot_event(event) {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule2101,
            span: cmd_tok.span,
            message: format!(
                "'regexp' in {event} may be expensive at high traffic volumes. \
                 Consider 'string match', 'switch -glob', or a data-group lookup."
            ),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **IRULE5001.** Ungated `log` in a high-frequency event.
    fn emit_irule5001_ungated_log(&mut self, cmd_name: &str, cmd_tok: Token, event: Option<&str>) {
        if cmd_name != "log" {
            return;
        }
        let Some(event) = event else { return };
        if !is_hot_event(event) {
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule5001,
            span: cmd_tok.span,
            message: format!(
                "'log' in {event} fires on every request. \
                 Set a debug flag in CLIENT_ACCEPTED (e.g. set debug 0) and gate with \
                 if {{$debug}} {{...}}."
            ),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **IRULE4001.** Write to a `static::` variable outside `RULE_INIT`.
    fn emit_irule4001_static_write(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        event: Option<&str>,
    ) {
        if event == Some("RULE_INIT") {
            return;
        }
        let Some(var_name) = static_var_from_set(cmd_name, args) else {
            return;
        };
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule4001,
            span: cmd_tok.span,
            message: format!(
                "Writing to '{var_name}' outside RULE_INIT is dangerous. \
                 static:: variables are shared across all connections; \
                 concurrent writes can cause race conditions."
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE4003.** Variable scoping concern across `when` events.
    fn emit_irule4003_var_scope(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        event: Option<&str>,
    ) {
        if cmd_name != "set" {
            return;
        }
        let Some(event) = event else { return };
        if event == "RULE_INIT" {
            return;
        }
        // Must be a write (`set var value`), not a read (`set var`).
        if args.len() < 2 {
            return;
        }
        let var_name = &args[0];
        // static:: handled by IRULE4001/4002; global ::vars skipped.
        if var_name.starts_with("static::") || var_name.starts_with("::") {
            return;
        }

        let registry = event_registry();
        let blocks = scan_when_blocks(&self.source);
        let mut concerns: Vec<String> = Vec::new();
        for (other_event, bodies) in &blocks {
            if other_event == event {
                continue;
            }
            for body in bodies {
                if var_referenced_in(var_name, body) {
                    if let Some(note) = registry.variable_scope_note(event, other_event) {
                        concerns.push(note);
                    }
                    break;
                }
            }
        }
        if concerns.is_empty() {
            return;
        }
        let mut msg = format!("Variable '{var_name}': {}", concerns[0]);
        if concerns.len() > 1 {
            msg = format!("{msg}; {}", concerns[1..].join("; "));
        }
        self.result.diagnostics.push(Diagnostic {
            code: DiagCode::Irule4003,
            span: cmd_tok.span,
            message: msg,
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **IRULE6001.** Global namespace variable usage (CMP pinning).
    fn emit_irule6001_global_var(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
        event: Option<&str>,
    ) {
        // `global varname` — imports from the global namespace.
        if cmd_name == "global"
            && let Some(var_name) = args.first()
        {
            let static_name = format!("static::{var_name}");
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule6001,
                span: cmd_tok.span,
                message: format!(
                    "'global {var_name}' imports from the global namespace, \
                     forcing CMP compatibility mode and pinning the virtual server \
                     to a single TMM. Use '{static_name}' instead."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
            return;
        }

        // `set ::var`, `incr ::var`, etc.
        if let Some(var_name) = global_var_from_command(cmd_name, args) {
            let bare = &var_name[2..];
            let static_name = format!("static::{bare}");
            let fix_span = var_write_index(cmd_name)
                .and_then(|idx| arg_tokens.get(idx))
                .map_or(cmd_tok.span, |t| t.span);
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule6001,
                span: fix_span,
                message: format!(
                    "Global namespace variable '{var_name}' forces CMP compatibility \
                     mode, pinning the virtual server to a single TMM. \
                     Use '{static_name}' instead."
                ),
                severity: Severity::Warning,
                fixes: vec![CodeFix {
                    span: fix_span,
                    new_text: static_name.clone(),
                    description: format!("Replace '{var_name}' with '{static_name}'"),
                    // IRULE6001: `static::` variables have per-TMM storage and a
                    // different lifetime from a global — the CMP fix, and a change of
                    // where the value lives.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }],
            });
            return;
        }

        // Implicit globals in RULE_INIT: `set var value` (no `::`) is global
        // because RULE_INIT executes at the global namespace scope.
        if event == Some("RULE_INIT")
            && let Some(bare) = implicit_global_var_from_command(cmd_name, args)
        {
            let static_name = format!("static::{bare}");
            let fix_span = var_write_index(cmd_name)
                .and_then(|idx| arg_tokens.get(idx))
                .map_or(cmd_tok.span, |t| t.span);
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule6001,
                span: fix_span,
                message: format!(
                    "'{bare}' in RULE_INIT is implicitly global — RULE_INIT \
                     runs at the global namespace scope. This forces CMP \
                     compatibility mode, pinning the virtual server to a \
                     single TMM. Use '{static_name}' instead."
                ),
                severity: Severity::Warning,
                fixes: vec![CodeFix {
                    span: fix_span,
                    new_text: static_name.clone(),
                    description: format!("Replace '{bare}' with '{static_name}'"),
                    // IRULE6001: as above.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }],
            });
        }
    }
}

/// Return `{event_name: [body_text, ...]}` for all `when` blocks via
/// balanced-brace scanning.
fn scan_when_blocks(source: &str) -> Vec<(String, Vec<String>)> {
    let bytes = source.as_bytes();
    let mut result: Vec<(String, Vec<String>)> = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = source[search..].find("when") {
        let kw = search + rel;
        search = kw + 4;
        // `\bwhen\s+` — require a word boundary before `when`.
        if kw > 0 && is_word_byte(bytes[kw - 1]) {
            continue;
        }
        let mut pos = kw + 4;
        // Require at least one whitespace separator after `when`.
        if pos >= bytes.len() || !is_ws(bytes[pos]) {
            continue;
        }
        while pos < bytes.len() && is_ws(bytes[pos]) {
            pos += 1;
        }
        // Event name: [A-Z_][A-Z0-9_]*
        let name_start = pos;
        if pos >= bytes.len() || !(bytes[pos] == b'_' || bytes[pos].is_ascii_uppercase()) {
            continue;
        }
        pos += 1;
        while pos < bytes.len()
            && (bytes[pos] == b'_'
                || bytes[pos].is_ascii_uppercase()
                || bytes[pos].is_ascii_digit())
        {
            pos += 1;
        }
        let event = source[name_start..pos].to_string();
        // Skip optional `priority N` / `timing enable|disable` before `{`.
        while pos < bytes.len() && bytes[pos] != b'{' {
            if is_ws(bytes[pos]) {
                pos += 1;
            } else if source[pos..].starts_with("priority") {
                pos += 8;
                while pos < bytes.len() && is_ws(bytes[pos]) {
                    pos += 1;
                }
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
            } else if source[pos..].starts_with("timing") {
                pos += 6;
                while pos < bytes.len() && is_ws(bytes[pos]) {
                    pos += 1;
                }
                while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
                    pos += 1;
                }
            } else {
                break;
            }
        }
        if pos >= bytes.len() || bytes[pos] != b'{' {
            continue;
        }
        // Balanced-brace scan.
        let mut depth = 1i32;
        let start = pos + 1;
        pos += 1;
        while pos < bytes.len() && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'\\' => pos += 1,
                _ => {}
            }
            pos += 1;
        }
        let body = source[start..pos.saturating_sub(1).min(source.len())].to_string();
        if let Some(entry) = result.iter_mut().find(|(e, _)| e == &event) {
            entry.1.push(body);
        } else {
            result.push((event, vec![body]));
        }
        search = pos;
    }
    result
}

/// Is `$var_name` referenced in *body*?  Matches
/// `$name` (not followed by a word char) or `${name}`.
fn var_referenced_in(var_name: &str, body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while let Some(rel) = body[i..].find('$') {
        let dollar = i + rel;
        i = dollar + 1;
        let after = dollar + 1;
        // `${name}` form.
        if body[after..].starts_with('{') {
            let inner = after + 1;
            if body[inner..].starts_with(var_name)
                && body.as_bytes().get(inner + var_name.len()) == Some(&b'}')
            {
                return true;
            }
            continue;
        }
        // `$name` not followed by a word char.
        if body[after..].starts_with(var_name) {
            let end = after + var_name.len();
            if bytes.get(end).is_none_or(|b| !is_word_byte(*b)) {
                return true;
            }
        }
    }
    false
}

/// Find the first `$var != 0` / `$var ne 0` / `0 != $var` / `0 ne $var`
/// comparison and return the bare variable name.  Uses leftmost-match,
/// alternation-A-then-B order.
fn find_loop_ne_zero(condition: &str) -> Option<String> {
    let b = condition.as_bytes();
    for i in 0..b.len() {
        if let Some(name) = match_dollar_ne_zero(b, i) {
            return Some(name);
        }
        if let Some(name) = match_zero_ne_dollar(b, i) {
            return Some(name);
        }
    }
    None
}

/// `\$\{?(\w+)\}? \s* (?:!=|ne) \s* 0`
fn match_dollar_ne_zero(b: &[u8], mut p: usize) -> Option<String> {
    if b.get(p)? != &b'$' {
        return None;
    }
    p += 1;
    let braced = b.get(p) == Some(&b'{');
    if braced {
        p += 1;
    }
    let name_start = p;
    while p < b.len() && is_word_byte(b[p]) {
        p += 1;
    }
    if p == name_start {
        return None;
    }
    let name = String::from_utf8_lossy(&b[name_start..p]).into_owned();
    if braced && b.get(p) == Some(&b'}') {
        p += 1;
    }
    p = skip_ws(b, p);
    p = match_ne_op(b, p)?;
    p = skip_ws(b, p);
    if b.get(p) == Some(&b'0') {
        Some(name)
    } else {
        None
    }
}

/// `0 \s* (?:!=|ne) \s* \$\{?(\w+)\}?`
fn match_zero_ne_dollar(b: &[u8], mut p: usize) -> Option<String> {
    if b.get(p)? != &b'0' {
        return None;
    }
    p += 1;
    p = skip_ws(b, p);
    p = match_ne_op(b, p)?;
    p = skip_ws(b, p);
    if b.get(p)? != &b'$' {
        return None;
    }
    p += 1;
    let braced = b.get(p) == Some(&b'{');
    if braced {
        p += 1;
    }
    let name_start = p;
    while p < b.len() && is_word_byte(b[p]) {
        p += 1;
    }
    if p == name_start {
        return None;
    }
    Some(String::from_utf8_lossy(&b[name_start..p]).into_owned())
}

/// Match `!=` or `ne`, returning the position past the operator.
fn match_ne_op(b: &[u8], p: usize) -> Option<usize> {
    let is_ne = (b.get(p) == Some(&b'!') && b.get(p + 1) == Some(&b'='))
        || (b.get(p) == Some(&b'n') && b.get(p + 1) == Some(&b'e'));
    is_ne.then_some(p + 2)
}

/// Does *body* contain `incr <var> -<digits>` (word-bounded) for the
/// given variable?
fn body_decrements(body: &str, var_name: &str) -> bool {
    let b = body.as_bytes();
    let mut search = 0usize;
    while let Some(rel) = body[search..].find("incr") {
        let kw = search + rel;
        search = kw + 4;
        // `\bincr` — word boundary before `incr`.
        if kw > 0 && is_word_byte(b[kw - 1]) {
            continue;
        }
        let mut p = kw + 4;
        // `\s+`
        let ws_start = p;
        p = skip_ws(b, p);
        if p == ws_start {
            continue;
        }
        // `(\w+)`
        let name_start = p;
        while p < b.len() && is_word_byte(b[p]) {
            p += 1;
        }
        if p == name_start {
            continue;
        }
        let name = &body[name_start..p];
        // `\s+`
        let ws2 = p;
        p = skip_ws(b, p);
        if p == ws2 {
            continue;
        }
        // `-\d+`
        if b.get(p) != Some(&b'-') {
            continue;
        }
        p += 1;
        let digit_start = p;
        while p < b.len() && b[p].is_ascii_digit() {
            p += 1;
        }
        if p == digit_start {
            continue;
        }
        // `\b` after the digits — next char must be a non-word byte or end.
        if b.get(p).is_some_and(|c| is_word_byte(*c)) {
            continue;
        }
        if name == var_name {
            return true;
        }
    }
    false
}

fn skip_ws(b: &[u8], mut p: usize) -> usize {
    while p < b.len() && is_ws(b[p]) {
        p += 1;
    }
    p
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_word_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(source: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        let res = a.analyse(source, "f5-irules");
        res.diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with("IRULE"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn has(source: &str, code: &str) -> bool {
        codes(source).iter().any(|(c, _)| c == code)
    }

    #[test]
    fn irule1004_fires_for_when_without_priority() {
        assert!(has("when HTTP_REQUEST { set x 1 }", "IRULE1004"));
    }

    #[test]
    fn irule1004_suppressed_with_explicit_priority() {
        assert!(!has(
            "when HTTP_REQUEST priority 100 { set x 1 }",
            "IRULE1004"
        ));
    }

    #[test]
    fn irule1002_fires_for_unknown_event() {
        assert!(has("when BOGUS_EVENT { log local0. hi }", "IRULE1002"));
    }

    #[test]
    fn irule1002_quiet_for_known_event() {
        assert!(!has("when HTTP_REQUEST { set x 1 }", "IRULE1002"));
    }

    #[test]
    fn irule3102_fires_for_unnormalised_getter() {
        assert!(has("when HTTP_REQUEST { set u [HTTP::uri] }", "IRULE3102"));
    }

    #[test]
    fn irule3102_quiet_with_normalized_flag() {
        assert!(!has(
            "when HTTP_REQUEST { set u [HTTP::uri -normalized] }",
            "IRULE3102"
        ));
    }

    #[test]
    fn irule3102_fires_for_getter_nested_in_argument() {
        assert!(has(
            "when HTTP_REQUEST { HTTP::respond 200 content [HTTP::query] }",
            "IRULE3102"
        ));
    }

    #[test]
    fn irule2003_fires_for_unsafe_command() {
        let cs = codes("when HTTP_REQUEST { uplevel 1 { set x 1 } }");
        assert!(cs.iter().any(|(c, _)| c == "IRULE2003"));
    }

    #[test]
    fn irule1003_fires_for_deprecated_event() {
        assert!(has("when AUTH_SUCCESS { log local0. hi }", "IRULE1003"));
    }

    #[test]
    fn irule1003_quiet_for_live_event() {
        assert!(!has("when HTTP_REQUEST { set x 1 }", "IRULE1003"));
    }

    #[test]
    fn irule2101_fires_for_regexp_in_hot_event() {
        assert!(has(
            "when HTTP_REQUEST { regexp {^/a} [HTTP::uri] }",
            "IRULE2101"
        ));
    }

    #[test]
    fn irule2101_quiet_outside_hot_event() {
        assert!(!has("when CLIENT_DATA { regexp {^/a} $x }", "IRULE2101"));
    }

    #[test]
    fn irule5001_fires_for_log_in_hot_event() {
        assert!(has("when HTTP_REQUEST { log local0. hi }", "IRULE5001"));
    }

    #[test]
    fn irule4001_fires_for_static_write_outside_rule_init() {
        assert!(has("when HTTP_REQUEST { set static::c 1 }", "IRULE4001"));
    }

    #[test]
    fn irule4001_suppressed_in_rule_init() {
        assert!(!has("when RULE_INIT { set static::c 1 }", "IRULE4001"));
    }

    #[test]
    fn irule6001_fires_for_global_qualified_write() {
        assert!(has("when HTTP_REQUEST { set ::counter 0 }", "IRULE6001"));
    }

    #[test]
    fn irule6001_fires_for_implicit_global_in_rule_init() {
        assert!(has("when RULE_INIT { set greeting hi }", "IRULE6001"));
    }

    #[test]
    fn irule6001_fires_for_global_command() {
        let cs = codes("when HTTP_REQUEST { global shared }");
        assert!(
            cs.iter()
                .any(|(c, m)| c == "IRULE6001" && m.contains("'global shared'"))
        );
    }

    #[test]
    fn irule5006_fires_for_proc_in_nested_body() {
        assert!(has(
            "when HTTP_REQUEST { proc helper {} { return 1 } }",
            "IRULE5006"
        ));
    }

    #[test]
    fn irule5006_quiet_for_top_level_proc() {
        assert!(!has("proc helper {} { return 1 }", "IRULE5006"));
    }

    #[test]
    fn irule5007_fires_for_event_command_at_top_level() {
        assert!(has("HTTP::respond 200", "IRULE5007"));
    }

    #[test]
    fn irule5007_quiet_inside_when_block() {
        assert!(!has("when HTTP_REQUEST { HTTP::respond 200 }", "IRULE5007"));
    }

    #[test]
    fn irule4003_fires_for_cross_event_variable() {
        let src = "when HTTP_REQUEST { set token abc }\nwhen CLIENT_DATA { log local0. $token }";
        assert!(has(src, "IRULE4003"));
    }

    #[test]
    fn irule5003_fires_for_ne_zero_decrement_loop() {
        assert!(has(
            "when HTTP_REQUEST { while {$count != 0} { incr count -1 } }",
            "IRULE5003"
        ));
    }

    #[test]
    fn irule5003_fires_for_zero_ne_braced_form() {
        assert!(has(
            "when HTTP_REQUEST { while {0 ne ${n}} { incr n -2 } }",
            "IRULE5003"
        ));
    }

    #[test]
    fn irule5003_quiet_for_greater_than_zero() {
        assert!(!has(
            "when HTTP_REQUEST { while {$x > 0} { incr x -1 } }",
            "IRULE5003"
        ));
    }

    #[test]
    fn irule5003_quiet_when_body_decrements_other_var() {
        assert!(!has(
            "when HTTP_REQUEST { while {$count != 0} { incr other -1 } }",
            "IRULE5003"
        ));
    }

    #[test]
    fn find_loop_ne_zero_handles_both_orders() {
        assert_eq!(find_loop_ne_zero("$count != 0").as_deref(), Some("count"));
        assert_eq!(find_loop_ne_zero("0 ne ${n}").as_deref(), Some("n"));
        assert_eq!(find_loop_ne_zero("$x > 0"), None);
    }

    #[test]
    fn no_irule_checks_outside_f5_dialect() {
        let mut a = Analyser::new();
        let res = a.analyse("when AUTH_SUCCESS { set static::c 1 }", "tcl");
        assert!(
            !res.diagnostics
                .iter()
                .any(|d| d.code.as_str().starts_with("IRULE"))
        );
    }

    #[test]
    fn scan_when_blocks_extracts_priority_form() {
        let blocks = scan_when_blocks("when HTTP_REQUEST priority 5 { body1 }");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, "HTTP_REQUEST");
        assert_eq!(blocks[0].1[0].trim(), "body1");
    }

    #[test]
    fn var_referenced_in_matches_both_forms() {
        assert!(var_referenced_in("x", "log local0. $x"));
        assert!(var_referenced_in("x", "log local0. ${x}"));
        assert!(!var_referenced_in("x", "log local0. $xyz"));
        assert!(!var_referenced_in("x", "no dollar here"));
    }

    // IRULE1001 — command-event-validity.  Messages and severities below are
    // pinned against the command-event-validity checker (run on the same
    // snippets) and the registry legality matrix.

    /// `(severity, message)` for every IRULE1001 diagnostic, whole-file.
    fn irule1001(source: &str) -> Vec<(Severity, String)> {
        let mut a = Analyser::new();
        a.analyse(source, "f5-irules")
            .diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::Irule1001)
            .map(|d| (d.severity, d.message))
            .collect()
    }

    #[test]
    fn irule1001_quiet_for_legal_command_with_inferred_profile() {
        // HTTP_REQUEST infers the HTTP profile, so HTTP::respond is fully
        // satisfied — no diagnostic at all.
        assert!(irule1001("when HTTP_REQUEST { HTTP::respond 200 content \"ok\" }").is_empty());
    }

    #[test]
    fn irule1001_warns_command_unmet_requirements() {
        // HTTP::respond in CLIENT_ACCEPTED: the L4 event provides no HTTP
        // profile, so it "may not work" with the missing-requirement reason.
        let diags = irule1001("when CLIENT_ACCEPTED { HTTP::respond 200 content \"ok\" }");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].0, Severity::Warning);
        assert_eq!(
            diags[0].1,
            "'HTTP::respond' may not work in CLIENT_ACCEPTED: requires profile FASTHTTP or HTTP."
        );
    }

    #[test]
    fn irule1001_hints_unconfirmed_namespace_profile() {
        // SSL::cipher is legal in HTTP_REQUEST but assumes an SSL profile the
        // file doesn't confirm — an informational hint, OR-listed + sorted.
        let diags = irule1001("when HTTP_REQUEST { SSL::cipher name }");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].0, Severity::Hint);
        assert_eq!(
            diags[0].1,
            "'SSL::cipher' assumes profile CLIENTSSL or PERSIST or SERVERSSL or \
             SSL_PERSISTENCE on the virtual server."
        );
    }

    #[test]
    fn irule1001_warns_excluded_event_with_available_list() {
        // HA::status is excluded from RULE_INIT; the "Available in: …" list is
        // the first five (sorted) legal events plus a "(+N more)" tail.
        let diags = irule1001("when RULE_INIT { HA::status }");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].0, Severity::Warning);
        assert!(
            diags[0]
                .1
                .starts_with("'HA::status' cannot be used in RULE_INIT. Available in: "),
            "{}",
            diags[0].1
        );
        assert!(diags[0].1.contains("(+") && diags[0].1.ends_with("more)."));
    }

    #[test]
    fn irule1001_hint_suppressed_by_profile_directive() {
        // A leading `# profiles: CLIENTSSL` directive confirms the SSL stack,
        // so SSL::cipher's profile hint is suppressed.
        assert!(
            irule1001("# profiles: CLIENTSSL\nwhen HTTP_REQUEST { SSL::cipher name }").is_empty()
        );
    }

    #[test]
    fn irule1001_hint_suppressed_by_inferred_event_profile() {
        // A CLIENTSSL_HANDSHAKE handler in the file infers CLIENTSSL, which
        // covers SSL::cipher's requirement — hint suppressed.
        let src =
            "when CLIENTSSL_HANDSHAKE { log local0. hi }\nwhen HTTP_REQUEST { SSL::cipher name }";
        assert!(irule1001(src).is_empty());
    }

    #[test]
    fn irule1001_quiet_at_top_level_without_event() {
        // No enclosing `when` block ⇒ no event context ⇒ the check never runs.
        assert!(irule1001("HTTP::respond 200").is_empty());
    }

    #[test]
    fn irule1001_quiet_outside_f5_dialect() {
        let mut a = Analyser::new();
        let res = a.analyse("when CLIENT_ACCEPTED { HTTP::respond 200 }", "tcl");
        assert!(
            !res.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::Irule1001)
        );
    }

    #[test]
    fn profile_info_description_or_lists_sorted_unconfirmed() {
        let profiles = profile_registry();
        // No requirement → None.
        assert_eq!(profile_info_description(&[], &[], profiles), None);
        // Confirmed by file → None.
        assert_eq!(
            profile_info_description(&["HTTP"], &["HTTP"], profiles),
            None
        );
        // Unconfirmed → sorted OR-list.
        assert_eq!(
            profile_info_description(&["HTTP", "FASTHTTP"], &[], profiles).as_deref(),
            Some("assumes profile FASTHTTP or HTTP on the virtual server")
        );
    }

    #[test]
    fn missing_requirements_description_reports_init_only_and_profiles() {
        let profiles = profile_registry();
        let events = event_registry();
        let init_req = EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: true,
            flow: false,
            capability: None,
        };
        let props = events.get_props("HTTP_REQUEST").expect("known event");
        assert_eq!(
            missing_requirements_description(props, &init_req, profiles),
            "only valid in RULE_INIT"
        );
    }
}
