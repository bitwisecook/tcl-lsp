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

//! Finding a command by token: generations, interned ids, import chains and queries, `info cmdtype` and the resolved name.

use super::super::{
    cmd_trace_owned_by, CommandGenerationLookup, ImportQueryPurpose, ImportQueryTarget, Interp,
};
use super::Command;
use crate::namespace::{NsId, GLOBAL};
use std::rc::Rc;
use tcl_runtime_api::guard::GuardDomain;

impl Interp {
    /// The generation of the command token `name` resolves to — the identity a
    /// command trace hangs off.
    pub(crate) fn resolve_cmd_token(&self, name: &[u8]) -> Option<u64> {
        self.namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
    }

    /// Resolve one retained command token wherever rename or visibility moves
    /// placed it, distinguishing an unavailable token from a retired one.
    pub(in super::super) fn command_by_generation(
        &self,
        generation: u64,
    ) -> CommandGenerationLookup {
        let Some((fqn, command)) = self.raw_command_location_by_generation(generation) else {
            return CommandGenerationLookup::Missing;
        };
        if !self.command_visible_for_surface_at(&command, Some(generation)) {
            return CommandGenerationLookup::Unavailable;
        }
        CommandGenerationLookup::Found { fqn, command }
    }

    /// Resolve an exact generation and its current projection without applying
    /// dispatch-surface policy.
    pub(in super::super) fn raw_command_location_by_generation(
        &self,
        generation: u64,
    ) -> Option<(Vec<u8>, Command)> {
        if let Some((fqn, command)) = self.namespaces.borrow().command_by_generation(generation) {
            return Some((fqn, command));
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            if binding.generation != generation {
                return None;
            }
            let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
            Some((fqn, binding.command.clone()))
        })
    }

    /// The command half of [`Self::raw_command_location_by_generation`].
    pub(in super::super) fn raw_command_by_generation(&self, generation: u64) -> Option<Command> {
        self.raw_command_location_by_generation(generation)
            .map(|(_, command)| command)
    }

    /// Whether an exact imported-command source chain reaches another command
    /// generation. The import loop gate must not substitute a recreated
    /// same-FQN namespace token for a retained source.
    pub(crate) fn import_chain_contains(&self, source_generation: u64, needle: u64) -> bool {
        let mut generation = source_generation;
        let mut visited = std::collections::BTreeSet::new();
        while visited.insert(generation) {
            if generation == needle {
                return true;
            }
            let Some(Command::Imported {
                source_generation, ..
            }) = self.raw_command_by_generation(generation)
            else {
                return false;
            };
            generation = source_generation;
        }
        false
    }

    /// Select the captured source token. Original origin/object purposes never
    /// replace a retired source using its reported spelling.
    fn import_source_binding(
        &self,
        generation: u64,
        purpose: ImportQueryPurpose,
    ) -> Option<(Vec<u8>, u64, Command)> {
        match purpose {
            ImportQueryPurpose::Introspection => match self.command_by_generation(generation) {
                CommandGenerationLookup::Found { fqn, command } => Some((fqn, generation, command)),
                CommandGenerationLookup::Unavailable | CommandGenerationLookup::Missing => None,
            },
            ImportQueryPurpose::Origin | ImportQueryPurpose::OriginalObject => self
                .raw_command_location_by_generation(generation)
                .map(|(fqn, command)| (fqn, generation, command)),
        }
    }

    /// Resolve an invocation's captured immediate source token with its
    /// availability gate. A recreated reported name never replaces that token.
    pub(in super::super) fn resolve_import_source(&self, generation: u64) -> Option<Command> {
        self.import_source_binding(generation, ImportQueryPurpose::Introspection)
            .map(|(_, _, command)| command)
    }

    /// Select an introspection query's original current-namespace table entry.
    pub(in super::super) fn import_query_binding(
        &self,
        name: &[u8],
    ) -> Option<(Vec<u8>, u64, Command)> {
        let namespaces = self.namespaces.borrow();
        let current = self.current_ns.get();
        Some((
            namespaces.resolve_fqn(current, name)?,
            namespaces.resolve_generation(current, name)?,
            namespaces.resolve(current, name)?,
        ))
    }

    /// Follow actual imported bindings to the query's terminal owner. The
    /// visited-generation set rejects a malformed cycle without truncating a
    /// valid chain. Names remain current projections, never visited identities.
    pub(in super::super) fn resolve_import_query(
        &self,
        (mut fqn, mut generation, mut command): (Vec<u8>, u64, Command),
        purpose: ImportQueryPurpose,
    ) -> Option<ImportQueryTarget> {
        let mut visited = std::collections::BTreeSet::new();
        let mut followed_import = false;
        loop {
            if !visited.insert(generation) {
                return None;
            }
            let Command::Imported {
                source_generation,
                ensemble,
                ..
            } = &command
            else {
                return Some(ImportQueryTarget {
                    fqn,
                    generation,
                    command,
                    retained_ensemble: None,
                    followed_import,
                });
            };
            followed_import = true;
            let ensemble = ensemble.as_ref().filter(|token| !token.is_deleted());
            if purpose == ImportQueryPurpose::Introspection {
                if let Some(ensemble) = ensemble {
                    return Some(ImportQueryTarget {
                        fqn,
                        generation,
                        retained_ensemble: Some(ensemble.clone()),
                        command,
                        followed_import,
                    });
                }
            }
            (fqn, generation, command) = self.import_source_binding(*source_generation, purpose)?;
        }
    }

    /// Drop the command/execution traces on `fqn` that belonged to the token
    /// being deleted — C's "free the whole `cmdPtr->tracePtr` list".
    ///
    /// Generations are minted in binding order, so a trace registered on a
    /// replacement the delete callback bound at this same name compares
    /// greater and survives. An unidentifiable dying token (an unbound or
    /// hidden name) takes the whole list, as it did before tokens were
    /// tracked.
    pub(in super::super) fn remove_cmd_traces_of_token(&mut self, fqn: &[u8], dying: Option<u64>) {
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.cmd_traces.len();
        traces
            .cmd_traces
            .retain(|t| t.name != fqn || !cmd_trace_owned_by(t.token, dying));
        let removed = traces.cmd_traces.len() != old_len;
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Intern an exact command token to a stable, dense raw `CommandId`, minting
    /// one on first sight. Backs `Namespaces::find_command`.
    pub(in super::super) fn intern_cmd(&self, fqn: &[u8], generation: u64) -> u32 {
        let mut a = self.cmd_arena.borrow_mut();
        let key = (fqn.to_vec(), generation);
        if let Some(&id) = a.ids.get(&key) {
            return id;
        }
        let id = u32::try_from(a.commands.len()).expect("command count fits u32");
        a.commands.push(key.clone());
        a.ids.insert(key, id);
        id
    }

    /// Resolve `name` from namespace context `cxt` (through the namespace tree to
    /// the root) to its command's FQN, then intern that to a stable raw
    /// `CommandId`. `None` if it resolves to no command. The `Namespaces::find_command`
    /// engine (`state_traits.rs`), keeping the namespace-table access here.
    pub(crate) fn find_command_id(&self, cxt: NsId, name: &[u8]) -> Option<u32> {
        let (fqn, generation) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.resolve_fqn(cxt, name)?,
                namespaces.resolve_generation(cxt, name)?,
            )
        };
        Some(self.intern_cmd(&fqn, generation))
    }

    /// The exact `(FQN, generation)` an interned raw `CommandId` names.
    pub(crate) fn command_identity(&self, id: u32) -> Option<(Vec<u8>, u64)> {
        self.cmd_arena.borrow().commands.get(id as usize).cloned()
    }

    /// The FQN projection of an interned raw `CommandId`.
    pub(crate) fn command_fqn(&self, id: u32) -> Option<Vec<u8>> {
        let (_, generation) = self.command_identity(id)?;
        self.raw_command_location_by_generation(generation)
            .map(|(fqn, _)| fqn)
    }

    /// The actual terminal binding an interned command was imported from,
    /// interned with its current name and genuine generation. Origin follows
    /// imported bindings independently from introspection's live-ensemble stop.
    /// Nonimports and malformed generation cycles have no imported source.
    pub(crate) fn imported_source_id(&self, id: u32) -> Option<u32> {
        let (_, generation) = self.command_identity(id)?;
        let (fqn, command) = self.raw_command_location_by_generation(generation)?;
        let target =
            self.resolve_import_query((fqn, generation, command), ImportQueryPurpose::Origin)?;
        target
            .followed_import
            .then(|| self.intern_cmd(&target.fqn, target.generation))
    }

    /// Immediate source binding and optional real-ensemble identity for a new
    /// `namespace import`. Import-of-import chains deliberately keep every hop:
    /// replacing or deleting the intermediate command must affect its own
    /// importers before `namespace origin` walks any farther toward the root.
    /// Only a *direct* ensemble source contributes an ensemble token; an
    /// imported ensemble is reached through its intermediate command binding.
    pub(crate) fn import_metadata_in(
        &self,
        source_ns: NsId,
        simple: &[u8],
    ) -> Option<(Vec<u8>, u64, Option<Rc<crate::ensemble::EnsembleToken>>)> {
        let namespaces = self.namespaces.borrow();
        let fqn = namespaces.command_fqn_at(source_ns, simple);
        let source_generation = namespaces.command_generation(source_ns, simple)?;
        let ensemble = match namespaces.command_in(source_ns, simple)? {
            Command::Ensemble(token) => Some(token),
            _ => None,
        };
        Some((fqn, source_generation, ensemble))
    }

    /// Whether `name` resolves to a command in the current namespace.
    pub(crate) fn command_exists(&self, name: &[u8]) -> bool {
        self.namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
            .is_some()
    }

    /// The `info cmdtype` classification of `name`, or `None` if no such command.
    pub(crate) fn cmdtype(&self, name: &[u8]) -> Option<&'static [u8]> {
        let cmd = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)?;
        Some(match cmd {
            // A coroutine resume command registers as a builtin but reports
            // its own cmdType (C's per-command registration).
            Command::Builtin(_) => match self.resolve_cmd_token(name) {
                Some(generation) if self.coros.borrow().contains_key(&generation) => b"coroutine",
                _ => b"native",
            },
            Command::ObjCmd(_) => b"native",
            Command::Proc(_) => b"proc",
            Command::Alias { .. } | Command::ParentAlias { .. } => b"alias",
            Command::Imported { .. } => b"import",
            Command::Ensemble(_) => b"ensemble",
            Command::OoObject(_) => b"object",
            Command::OoMy(_) => b"privateObject",
            Command::OoMyClass(_) => b"privateClass",
            Command::ChildInterp(_) => b"interp",
        })
    }

    /// The `::tcl::mathfunc::*` function names (`info functions`).
    pub(crate) fn mathfunc_names(&self) -> Vec<Vec<u8>> {
        let surface =
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version());
        if !surface.has_math_function_command_table() {
            return surface
                .builtin_math_function_names()
                .into_iter()
                .map(|name| name.as_bytes().to_vec())
                .collect();
        }
        let id = self
            .namespaces
            .borrow()
            .find_namespace(GLOBAL, b"::tcl::mathfunc");
        id.map_or_else(Vec::new, |id| self.visible_command_names_in(id))
    }

    /// The canonical FQN `name` resolves to (full resolution order), or `None`
    /// if no such command — for `trace add|remove|info command|execution`, which
    /// must address the same binding `dispatch` hits and error
    /// `invalid command name` on a miss.
    pub(crate) fn resolve_cmd_fqn(&self, name: &[u8]) -> Option<Vec<u8>> {
        self.namespaces
            .borrow()
            .resolve_fqn(self.current_ns.get(), name)
    }
}

