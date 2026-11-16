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
//! - **IRULE1004** (HINT): event-handler priority omitted where that
//!   handler's registry policy requires an explicit value.
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
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use super::state::Analyser;
use super::types::Severity;

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

/// Whether `event` is deprecated at `target` — the unified lifecycle rule:
/// deprecation applies only while the event is still available, so a retired
/// event reports as retired (IRULE1002), never as deprecated.
fn is_deprecated_event(event: &str, target: Option<&str>) -> bool {
    event_registry()
        .get_props(event)
        .is_some_and(|p| p.is_deprecated(target))
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
        arg_single: &[bool],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let event = self.current_event.clone();
        let event_ref = event.as_deref();
        if let Some(ev) = event_ref {
            self.emit_irule1001_command_event_validity(cmd_name, args, cmd_tok, ev);
        }
        self.emit_irule1002_unknown_event(cmd_name, args, arg_tokens, arg_single);
        self.emit_irule1003_deprecated_event(cmd_name, args, arg_tokens, arg_single);
        self.emit_irule1004_event_handler_missing_priority(
            cmd_name, args, arg_tokens, arg_single, cmd_tok,
        );
        self.emit_irule2003_unsafe_command(cmd_name, cmd_tok);
        self.emit_irule3102_unnormalised_getter(arg_tokens, cmd_tok);
        self.emit_irule2101_heavy_regex(cmd_name, cmd_tok, event_ref);
        self.emit_irule5001_ungated_log(cmd_name, cmd_tok, event_ref);
        let execution_context = self.irules_execution_context(scope_path);
        self.emit_irule5006_top_level_only(cmd_name, cmd_tok, execution_context);
        self.emit_irule5007_top_level_executable(cmd_name, cmd_tok, execution_context);
        self.emit_irule5003_loop_bound_inequality(cmd_name, args, arg_tokens);
    }

    /// Compute and cache the file-level profile stack for IRULE1001's
    /// informational hint.  No-op outside
    /// the `f5-irules` dialect.  Called once from [`Self::analyse`] after the
    /// registry is built; the result is read immutably by
    /// [`Self::emit_irule1001_command_event_validity`].
    pub(super) fn compute_irules_file_profiles(&mut self) {
        if !self.profile.is_irules() {
            return;
        }
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        self.irules_file_profiles = Some(
            tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
                &self.source,
                event_registry(),
                profile_registry(),
                registry,
                &self.head_identities,
            ),
        );
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
    #[allow(
        clippy::too_many_lines,
        reason = "event legality, protocol requirements, and profile hints intentionally produce one cohesive diagnostic"
    )]
    fn emit_irule1001_command_event_validity(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        event: &str,
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let Some(spec) = crate::environment_ingress::irules_context()
            .context()
            .resolve_spec(registry, cmd_name)
        else {
            return;
        };
        let events = event_registry();
        let profiles = profile_registry();
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let requirements = spec.event_requirements_for_args(&arg_refs);

        if registry.is_irules_call_legal_in_event(cmd_name, &arg_refs, event, events, profiles) {
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
            let hint = (!requirements.matched_form)
                .then(|| {
                    profiles.namespace_for_command_spec(spec).and_then(|ns| {
                        profile_info_description(ns.profiles, &file_profiles, profiles)
                    })
                })
                .flatten()
                .or_else(|| {
                    requirements.requires.and_then(|req| {
                        profile_info_description(req.profiles, &file_profiles, profiles)
                    })
                });
            if let Some(info) = hint {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule1001,
                        cmd_tok.span,
                        format!("'{cmd_name}' {info}."),
                        Severity::Hint,
                    ));
            }
            return;
        }

        // A registry-declared call form can have a closed set of legal events
        // (for example, a read form that only has a live message in one event).
        // Report that exact contract before the broader layer/profile fallback.
        // This is entirely form-data-driven: the analyser does not name the
        // command or its selector words.
        if !requirements.only_in.is_empty() && !requirements.only_in.contains(&event) {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::Irule1001,
                    cmd_tok.span,
                    format!(
                        "'{cmd_name}' cannot be used in {event}. Available in: {}.",
                        requirements.only_in.join(", ")
                    ),
                    Severity::Warning,
                ));
            return;
        }

        // Illegal — produce a detailed warning.
        if spec.excluded_events.contains(&event) {
            let valid = registry.irules_events_for_call(cmd_name, &arg_refs, events, profiles);
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
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::Irule1001,
                    cmd_tok.span,
                    format!("'{cmd_name}' cannot be used in {event}.{hint}"),
                    Severity::Warning,
                ));
            return;
        }

        // Layer-based requirements — produce a reasoned warning.  An unknown
        // event has no props to explain, so emit the generic form.
        if let Some(req) = requirements.requires {
            let message = match events.get_props(event) {
                None => format!("'{cmd_name}' may not work in {event}."),
                Some(props) => {
                    let desc = missing_requirements_description(props, req, profiles);
                    format!("'{cmd_name}' may not work in {event}: {desc}.")
                }
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::Irule1001,
                    cmd_tok.span,
                    message,
                    Severity::Warning,
                ));
        }
    }

    /// **IRULE5003.** `while {$var != 0} { incr var -1 }` can miss zero.
    ///
    /// The loop is the one whose iteration plan is a bare condition, read
    /// with its body off the plan's words
    /// ([`crate::value_transfer::resolved_iteration_plan`]); the body
    /// decrements the variable when a cell update the registry declares an
    /// integer increment of it adds a negative amount. An analyse with no
    /// registry reads the catalogue's, as the loop checks do.
    fn emit_irule5003_loop_bound_inequality(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        let Ok(generation) = self.try_analysis_context() else {
            return;
        };
        let registry = generation.commands();
        let Some(spec) = generation.context().resolve_spec(registry, cmd_name) else {
            return;
        };
        let head = spec.descriptor_key().label();
        let Some(plan) = crate::value_transfer::resolved_iteration_plan(registry, head, args)
        else {
            return;
        };
        let (tcl_registry::value_transfer::IterableKind::Condition(condition), Some(body)) =
            (plan.iterable, plan.body)
        else {
            return;
        };
        let (Some(tok), Some(condition), Some(body)) = (
            arg_tokens.get(condition.0),
            args.get(condition.0),
            args.get(body.body.0),
        ) else {
            return;
        };
        let Some(var_name) = find_loop_ne_zero(condition) else {
            return;
        };
        if !body_decrements(
            body,
            &var_name,
            &self.analysis_context(),
            self.lexer_config(),
        ) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule5003,
                tok.span,
                format!(
                    "Loop condition '${var_name} != 0' can miss zero if decremented past it. \
                 Consider '${var_name} > 0'."
                ),
                Severity::Hint,
            ));
    }

    /// **IRULE5006.** A top-level-only command (`when` / `proc` /
    /// `priority` / `timing`) used inside a nested body.
    ///
    /// **Measurements §4c** — the rule compiler's ban is *lexical* and its
    /// scan recurses into **braced script literals**: `eval {proc …}` and
    /// `uplevel #0 {proc …}` (any level spelling, any nesting) inside a
    /// `when` body are rejected identically to bare `proc`. The analyser
    /// mirrors that through the generic `ArgRole::Body` recursion in
    /// [`super::commands`]: a braced `eval`/`uplevel` body is walked as a
    /// script with the enclosing event context intact, so this check fires
    /// on the nested `proc` with the same diagnostic as the bare form.
    /// Script text held in a variable escapes the scan — the non-literal
    /// body is not walked (this check abstains) and the realm state widens
    /// instead (see `widen_irules_dynamic_eval_bindings`).
    fn emit_irule5006_top_level_only(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        context: tcl_registry::events::IrulesExecutionContext,
    ) {
        let misplaced = self.registry.as_ref().is_some_and(|registry| {
            registry.irules_command_placement(cmd_name, context)
                == tcl_registry::events::IrulesCommandPlacement::RequiresTopLevel
        });
        if !misplaced {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule5006,
                cmd_tok.span,
                format!("'{cmd_name}' is only valid at the top level of an iRule."),
                Severity::Warning,
            ));
    }

    /// **IRULE5007.** An executable command used on iRules' declaration-only
    /// top level. The registry trait marks the complete allowed declaration
    /// surface; everything else must execute inside an event or procedure.
    fn emit_irule5007_top_level_executable(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        context: tcl_registry::events::IrulesExecutionContext,
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        if registry.irules_command_placement(cmd_name, context)
            != tcl_registry::events::IrulesCommandPlacement::RequiresEventOrProcedure
        {
            return;
        }
        self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::Irule5007,
    cmd_tok.span,
    format!(
                "'{cmd_name}' is executable and cannot appear at iRules top level — use it inside a `when` block or a top-level `proc`."
            ),
    Severity::Warning,
));
    }

    pub(super) fn irules_execution_context(
        &self,
        scope_path: &[usize],
    ) -> tcl_registry::events::IrulesExecutionContext {
        use tcl_registry::events::IrulesExecutionContext;

        if self.current_event.is_some() {
            IrulesExecutionContext::EventBody
        } else if self.scope_path_in_proc_body(scope_path) {
            IrulesExecutionContext::ProcedureBody
        } else if self.body_depth == 0 {
            IrulesExecutionContext::TopLevel
        } else {
            IrulesExecutionContext::InvalidNestedBody
        }
    }

    /// Resolve one source-aware, top-level event declaration without checking
    /// whether its event selector is known. This preserves IRULE1002's job:
    /// it reports a valid declaration whose selector is unknown, while the
    /// executable inventories reject that same declaration.
    fn irules_event_declaration_shape(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> Option<tcl_registry::events::IrulesTopLevelDeclaration> {
        if self.body_depth != 0 {
            return None;
        }
        let registry = self.registry.as_deref()?;
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            arg_tokens,
            arg_single,
        )?;
        let arguments = tcl_registry::events::IrulesDeclarationArguments::new(
            &words, arg_tokens, arg_single, &closed,
        )?;
        registry.irules_top_level_declaration_shape(cmd_name, arguments)
    }

    /// **IRULE1002.** `when` references an unknown iRules event name.
    /// Only literal event names are validated (`$var` / `[cmd]` skipped).
    fn emit_irule1002_unknown_event(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) {
        if !matches!(
            self.irules_event_declaration_shape(cmd_name, args, arg_tokens, arg_single),
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Event { .. })
        ) {
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
            // session's target release (the oldest-supported default when
            // nothing pins it).
            let target = self.library_versions.bigip_version.clone().or_else(|| {
                tcl_dialect::VersionKey::BigipVersion
                    .default_version()
                    .map(str::to_owned)
            });
            if let Some(target) = target
                && let Some(lifecycle) = event_registry().event_lifecycle(event_name)
                && !lifecycle.available_at(Some(&target))
            {
                // Retirement is exclusive: `retired: 10.0.0` means the event
                // is gone *from* 10.0.0, so the surviving range ends one
                // release earlier and is described as "before" it.
                let range = match (lifecycle.introduced, lifecycle.retired) {
                    (Some(min), Some(retired)) => {
                        format!("BIG-IP {min} up to (not including) {retired}")
                    }
                    (Some(min), None) => format!("BIG-IP {min} and later"),
                    (None, Some(retired)) => {
                        format!("BIG-IP releases before {retired}")
                    }
                    (None, None) => "an unknown BIG-IP range".to_owned(),
                };
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule1002,
                        tok.span,
                        format!(
                            "iRules event '{event_name}' exists in {range}, but the target \
                         release is {target}."
                        ),
                        Severity::Warning,
                    ));
            }
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule1002,
                tok.span,
                format!("Unknown iRules event '{event_name}'. Check the event name spelling."),
                Severity::Warning,
            ));
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
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule2003,
                cmd_tok.span,
                format!("'{cmd_name}' is unsafe in iRules and may allow context escalation"),
                Severity::Error,
            ));
    }

    /// **IRULE3102.** `HTTP::path` / `HTTP::uri` / `HTTP::query` getter
    /// used without `-normalized`.  Analyser-level per-command check that
    /// fires in `tcl diag` and the LSP (the flow-path getter scan in
    /// `irules_checks.rs` is a separate surface).  Reuses
    /// the shared getter-form predicate so the trigger stays
    /// single-sourced.
    fn emit_irule3102_unnormalised_getter(
        &mut self,
        argument_tokens: &[Token],
        command_token: Token,
    ) {
        let Some(original) = self.original_diagnostic_call_at(command_token, argument_tokens)
        else {
            return;
        };
        let fires = original.with_schema(|schema| {
            let arguments = schema.words.arguments();
            let values = (0..arguments.exact_argv_len()?)
                .map(|ordinal| {
                    let word = arguments.get(ordinal)?;
                    let bytes = word
                        .literal()
                        .map(str::as_bytes)
                        .or_else(|| word.native_bytes())?;
                    Some(std::str::from_utf8(bytes).ok()?.to_owned())
                })
                .collect::<Option<Vec<_>>>()?;
            crate::irules_checks::is_unnormalised_getter(
                &schema.semantics.options.keyword_table(),
                &values,
            )
            .then_some(())
        });
        if fires != Some(Some(())) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule3102,
                command_token.span,
                crate::irules_checks::format_message(original.command()),
                Severity::Warning,
            ));
    }

    /// **IRULE1003.** Deprecated iRules event referenced by `when`.
    fn emit_irule1003_deprecated_event(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) {
        if !matches!(
            self.irules_event_declaration_shape(cmd_name, args, arg_tokens, arg_single),
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Event { .. })
        ) {
            return;
        }
        let (Some(event_name), Some(tok)) = (args.first(), arg_tokens.first()) else {
            return;
        };
        let target = self.library_versions.bigip_version.clone().or_else(|| {
            tcl_dialect::VersionKey::BigipVersion
                .default_version()
                .map(str::to_owned)
        });
        if !is_deprecated_event(event_name, target.as_deref()) {
            return;
        }
        let since = event_registry()
            .event_lifecycle(event_name)
            .and_then(|life| life.deprecated)
            .map_or_else(String::new, |release| format!(" as of BIG-IP {release}"));
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule1003,
                tok.span,
                format!("'{event_name}' event is deprecated{since}."),
                Severity::Warning,
            ));
    }

    /// **IRULE1004.** An event-handler command whose registry policy requires
    /// an explicit priority has omitted its priority keyword.
    fn emit_irule1004_event_handler_missing_priority(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        cmd_tok: Token,
    ) {
        if !matches!(
            self.irules_event_declaration_shape(cmd_name, args, arg_tokens, arg_single),
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Event { .. })
        ) {
            return;
        }
        let Some(policy) = self
            .registry
            .as_deref()
            .and_then(|registry| registry.event_handler_priority(cmd_name))
        else {
            return;
        };
        if !policy.warn_when_implicit || args.iter().skip(1).any(|arg| arg == policy.keyword) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule1004,
                cmd_tok.span,
                format!(
                    "'{cmd_name}' missing an explicit {}. Add '{} <N>' to control execution order.",
                    policy.keyword, policy.keyword,
                ),
                Severity::Hint,
            ));
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
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule2101,
                cmd_tok.span,
                format!(
                    "'regexp' in {event} may be expensive at high traffic volumes. \
                 Consider 'string match', 'switch -glob', or a data-group lookup."
                ),
                Severity::Hint,
            ));
    }

    /// **IRULE5001.** Ungated `log` in a high-frequency event.
    ///
    /// "Ungated" is the load-bearing word. The message tells the author to
    /// put the call behind a debug flag, so a `log` that already sits behind
    /// one is not reported — a check that survives its own recommended fix
    /// teaches the reader to ignore it. [`Self::irules_debug_gate_opens`]
    /// decides what counts as a gate and `irules_debug_gate_depth` carries
    /// that decision down into every nested body.
    fn emit_irule5001_ungated_log(&mut self, cmd_name: &str, cmd_tok: Token, event: Option<&str>) {
        if cmd_name != "log" || self.irules_debug_gate_depth > 0 {
            return;
        }
        let Some(event) = event else { return };
        if !is_hot_event(event) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::Irule5001,
                cmd_tok.span,
                format!(
                    "'log' in {event} fires on every request. \
                 Set a debug flag in CLIENT_ACCEPTED (e.g. set debug 0) and gate with \
                 if {{$debug}} {{...}}."
                ),
                Severity::Hint,
            ));
    }

    /// Whether this command's bodies run only when a debug flag says so —
    /// the gate IRULE5001 tells the author to add.
    ///
    /// Registry-driven. The command must select its bodies by a decision
    /// rather than by iteration — [`Traits::CONTROL_FLOW`] without
    /// [`Traits::HAS_LOOP_BODY`], which leaves `if`, `switch`, and `try` —
    /// and one of the words it evaluates rather than executes must read a
    /// debug flag: a `static::` variable, or a name assigned by an event that
    /// runs less often than once per request (`set debug 0` in `RULE_INIT`,
    /// the pair the message prescribes).
    ///
    /// A loop is excluded because its selector is a sequence, not a
    /// decision: `foreach ip $static::allowlist` iterates a `static::` list
    /// without saying anything about debugging, and treating it as a gate
    /// would silence the hint for every `log` in the body.
    ///
    /// Every arm of a gating command counts, `else` included. A `log`
    /// reached only through a decision made on a debug flag is gated
    /// whichever way that decision went, and over-suppressing is the safe
    /// direction for a hint whose whole complaint is a missing gate.
    pub(super) fn irules_debug_gate_opens(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) -> bool {
        // Cheap gates first: only an iRules event body can hold the `log`
        // this suppresses, and a command that selects a body from a
        // selector has at least those two words.
        if args.len() < 2 || self.current_event.is_none() || !self.profile.is_irules() {
            return false;
        }
        let Some(selectors) = self.irules_branch_selector_words(cmd_name, args) else {
            return false;
        };
        let flags = self.irules_debug_flag_names().to_vec();
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let bindings = self.head_identities.source_bindings_ref();
        let ingress = bindings.invocation_at_source(cmd_name, cmd_tok.span.start());
        for index in selectors {
            if let Some(token) = arg_tokens.get(index) {
                for access in bindings.variable_accesses_in_span(token.span) {
                    let place = crate::var_resolve::resolve_substitution_access(
                        &access.original_spelling,
                        &access.variable_context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    if crate::connection_scope::cell_from_place(&place)
                        .is_some_and(|cell| flags.contains(&cell))
                        || names_static_variable(&access.original_spelling)
                        || (!place.dynamic
                            && place.is_global()
                            && tcl_registry::f5::namespace_storage_domain(
                                tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                                &place.ns,
                            ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace)
                    {
                        return true;
                    }
                }
            }
            let Some(word) = args.get(index) else {
                continue;
            };
            // A hint can be suppressed for unresolved named reads in a nested
            // command. Direct lexical references above use their actual cells.
            let expression =
                tcl_syntax::expr::parser::parse_expr_for_profile(word, Some(self.profile));
            // This is suppress-only advisory evidence. Structural expression
            // references can be considered even when execution has not been
            // proved; their cell still needs the retained point binding context.
            let direct_flag = expression.vars_parsed_only().iter().any(|name| {
                let place = crate::var_resolve::resolve_literal_access(
                    name,
                    &ingress.variable_context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                crate::connection_scope::cell_from_place(&place)
                    .is_some_and(|cell| flags.contains(&cell))
                    || names_static_variable(name)
                    || (!place.dynamic
                        && place.is_global()
                        && tcl_registry::f5::namespace_storage_domain(
                            tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                            &place.ns,
                        ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace)
            });
            if direct_flag
                || expression
                    .command_texts()
                    .iter()
                    .any(|text| mentions_static_variable(text))
            {
                return true;
            }
        }
        false
    }

    /// The words a branch-selecting command evaluates to choose *which* of
    /// its bodies run — the arguments it neither executes as a script nor
    /// takes as a variable name to write. `None` when the command iterates
    /// its body rather than selecting it, or does not select a body at all.
    ///
    /// Dropping the [`ArgRole::VarWrite`] words is what keeps a command whose
    /// body always runs from opening a gate. `catch` is the case: its body is
    /// unconditional and its only other arguments are the result and options
    /// variables, so `catch { log local0. hi } static::err` names a
    /// `static::` variable without any decision reading one. Together with
    /// the loop exclusion this leaves `if`, `switch`, and `case` — every one
    /// of them a command all of whose bodies are selected — so a gate never
    /// spans a body that runs regardless.
    fn irules_branch_selector_words(&self, cmd_name: &str, args: &[String]) -> Option<Vec<usize>> {
        let registry = self.registry.as_deref()?;
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let traits = registry.invocation_traits(
            cmd_name,
            &words,
            Some(self.analysis_context().context().authoring_query()),
        );
        if !traits.contains(Traits::CONTROL_FLOW) || traits.contains(Traits::HAS_LOOP_BODY) {
            return None;
        }
        let bodies = registry.arg_indices_for_role(cmd_name, &words, ArgRole::Body);
        let written = registry.arg_indices_for_role(cmd_name, &words, ArgRole::VarWrite);
        Some(
            words
                .iter()
                .enumerate()
                .filter(|(index, _)| !bodies.contains(index) && !written.contains(index))
                .map(|(index, _)| index)
                .collect(),
        )
    }

    /// Every variable name assigned by an event that runs less often than
    /// once per request — the debug flags a hot-event `log` can be gated on.
    /// Built once per source and memoised beside
    /// [`Self::irules_event_bodies`], which supplies its input.
    fn irules_debug_flag_names(&mut self) -> &[crate::connection_scope::EventCell] {
        if self.irules_debug_flags.is_none() {
            let flags = self.registry.as_deref().map_or_else(Vec::new, |registry| {
                collect_debug_flag_names(
                    registry,
                    &self.source,
                    &self.head_identities,
                    self.file_lexer_config(),
                    &self.analysis_context(),
                )
            });
            self.irules_debug_flags = Some(flags);
        }
        self.irules_debug_flags.as_deref().unwrap_or_default()
    }
}

/// Every `(event name, body texts)` pair in `source`, keyed by the registry
/// rather than by the word `when`.
///
/// The shared owner accepts only top-level commands whose effective binding at
/// that absolute offset carries [`Traits::IS_EVENT_HANDLER`]. Invalid nested
/// handlers, inert data, and unavailable command-table mutations therefore
/// cannot enter the cross-event inventory. Non-literal event names are already
/// rejected by that owner. Its caller-supplied profile preserves the TMM `}{`
/// ghost separator.
#[cfg(test)]
fn collect_event_bodies(
    source: &str,
    registry: Option<&CommandRegistry>,
    resolver: &dyn tcl_registry::events::CommandHeadResolver,
) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let Some(registry) = registry else { return out };
    let events = tcl_registry::events::EventRegistry::build();
    for handler in tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
        source, registry, resolver,
    ) {
        if !events.is_known(&handler.event) {
            continue;
        }
        let Some(body) = source.get(handler.body_span.as_range()) else {
            continue;
        };
        if let Some(entry) = out.iter_mut().find(|(name, _)| name == &handler.event) {
            entry.1.push(body.to_owned());
        } else {
            out.push((handler.event, vec![body.to_owned()]));
        }
    }
    out
}

/// Whether a selector mentions a worker namespace while no flag inventory
/// is required. Exact setup-event definitions below use the shared Tcl cells.
fn mentions_static_variable(word: &str) -> bool {
    word.contains("static::")
}

/// Whether a written variable name selects BIG-IP's `static::` namespace.
/// Suppress-only advisory evidence, like [`mentions_static_variable`]: under
/// the fresh rule-load realm an entered event body's frame leaves a relative
/// name's current namespace unproved, but a decision on a `static::` flag is
/// still the gate IRULE5001 prescribes, and over-suppressing a hint is the
/// safe direction.
fn names_static_variable(name: &str) -> bool {
    tcl_syntax::naming::written_command_parts(name.as_bytes())
        .qualifiers
        .first()
        .is_some_and(|namespace| *namespace == b"static")
}

/// Possible debug flags from infrequently invoked handlers. Reuse the source
/// kernel's positioned bindings and shared analysis CFG, including possible
/// body regions, instead of reparsing command heads or introducing a walker.
fn collect_debug_flag_names(
    registry: &CommandRegistry,
    source: &str,
    resolver: &crate::realm::CommandBindingRealm,
    config: tcl_lexer::LexerConfig,
    metadata: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> Vec<crate::connection_scope::EventCell> {
    use tcl_registry::events::EventExecutionMultiplicity;
    let mut names = std::collections::HashSet::new();
    let bindings = resolver.source_bindings_ref();
    let declarations = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    for candidate in declarations.iter().flat_map(|command| {
        bindings
            .deferred_rule_declaration_candidates(command.span.start())
            .into_iter()
            .flatten()
    }) {
        let tcl_registry::events::IrulesTopLevelDeclaration::Event { event, .. } =
            &candidate.declaration
        else {
            continue;
        };
        if !matches!(
            event_registry().execution_multiplicity(event),
            EventExecutionMultiplicity::OncePerWorkerEpoch
                | EventExecutionMultiplicity::OncePerConnection
        ) {
            continue;
        }
        let Some(body) = &candidate.body else {
            continue;
        };
        let Some(selected) = bindings.selected_source(body) else {
            continue;
        };
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(registry.profile())
            .with_context_registry(std::sync::Arc::clone(metadata));
        let Ok(text) = body.try_text() else {
            continue;
        };
        let mut script = lowerer.lower_into_script_with_bindings(text, body.base(), "::", selected);
        script.executed_source = Some(std::sync::Arc::new(body.clone()));
        let cfg = crate::cfg_builder::build_analysis_body(
            &format!("::when::{event}"),
            &script,
            crate::ir::ExecutionNamespace::exact("::"),
            registry,
            config,
            std::sync::Arc::clone(metadata),
        );
        let points = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &cfg,
            crate::connection_scope::event_resolve_context(event),
            registry,
        );
        for (&block_id, block) in &cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                for place in crate::place_bridge::def_places_with_continuation(
                    statement,
                    points.before_statement(block_id, index),
                    points.after_statement(block_id, index),
                    registry,
                ) {
                    if let Some(name) = crate::connection_scope::cell_from_place(&place) {
                        names.insert(name);
                    }
                }
            }
        }
    }
    names.into_iter().collect()
}

/// Is `$var_name` referenced in *body*?  Matches
/// `$name` (not followed by a word char) or `${name}`.
#[cfg(test)]
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

/// Whether a command in `body`, at any depth, decrements `var_name`: the
/// registry's integer cell update of it, over literal words, adding a
/// negative amount ([`super::bounds_checks::command_increment`]).
fn body_decrements(
    body: &str,
    var_name: &str,
    registry: &tcl_registry::model::ContextRegistry,
    lexer_config: tcl_lexer::LexerConfig,
) -> bool {
    let metadata = super::bounds_checks::BoundsMetadataContext::Retained(registry);
    super::bounds_checks::any_command_recursive(body, metadata, lexer_config, &mut |cmd| {
        super::bounds_checks::command_increment(cmd, var_name, metadata, lexer_config)
            .is_some_and(|amount| amount < 0)
    })
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
mod tests;
