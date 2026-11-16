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

//! The selected dialect's command surface: the builtins and bound commands a release exposes, the TclOO root gate, and a namespace's visible names.

use super::super::Interp;
use super::Command;
use crate::namespace::{NsId, GLOBAL};
use tcl_core_types::OoId;

impl Interp {
    /// Whether a builtin command is exposed on this interpreter's selected
    /// runtime surface. The registry recognises versioned builtin entries;
    /// unrecognised names remain available for user-defined commands.
    ///
    /// Two registry-backed cases: the math-function surface
    /// (`::tcl::mathfunc::*` vs the 8.4 fixed table), and release
    /// availability — a builtin whose registry spec the pinned profile's
    /// availability mask does not admit (`lassign` at 8.4, `lpop` before
    /// 9.0) resolves like C Tcl, to `invalid command name`. Only
    /// registry-known builtins are gated: user procs and names the registry
    /// does not know remain callable, so a polyfill proc shadowing a hidden
    /// builtin keeps working.
    pub(crate) fn builtin_command_visible_for_surface(&self, name: &[u8]) -> bool {
        let surface = (
            self.runtime_version(),
            std::ptr::from_ref(self.dialect_profile()).addr(),
            self.0
                .profile_registry
                .get()
                .map_or(0, |registry| std::ptr::from_ref(registry).addr()),
        );
        {
            let memo = self.0.builtin_surface_visibility.borrow();
            if memo.surface == Some(surface) {
                if let Some(&visible) = memo.names.get(name) {
                    return visible;
                }
            }
        }
        let visible = self.builtin_command_visible_for_surface_uncached(name);
        let mut memo = self.0.builtin_surface_visibility.borrow_mut();
        if memo.surface != Some(surface) {
            memo.surface = Some(surface);
            memo.names.clear();
        }
        memo.names.insert(name.to_vec(), visible);
        visible
    }

