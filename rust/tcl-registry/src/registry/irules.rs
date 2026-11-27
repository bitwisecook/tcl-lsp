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

//! iRules queries: which commands are legal in which events, top-level declarations, event handlers, data collection and taint sources.

use super::CommandRegistry;
use crate::events::{
    DataCollectionAction, DataCollectionOperation, DataCollectionProtocol, EventHandlerPriority,
};
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::side_effects::SideSwitchTarget;
use crate::spec::CommandSpec;
use crate::traits::Traits;
use tcl_dialect::model::{Family, SurfaceQuery};

/// Resolved metadata for an iRules event — the result of
/// [`CommandRegistry::event_info`].
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// The upper-cased event name as queried.
    pub event: String,
    /// Introduction / deprecation / retirement releases on the BIG-IP axis —
    /// explicit data, with an absent introducing release inheriting the axis
    /// baseline (15.0.0). Entirely unspecified for an unknown event.
    pub lifecycle: Lifecycle,
    /// The event's lifecycle state at the queried BIG-IP release.
    pub lifecycle_state: LifecycleState,
    /// Whether the event is a recognised iRules event.
    pub known: bool,
    /// `"init"` / `"once_per_connection"` / `"per_request"` / `"unknown"`.
    pub multiplicity: &'static str,
    /// Description prose, or `""` when none is recorded.
    pub description: String,
    /// Connection-side label, or `"unknown"` for an unrecognised event.
    pub side: &'static str,
    /// Transport string (`"tcp"`, `"tcp/udp"`, `""`), or `None` for an
    /// unrecognised event.
    pub transport: Option<String>,
    /// Profile types implied by the event, sorted.
    pub implied_profiles: Vec<&'static str>,
    /// Sorted names of every command valid in this event (empty when the
    /// event is unknown).
    pub valid_commands: Vec<String>,
}

impl EventInfo {
    /// Number of commands valid in this event.
    #[must_use]
    pub fn valid_command_count(&self) -> usize {
        self.valid_commands.len()
    }
}

impl CommandRegistry {
    /// The taint-source colour declared by `command`'s spec, or `None`
    /// when it is not a source.
    ///
    /// Reads the installed set's index, *derived* once from every
    /// installed spec's [`crate::CommandSpec::taint_source`]. It is deliberately **dialect-agnostic and
    /// independent of which dialects are loaded into this registry**:
    /// an iRules
    /// getter such as `HTTP::path` is a known source even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules commands.
    #[must_use]
    pub fn taint_source(&self, command: &str) -> Option<crate::taint::TaintColour> {
        super::surfaces::installed()
            .taint_sources()
            .get(command)
            .copied()
    }