impl tcl_runtime_api::semantic_identity::CommandIdentityTokens for Interp {
    fn interpreter_ref(&self) -> tcl_core_types::InterpreterRef {
        self.native_command_interpreter.interpreter_ref()
    }

    fn command_ref(
        &self,
        command: tcl_runtime_api::CommandId,
    ) -> tcl_core_types::Answer<tcl_core_types::CommandRef> {
        use tcl_core_types::{Answer, NamingAuthority, NamingProvenance, UnknownReason};
        let token = self
            .cmd_arena
            .borrow()
            .commands
            .get(command.0 as usize)
            .map(|(_, token)| *token);
        let Some(token) = token else {
            return Answer::unknown(UnknownReason::UnavailableContext);
        };
        let runtime = self.native_command_interpreter;
        Answer::proven(runtime.command_ref(token)).with_evidence(
            NamingProvenance::Runtime {
                owner: runtime.owner,
                interpreter: runtime.interpreter,
            },
            NamingAuthority::Runtime,
        )
    }
}

#[cfg(test)]
mod semantic_identity_tests {
    use super::*;
    use tcl_runtime_api::{semantic_identity::CommandIdentityTokens, Namespaces, ROOT_NS};

    #[test]
    fn command_refs_survive_rename_and_split_on_recreation() {
        let mut interp = Interp::new();
        assert_eq!(
            interp.eval_str(b"proc p {} {return p}"),
            crate::interp::Code::Ok
        );
        let initial_handle = interp.find_command(ROOT_NS, "p").unwrap();
        let initial = interp.command_ref(initial_handle).into_proven().unwrap();
        assert_eq!(interp.eval_str(b"rename p q"), crate::interp::Code::Ok);
        let moved_handle = interp.find_command(ROOT_NS, "q").unwrap();
        assert_eq!(
            initial,
            interp.command_ref(moved_handle).into_proven().unwrap()
        );
        assert_eq!(
            interp.eval_str(b"proc p {} {return newp}"),
            crate::interp::Code::Ok
        );
        let recreated = interp.find_command(ROOT_NS, "p").unwrap();
        assert_ne!(
            initial,
            interp.command_ref(recreated).into_proven().unwrap()
        );
        assert_eq!(
            interp.eval_str(b"rename q {}; proc q {} {return fresh}"),
            crate::interp::Code::Ok
        );
        let fresh = interp.find_command(ROOT_NS, "q").unwrap();
        assert_ne!(initial, interp.command_ref(fresh).into_proven().unwrap());
        assert_eq!(
            initial,
            interp.command_ref(initial_handle).into_proven().unwrap()
        );
        let mut counts = tcl_core_types::answer::ConformanceCounts::default();
        counts.observe(
            &interp
                .command_ref(moved_handle)
                .map(|reference| reference == initial),
            Some(&true),
        );
        counts.observe(
            &interp
                .command_ref(recreated)
                .map(|reference| reference != initial),
            Some(&true),
        );
        counts.observe(
            &interp
                .command_ref(fresh)
                .map(|reference| reference != initial),
            Some(&true),
        );
        println!("runtime: {counts}");
    }
}