    fn builtin_command_visible_for_surface_uncached(&self, name: &[u8]) -> bool {
        core::str::from_utf8(name).map_or(true, |name| {
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version())
                .permits_builtin_math_function_command(name)
                && self.profile_admits_registry_builtin(name)
        })
    }

    /// Record `owner` as an engine-installed TclOO root object command. The
    /// registry spelling is retained only as the dialect availability key;
    /// command rename changes the display projection, not this identity.
    pub(crate) fn declare_registry_object_root(&self, owner: OoId, registry_name: &[u8]) {
        self.0
            .registry_object_roots
            .borrow_mut()
            .insert(owner, registry_name.to_vec());
    }

    /// Drop an engine-installed root marking with the OO identity it described.
    pub(crate) fn forget_registry_object_root(&self, owner: OoId) {
        self.0.registry_object_roots.borrow_mut().remove(&owner);
    }

    /// Whether `fqn` is an engine-installed TclOO root that this release does
    /// **not** have (e.g. `::oo::configurable` on an 8.6 surface). Such a root
    /// is invisible to every dispatch and enumeration path, so for anything
    /// that asks "is this name taken?" it must read as free — real tclsh 8.6
    /// has no `::oo::configurable`, and a script may define one.
    pub(crate) fn is_gate_hidden_object_root(&self, fqn: &[u8]) -> bool {
        let owner = match self.0.namespaces.borrow().resolve(GLOBAL, fqn) {
            Some(Command::OoObject(owner)) => owner,
            _ => return false,
        };
        self.0
            .registry_object_roots
            .borrow()
            .get(&owner)
            .is_some_and(|registry_name| !self.builtin_command_visible_for_surface(registry_name))
    }

    /// Retire the engine root hidden at `fqn` by the selected dialect before a
    /// script claims that spelling. Its OO lifecycle must finish before the
    /// replacement creates or adopts the same instance namespace.
    pub(crate) fn retire_gate_hidden_object_root(&mut self, fqn: &[u8]) -> bool {
        let owner = match self.0.namespaces.borrow().resolve(GLOBAL, fqn) {
            Some(Command::OoObject(owner)) => owner,
            _ => return false,
        };
        let hidden = self
            .0
            .registry_object_roots
            .borrow()
            .get(&owner)
            .is_some_and(|registry_name| !self.builtin_command_visible_for_surface(registry_name));
        if hidden {
            self.oo_command_renamed(owner, None);
        }
        hidden
    }

    /// Retire a hidden stock OO root only at the exact selected publication slot.
    pub(crate) fn retire_gate_hidden_object_in_slot(&mut self, ns: NsId, tail: &[u8]) {
        let owner = self.namespaces().command_in(ns, tail).and_then(|command| {
            let owner = command.oo_object()?;
            self.0
                .registry_object_roots
                .borrow()
                .get(&owner)
                .is_some_and(|name| !self.builtin_command_visible_for_surface(name))
                .then_some(owner)
        });
        if let Some(owner) = owner {
            self.oo_command_renamed(owner, None);
        }
    }

    /// The availability half of [`Self::builtin_command_visible_for_surface`]:
    /// whether the pinned profile admits registry builtin `name`. Names the
    /// registry does not know are always admitted — they may be engine
    /// extensions the registry has no spec for.
    fn profile_admits_registry_builtin(&self, name: &str) -> bool {
        let Some(registry) = self.0.profile_registry.get() else {
            return true; // the permissive fallback profile gates nothing
        };
        if let Some(admitted) = tcl_registry::CommandRegistry::native_stock_registration_admission(
            tcl_registry::InvocationDialect::of_profile(self.dialect_profile()),
            name,
        ) {
            return admitted;
        }
        registry.get(name).is_none()
            || registry
                .get_for_surface(name, self.0.dialect_point.get())
                .is_some()
    }

    /// Apply the selected dialect's command-surface gate to an already-bound
    /// command token. Engine builtins retain their registry identity by exact
    /// generation across rename and hide; other native commands use their
    /// current Tcl-facing spelling as before.
    pub(in super::super) fn command_visible_for_surface_at(
        &self,
        command: &Command,
        generation: Option<u64>,
    ) -> bool {
        match command {
            Command::Builtin(_) => {
                if generation.is_some_and(|token| self.is_stock_scripted_worker(token)) {
                    return true;
                }
                let registry_name = generation.and_then(|generation| {
                    self.0
                        .registry_builtin_names
                        .borrow()
                        .get(&generation)
                        .cloned()
                });
                registry_name
                    .as_deref()
                    .is_none_or(|name| self.builtin_command_visible_for_surface(name))
            }
            Command::Ensemble(_) => generation
                .and_then(|generation| self.stock_ensemble_registry_identity(generation))
                .is_none_or(|name| self.builtin_command_visible_for_surface(&name)),
            Command::OoObject(id) => self
                .0
                .registry_object_roots
                .borrow()
                .get(id)
                .is_none_or(|name| self.builtin_command_visible_for_surface(name)),
            _ => true,
        }
    }

    /// Observe the selected surface at an existing slot without reinterpreting
    /// its namespace components or recovering identity from a reporting name.
    pub(crate) fn has_visible_command_in_slot(&self, namespace: NsId, simple: &[u8]) -> bool {
        let namespaces = self.namespaces();
        namespaces
            .command_in(namespace, simple)
            .is_some_and(|command| {
                self.command_visible_for_surface_at(
                    &command,
                    namespaces.command_generation(namespace, simple),
                )
            })
    }

    /// Resolve `name` (from namespace `origin`) to a command handle, applying
    /// the availability gate of [`Self::builtin_command_visible_for_surface`]
    /// to the **final** resolved builtin identity: a builtin the emulated
    /// release does not carry resolves to `None`, exactly as if no command of
    /// that name existed.
    ///
    /// This is the single owner of the gate on the dispatch side. Without it,
    /// a resolve-then-[`Self::invoke`] shape — the alias trampoline
    /// ([`Self::dispatch_alias`]) or the `namespace import` redirect
    /// ([`Command::Imported`]) — could reach the builtin behind an ungated
    /// second resolution, making a release-hidden builtin callable through an
    /// alias or an imported spelling. Every name→`Command` step that feeds
    /// `invoke` goes through here, so a new dispatch path cannot silently
    /// reopen that hole.
    ///
    /// A gated miss is deliberately indistinguishable from a deleted command:
    /// each caller then reports the miss the way it already reports a target
    /// that genuinely does not exist (`invalid command name "<target>"`,
    /// naming the resolved target rather than the alias — matching real tclsh
    /// 8.6/9.0 for `interp alias {} la {} nosuchcmd; la`), which is precisely
    /// the "this release does not have that command" contract this gate
    /// implements.
    pub(crate) fn resolve_dispatchable(&self, origin: NsId, name: &[u8]) -> Option<Command> {
        self.resolve_dispatchable_with_generation(origin, name)
            .map(|(command, _)| command)
    }

    /// Resolve an invocation and retain the selected token generation together.
    /// Native implementation identity must not be reconstructed from display argv.
    pub(crate) fn resolve_dispatchable_with_generation(
        &self,
        origin: NsId,
        name: &[u8],
    ) -> Option<(Command, Option<u64>)> {
        let (command, generation) = {
            let ns = self.namespaces.borrow();
            (
                ns.resolve(origin, name)?,
                ns.resolve_generation(origin, name),
            )
        };
        self.command_visible_for_surface_at(&command, generation)
            .then_some((command, generation))
    }

    /// Command names in the current namespace, filtered through the selected
    /// runtime surface (`info commands`).
    #[must_use]
    pub fn command_names(&self) -> Vec<Vec<u8>> {
        self.visible_command_names_in(self.current_ns.get())
    }

    /// The directly-bound command names that the selected runtime surface
    /// permits callers to enumerate. This is the common filter behind the
    /// `info commands` adapter and internal namespace listings.
    pub(crate) fn visible_command_names_in(&self, id: NsId) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        ns.command_names(id)
            .iter()
            .filter_map(|name| {
                // Mirror `resolve_dispatchable`'s gate exactly: a command the
                // release does not have must not be *listed* either, or
                // `info commands ::oo::*` on an 8.4 surface advertises names
                // that then fail to dispatch. The TclOO roots the engine
                // installs on the registry's behalf are gated alongside
                // builtins; every script-created object stays invariant. Use
                // the exact generation too, so a renamed builtin is gated by
                // the same stable registry identity as dispatch.
                let command = ns.command_in(id, name)?;
                self.command_visible_for_surface_at(&command, ns.command_generation(id, name))
                    .then(|| name.to_vec())
            })
            .collect()
    }

    /// Actual original Jim table reports, filtered using their independent comparison slots.
    pub(crate) fn visible_command_report_names_in(&self, id: NsId) -> Vec<Vec<u8>> {
        let names = self.visible_command_names_in(id);
        let namespaces = self.namespaces();
        names
            .iter()
            .map(|name| namespaces.command_table_report_name(id, name).to_vec())
            .collect()
    }

    /// Report original table keys only for actual visible Jim alias bindings.
    pub(crate) fn visible_alias_report_names_in(
        &self,
        id: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim alias inventory issuer",
            ));
        }
        let names = self.visible_command_names_in(id);
        let namespaces = self.namespaces();
        Ok(names
            .iter()
            .filter(|name| matches!(namespaces.command_in(id, name), Some(Command::Alias { .. })))
            .map(|name| namespaces.command_table_report_name(id, name).to_vec())
            .collect())
    }

    /// The unqualified names of `id`'s commands that `namespace import`
    /// created — C's `NamespaceImportCmd` introspection form (`objc == 1`),
    /// which walks the namespace's `cmdTable` for entries whose `deleteProc`
    /// is `DeleteImportedCmd`. Sorted: C yields hash order, which is not
    /// reproducible, and the VM sorts for the same reason.
    pub(crate) fn imported_command_tails(&self, id: NsId) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let mut names: Vec<Vec<u8>> = ns
            .command_names(id)
            .iter()
            .filter(|name| matches!(ns.resolve(id, name), Some(Command::Imported { .. })))
            .map(|name| name.to_vec())
            .collect();
        names.sort();
        names
    }
}