    /// Return the sorted names of every f5-irules command valid in `event`.
    ///
    /// The valid-command set: a command
    /// is valid when it supports the iRules dialect, the event is not in its
    /// `excluded_events`, and either it carries no `event_requires` or the
    /// event's [`crate::events::EventProps`] satisfy them.  Returns an empty
    /// vector for an unknown event.
    #[must_use]
    pub fn valid_irules_commands_for_event<'a>(
        &'a self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> Vec<&'a str> {
        let Some(props) = events.get_props(event) else {
            return Vec::new();
        };
        // The declared version range for an F5-surface spec: explicit
        // introduction/removal data, or the axis baseline (15.0) for a
        // spec with none. `bigip_version: None` keeps the pre-version
        // behaviour (no filtering) so digest-stable callers opt in.
        let version_ok = |spec: &CommandSpec| {
            let Some(version) = bigip_version else {
                return true;
            };
            spec.lifecycle
                .with_baseline(tcl_dialect::VersionKey::BigipVersion.baseline_version())
                .available_at(Some(version))
        };
        let mut names: Vec<&str> = self
            .by_name
            .iter()
            .filter_map(|(name, specs)| {
                // Best spec for the dialect — the §5.3 most-specific rule,
                // matching `get_for_surface`.
                let spec =
                    self.best_visible(specs, Some(SurfaceQuery::any_release(Family::F5Irules)))?;
                if !version_ok(spec) {
                    return None;
                }
                if spec.excluded_events.contains(&event) {
                    return None;
                }
                if let Some(req) = spec.event_requires.as_ref()
                    && !crate::events::event_satisfies(props, req, event, profiles)
                {
                    return None;
                }
                Some(*name)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Whether `command` is legal in iRules `event` — the O(1) legality-matrix
    /// test.
    ///
    /// A command is legal when the event is known, the command supports the
    /// iRules dialect, the event is not in the command's `excluded_events`,
    /// and the command's [`crate::events::EventRequires`] (if any) are
    /// satisfied by the event's [`crate::events::EventProps`].  An unknown
    /// event is illegal for every command — an event with no props has an
    /// empty valid-command set.
    #[must_use]
    pub fn is_irules_command_legal_in_event(
        &self,
        command: &str,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        self.is_irules_call_legal_in_event(command, &[], event, events, profiles)
    }

    /// Whether a concrete iRules command call is legal in `event`.
    ///
    /// Unlike [`Self::is_irules_command_legal_in_event`], this resolves the
    /// command's registry-declared argument-prefix event forms before applying
    /// the common event matrix. Consumers pass the words after the command
    /// name; no consumer needs command-specific subcommand knowledge.
    #[must_use]
    pub fn is_irules_call_legal_in_event(
        &self,
        command: &str,
        args: &[&str],
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        let Some(props) = events.get_props(event) else {
            return false;
        };
        let Some(spec) =
            self.get_for_surface(command, Some(SurfaceQuery::any_release(Family::F5Irules)))
        else {
            return false;
        };
        if spec.excluded_events.contains(&event) {
            return false;
        }
        let requirements = spec.event_requirements_for_args(args);
        if !requirements.only_in.is_empty() && !requirements.only_in.contains(&event) {
            return false;
        }
        requirements
            .requires
            .is_none_or(|req| crate::events::event_satisfies(props, req, event, profiles))
    }

    /// Sorted iRules events where `command` is legal — the inverse of
    /// [`Self::valid_irules_commands_for_event`].  Used by the IRULE1001
    /// "Available in: …" hint.
    #[must_use]
    pub fn irules_events_for_command<'a>(
        &self,
        command: &str,
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        self.irules_events_for_call(command, &[], events, profiles)
    }

    /// Sorted iRules events where a concrete command call is legal. See
    /// [`Self::is_irules_call_legal_in_event`] for the argument-form-aware
    /// contract resolution.
    #[must_use]
    pub fn irules_events_for_call<'a>(
        &self,
        command: &str,
        args: &[&str],
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        let mut names: Vec<&str> = events
            .all_event_names()
            .into_iter()
            .filter(|event| {
                self.is_irules_call_legal_in_event(command, args, event, events, profiles)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Resolve full metadata for an iRules `event` — the engine behind
    /// `f5 irule event-info`.
    ///
    /// The event name is upper-cased and trimmed. `known` /
    /// `valid_commands` come from this registry (the cross-product is the
    /// same as [`Self::valid_irules_commands_for_event`]); the side /
    /// transport / implied-profiles / description / multiplicity come from
    /// `events`. `deprecated` is always `false` — the `when`-argument-value
    /// detail path carries no "deprecated" markers (independent of
    /// [`crate::events::EventProps::deprecated`]).
    #[must_use]
    pub fn event_info(
        &self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> EventInfo {
        let name = event.trim().to_uppercase();
        let target = bigip_version
            .or(tcl_dialect::VersionKey::BigipVersion.default_version())
            .unwrap_or("16.1.0");
        let known =
            !name.is_empty() && events.is_known(&name) && events.event_available_at(&name, target);
        let valid_commands: Vec<String> = if known {
            self.valid_irules_commands_for_event(&name, events, profiles, Some(target))
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        let props = events.get_props(&name);
        let lifecycle = events.event_lifecycle(&name).unwrap_or_default();
        EventInfo {
            lifecycle,
            lifecycle_state: lifecycle.state_at(Some(target)),
            known,
            multiplicity: events.multiplicity(&name),
            description: events.description(&name).unwrap_or("").to_owned(),
            side: props.map_or("unknown", crate::events::EventProps::side_label),
            // "No transport" is modelled as `None`, not an empty string.
            transport: props.and_then(|p| (!p.transport.is_empty()).then(|| p.transport.join("/"))),
            implied_profiles: {
                let mut v: Vec<&'static str> =
                    props.map_or_default(|p| p.implied_profiles.to_vec());
                v.sort_unstable();
                v
            },
            event: name,
            valid_commands,
        }
    }

    /// Whether `name` is valid only at the top level of an iRule script
    /// (`when`, `proc`, `priority`, `timing`).  Drives the IRULE5006 /
    /// IRULE5007 placement checks ([`Traits::IRULES_TOP_LEVEL_ONLY`]).
    #[must_use]
    pub fn is_irules_top_level_only(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| spec.traits.contains(Traits::IRULES_TOP_LEVEL_ONLY))
    }

    /// Apply the iRules command-placement contract to one resolved head.
    ///
    /// The only legal file-level forms carry
    /// [`Traits::IRULES_TOP_LEVEL_ONLY`]. Every other invocation, including a
    /// user-defined command absent from this registry, is executable and must
    /// occur inside an event or procedure. An invalid nested top-level body is
    /// not cascaded: its enclosing executable command already carries the
    /// placement finding, while declaration forms nested there remain
    /// independently reportable.
    #[must_use]
    pub fn irules_command_placement(
        &self,
        name: &str,
        context: crate::events::IrulesExecutionContext,
    ) -> crate::events::IrulesCommandPlacement {
        use crate::events::{IrulesCommandPlacement, IrulesExecutionContext};

        if self.is_irules_top_level_only(name) {
            return if context == IrulesExecutionContext::TopLevel {
                IrulesCommandPlacement::Allowed
            } else {
                IrulesCommandPlacement::RequiresTopLevel
            };
        }
        match context {
            IrulesExecutionContext::TopLevel => IrulesCommandPlacement::RequiresEventOrProcedure,
            IrulesExecutionContext::EventBody
            | IrulesExecutionContext::ProcedureBody
            | IrulesExecutionContext::InvalidNestedBody => IrulesCommandPlacement::Allowed,
        }
    }

    /// Validate and classify one invocation on iRules' top-level declaration
    /// surface. Arity, argument layout, declaration traits, and known-event
    /// membership are decided here so compiler/tooling consumers cannot admit
    /// different executable regions.
    #[must_use]
    pub fn irules_top_level_declaration(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let declaration = self.irules_top_level_declaration_shape(name, arguments)?;
        match &declaration {
            Declaration::Event { event, .. } if !events.is_known(event) => None,
            Declaration::Event { .. }
            | Declaration::Procedure { .. }
            | Declaration::Priority { .. }
            | Declaration::Timing { .. } => Some(declaration),
        }
    }

    /// Validate an iRules file-level declaration without requiring an event
    /// selector to be known.
    ///
    /// This is the source-aware declaration owner used by lowering, symbols,
    /// diagnostics, and inventories. It deliberately accepts an unknown event
    /// with a valid declaration shape so diagnostics can report that selector;
    /// executable inventories use [`Self::irules_top_level_declaration`],
    /// which additionally requires the event registry membership.
    #[must_use]
    pub fn irules_top_level_declaration_shape(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let args = arguments.values();
        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count)
            || self.irules_command_placement(name, crate::events::IrulesExecutionContext::TopLevel)
                != crate::events::IrulesCommandPlacement::Allowed
        {
            return None;
        }
        if let Some(effect) = self.irules_top_level_effect(name, args) {
            return Some(effect);
        }
        let bodies = self.arg_indices_for_role(name, args, crate::ArgRole::Body);
        let [body_index] = bodies.as_slice() else {
            return None;
        };
        if !arguments.is_braced_literal(*body_index) {
            return None;
        }
        if spec.traits.contains(crate::Traits::IS_EVENT_HANDLER) {
            return self.irules_event_declaration_shape(arguments);
        }
        if spec.traits.contains(crate::Traits::DEFINES_PROCEDURE) {
            let names = self.arg_indices_for_role(name, args, crate::ArgRole::Name);
            let [name_index] = names.as_slice() else {
                return None;
            };
            return Some(Declaration::Procedure {
                name_index: *name_index,
                body_index: *body_index,
            });
        }
        None
    }

    /// Parse a stateful iRules file-level declaration through its spec-owned
    /// effect descriptor.
    #[must_use]
    pub fn irules_top_level_effect(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count) {
            return None;
        }
        match spec.irules_top_level_effect {
            Some(crate::events::IrulesTopLevelEffect::Priority) => {
                let [value] = args else { return None };
                let value = value.parse::<u16>().ok()?;
                crate::events::BIGIP_EVENT_HANDLER_PRIORITY
                    .accepts(value)
                    .then_some(Declaration::Priority { value })
            }
            Some(crate::events::IrulesTopLevelEffect::Timing) => {
                let [value] = args else { return None };
                if !crate::events::IRULES_TIMING_SPELLINGS.contains(value) {
                    return None;
                }
                let enabled = matches!(*value, "on" | "enable");
                Some(Declaration::Timing { enabled })
            }
            None => None,
        }
    }

    /// Validate the shared iRules event-handler argument grammar independent
    /// of its resolved registry spelling.
    #[must_use]
    pub fn irules_event_declaration(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        let declaration = self.irules_event_declaration_shape(arguments)?;
        let crate::events::IrulesTopLevelDeclaration::Event { event, .. } = &declaration else {
            return None;
        };
        events.is_known(event).then_some(declaration)
    }

    /// Validate event-handler layout without requiring the event name to be
    /// known. Diagnostic consumers use this to report an otherwise valid
    /// top-level declaration whose event selector is unknown.
    #[must_use]
    pub fn irules_event_declaration_shape(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        self.read_irules_event_declaration(arguments)?.ok()
    }

    /// Read the event declaration, preserving the argument index of a rejected
    /// priority. Other incomplete or malformed declarations return `None`.
    ///
    /// # Errors
    /// The error is the index of an explicit priority outside the policy.
    #[must_use]
    pub fn read_irules_event_declaration(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<Result<crate::events::IrulesTopLevelDeclaration, usize>> {
        use crate::events::{
            IRULES_PRIORITY_CLAUSE as PRIORITY, IRULES_TIMING_CLAUSE as TIMING,
            IRULES_TIMING_SPELLINGS, IrulesTopLevelDeclaration as Declaration,
        };
        let args = arguments.values();
        let (event, body_index, priority_index) = match args {
            [event, _] => (event, 1, None),
            [event, PRIORITY, _, _] => (event, 3, Some(2)),
            [event, TIMING, value, _] if IRULES_TIMING_SPELLINGS.contains(value) => {
                (event, 3, None)
            }
            [event, PRIORITY, _, TIMING, timing, _] if IRULES_TIMING_SPELLINGS.contains(timing) => {
                (event, 5, Some(2))
            }
            [event, TIMING, timing, PRIORITY, _, _] if IRULES_TIMING_SPELLINGS.contains(timing) => {
                (event, 5, Some(4))
            }
            _ => return None,
        };
        if !arguments.is_braced_literal(body_index) {
            return None;
        }
        let priority = match priority_index {
            Some(index) => match Self::irules_event_priority(args[index]) {
                Some(priority) => Some(priority),
                None => return Some(Err(index)),
            },
            None => None,
        };
        Some(Ok(Declaration::Event {
            event: event.to_uppercase(),
            body_index,
            priority,
        }))
    }

    /// A `priority` clause value the BIG-IP handler priority policy accepts.
    fn irules_event_priority(value: &str) -> Option<u16> {
        value
            .parse::<u16>()
            .ok()
            .filter(|priority| crate::events::BIGIP_EVENT_HANDLER_PRIORITY.accepts(*priority))
    }

    /// What an incomplete event declaration of `spec` takes at argument
    /// `active` (0-based), given the words `written` before it (`None` for a
    /// non-literal word) — the layout [`Self::irules_event_declaration_shape`]
    /// validates, read one position at a time: the event, then each optional
    /// clause of [`crate::events::IRULES_EVENT_CLAUSES`] at most once and in
    /// either order, each followed by its value, then the body.
    ///
    /// Clause keywords are the spec's argument-1 values and timing values its
    /// argument-2 values (the position a leading `timing` clause's value
    /// takes), both with their version gates at `package_version`.
    ///
    /// `None` for a spec that is not an event handler, when a word before
    /// `active` is non-literal or missing, or when the written words already
    /// leave the layout. Completion advice only: no body entry, frame or
    /// handler identity follows from it.
    #[must_use]
    pub fn irules_event_declaration_slot(
        &self,
        spec: &CommandSpec,
        written: &[Option<&str>],
        active: usize,
        package_version: Option<&str>,
    ) -> Option<crate::events::IrulesDeclarationSlot> {
        use crate::events::{
            IRULES_EVENT_CLAUSES, IRULES_PRIORITY_CLAUSE, IRULES_TIMING_SPELLINGS,
            IrulesDeclarationSlot as Slot,
        };
        if !spec.traits.contains(Traits::IS_EVENT_HANDLER) {
            return None;
        }
        let words = written
            .get(..active)?
            .iter()
            .copied()
            .collect::<Option<Vec<&str>>>()?;
        if active == 0 {
            return Some(Slot::Event);
        }
        let mut remaining: Vec<&str> = IRULES_EVENT_CLAUSES.to_vec();
        let mut index = 1;
        while index < active {
            let keyword = words[index];
            let at = remaining.iter().position(|clause| *clause == keyword)?;
            let clause = remaining.remove(at);
            if index + 1 == active {
                return Some(if clause == IRULES_PRIORITY_CLAUSE {
                    Slot::Priority
                } else {
                    Slot::Timing(spec.available_arg_values_at(2, package_version))
                });
            }
            let value = words[index + 1];
            let accepted = if clause == IRULES_PRIORITY_CLAUSE {
                Self::irules_event_priority(value).is_some()
            } else {
                IRULES_TIMING_SPELLINGS.contains(&value)
            };
            if !accepted {
                return None;
            }
            index += 2;
        }
        if remaining.is_empty() {
            return Some(Slot::Body);
        }
        Some(Slot::Clause(
            spec.available_arg_values_at(1, package_version)
                .into_iter()
                .filter(|value| remaining.contains(&value.value))
                .collect(),
        ))
    }

    /// Whether `name` should appear as a notable action node in a flow
    /// diagram ([`Traits::DIAGRAM_ACTION`]). Accepts both the bare
    /// (`HTTP::respond`) and the canonical (`::HTTP::respond`) spelling —
    /// the leading `::` stamped on `Statement::Call.canonical_command` by
    /// lowering is stripped to recover the bare registration form — and
    /// reflects the dialects loaded into this registry (the diagram-action
    /// set is part of the per-registry trait index, so a
    /// `--dialect f5-irules` registry recognises iRules actions).
    #[must_use]
    pub fn is_diagram_action(&self, name: &str) -> bool {
        let has = |n: &str| {
            self.by_name.get(n).is_some_and(|specs| {
                specs
                    .iter()
                    .any(|s| s.traits.contains(Traits::DIAGRAM_ACTION))
            })
        };
        has(name) || tcl_syntax::naming::unroot_rooted_key(name).is_some_and(has)
    }

    /// Whether `name` is explicitly marked **never** translatable to F5
    /// Distributed Cloud (XC) — any registered spec whose
    /// [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(false)`. Consumed by the `f5-xc` iRule→XC translator.
    #[must_use]
    pub fn is_xc_never_translatable(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(false)))
    }

    /// Whether `name` is explicitly marked translatable to XC despite an
    /// otherwise-untranslatable namespace prefix — any registered spec
    /// whose [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(true)`.
    #[must_use]
    pub fn is_xc_translatable_override(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(true)))
    }

    /// Whether `name` carries the [`Traits::NOT_PROC_FACTORY`] trait —
    /// a registered command head that incidentally matches the
    /// proc-factory token shape but is not a factory wrapper.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_not_proc_factory(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::NOT_PROC_FACTORY))
        })
    }

    /// Whether `name` is an iRules side-switch — `clientside`,
    /// `serverside`, or `peer` — i.e. a command that evaluates its
    /// nesting-script body under a different connection-side context
    /// than the surrounding event ([`Traits::IS_SIDE_SWITCH`]).
    /// Consulted by the iRules collect/release/payload flow check when
    /// it descends into a side-switch body.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_side_switch(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::IS_SIDE_SWITCH))
        })
    }

    /// Return the side context selected by `name`'s nesting-script body.
    ///
    /// This is more precise than [`Self::is_side_switch`]: callers that need
    /// to recurse into a body use the declared fixed/client/server/peer
    /// behaviour instead of matching a command spelling.
    #[must_use]
    pub fn side_switch_target(&self, name: &str) -> Option<SideSwitchTarget> {
        self.get(name)?.side_switch_target
    }

    /// Return the data-collection lifecycle fact declared by `name`.
    ///
    /// This is the only lookup consumers need for collect/release/payload
    /// analysis: command files opt in explicitly, so a similarly named custom
    /// command is never mistaken for an iRules buffer operation.
    #[must_use]
    pub fn data_collection_operation(&self, name: &str) -> Option<DataCollectionOperation> {
        self.get(name)?.data_collection
    }

    /// Return the declared protocol facts for `protocol`.
    ///
    /// A protocol can be known even when it has no `collect` command (UDP and
    /// ASM payloads are immediately available), so this scans all declared
    /// lifecycle operations rather than only collect commands.
    #[must_use]
    pub fn data_collection_protocol(&self, protocol: &str) -> Option<DataCollectionProtocol> {
        self.query_surfaces()
            .data_collection
            .iter()
            .filter_map(|spec| spec.data_collection)
            .find(|operation| operation.protocol.name.eq_ignore_ascii_case(protocol))
            .map(|operation| operation.protocol)
    }

    /// Return the registered collect command for `protocol`, if it has one.
    ///
    /// Editor quick fixes use this to offer only commands that really exist;
    /// for example, UDP payload access deliberately returns `None` because
    /// BIG-IP provides each datagram without a `UDP::collect` command.
    #[must_use]
    pub fn data_collection_collect_command(&self, protocol: &str) -> Option<&'static CommandSpec> {
        self.query_surfaces()
            .data_collection
            .iter()
            .copied()
            .find(|spec| {
                spec.data_collection.is_some_and(|operation| {
                    operation.action == DataCollectionAction::Collect
                        && operation.protocol.name.eq_ignore_ascii_case(protocol)
                })
            })
    }

    /// Return the event-handler priority policy declared by `name`.
    ///
    /// BIG-IP's `when` handler has a runtime default priority, so the policy
    /// records that omission is valid. Consumers can still honour a stricter
    /// policy for a dialect whose handler spec opts into one.
    #[must_use]
    pub fn event_handler_priority(&self, name: &str) -> Option<EventHandlerPriority> {
        self.get(name)?.event_handler_priority
    }

    /// Return the command spec that declares an event-handler priority grammar.
    ///
    /// Code generators and editor fixes use this when they need to create a
    /// handler rather than inspect an existing command. This keeps both the
    /// handler spelling and its priority grammar in the registry. Returns
    /// `None` if the dialect has no handler grammar or more than one, because
    /// choosing between multiple handlers requires additional registry data.
    #[must_use]
    pub fn event_handler_spec(&self) -> Option<&CommandSpec> {
        let mut handlers = self
            .by_name
            .values()
            .flatten()
            .filter(|spec| spec.event_handler_priority.is_some());
        let handler = handlers.next()?;
        handlers.next().is_none().then_some(handler)
    }
}
