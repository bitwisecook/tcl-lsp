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

//! The command surface: the profile governing which builtins are available, and whether a builtin is visible under it.

use super::super::commands::CommandSidecarKey;
use super::super::{InterpState, Vm};
use crate::command::Command;

impl Vm {
    /// Override only the builtin command-availability surface.
    ///
    /// Compilation, bytecode profile validation, lexer/expr semantics, and
    /// [`Self::dialect_profile`] remain unchanged. This is for embedding hosts
    /// that execute a sandboxed dialect inside a broader host interpreter.
    /// A named override is rejected when its Tcl runtime base is older than
    /// the compilation dialect's base, or when it hides a bytecode-compiled
    /// command available to that dialect. The permissive fallback is the
    /// deliberate universal-surface exception: it never hides a builtin, even
    /// though its inert runtime base remains Tcl 9.0. Otherwise a specialised
    /// opcode could execute a command that ordinary lookup hides. Returns
    /// whether the requested surface was installed.
    #[must_use]
    pub fn set_command_surface_profile(
        &mut self,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        // `plain_tcl` deliberately gates no commands, so it is compatible
        // with every dialect independently of its fallback runtime base.
        // Keep the runtime and visibility checks coupled for named profiles:
        // a named surface must still be new enough *and* expose each compiled
        // command from the execution dialect.
        if !profile.is_fallback()
            && (profile.vm_runtime_version < self.pin.profile.vm_runtime_version
                || !Self::command_surface_covers_compiled_commands(self.pin.profile, profile))
        {
            return false;
        }
        if !std::ptr::eq(self.command_surface_profile, profile) {
            self.advance_native_operation_context();
        }
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        self.command_surface_profile = profile;
        self.command_surface_point = Some(crate::environment::surface_point(profile));
        self.profile_registry =
            (!profile.is_fallback()).then(|| crate::environment::store_for_profile(profile));
        true
    }

    /// Whether `surface` exposes every registry command whose source-profile
    /// bytecode may bypass normal command lookup.  Runtime-version ordering is
    /// necessary but not sufficient: profiles on the same Tcl release can
    /// carry disjoint availability masks (for example BPF versus BIG-IP).
    ///
    /// `Traits::BYTE_COMPILED` is the registry-owned declaration that a
    /// literal command head may be lowered to bytecode.  Checking its actual
    /// source and target profile visibility preserves compatible cross-profile
    /// hosts such as iRules over a plain Tcl 8.4 host without allowing a
    /// narrowed same-release surface to hide an intrinsic.
    fn command_surface_covers_compiled_commands(
        dialect: &'static tcl_dialect::DialectProfile,
        surface: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if surface.is_fallback() {
            return true;
        }
        // Both halves are per-environment generations: the
        // store is the generation's, and the mask each side is checked under
        // is its environment's document authoring mask, which the ingress seam
        // pins equal to the profile point this read used.
        let compiled_registry = crate::environment::store_for_profile(dialect);
        let surface_registry = crate::environment::store_for_profile(surface);
        let compiled_mask = Some(crate::environment::surface_point(dialect));
        let surface_point = Some(crate::environment::surface_point(surface));
        compiled_registry.command_names().all(|name| {
            let Some(spec) = compiled_registry.get_for_surface(name, compiled_mask) else {
                return true;
            };
            !spec.traits.contains(tcl_registry::Traits::BYTE_COMPILED)
                || surface_registry
                    .get_for_surface(name, surface_point)
                    .is_some()
        })
    }

    /// The profile currently governing builtin command availability.
    #[must_use]
    pub fn command_surface_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.command_surface_profile
    }
}

impl InterpState {
    /// Release/dialect visibility belongs to the interpreter state because
    /// command candidate traversal happens here, including for parked child
    /// interpreters.  Filtering after resolution is too late: an unavailable
    /// namespace-local builtin must not shadow a later path/global candidate.
    /// The registry owns the versioned filters; procedures and aliases are
    /// invariant, while registered builtin/native handlers retain a stable
    /// identity through import, rename, hide, and expose.
    pub(in super::super) fn builtin_command_visible_for_surface(
        &self,
        name: &str,
        command: &Command,
    ) -> bool {
        self.builtin_command_visible_for_identity(name, command, None)
    }

    pub(in super::super) fn builtin_command_visible_for_identity(
        &self,
        name: &str,
        command: &Command,
        identity: Option<&str>,
    ) -> bool {
        // `Command::Object` joins the gated kinds for the TclOO **roots** only.
        // `oo::object`/`oo::class`/`oo::configurable` are command-table entries
        // the engine installs on behalf of the registry (which dates them
        // TCL86_PLUS / TCL90_PLUS), so an 8.4 or 8.5 surface must not carry
        // them and an 8.6 surface must not carry `oo::configurable`. Every
        // *other* object command is script-created — `oo::class create lpop`
        // is a user command that happens to share a registry name — so it is
        // invariant, exactly like a proc.
        //
        // A root's registry identity is recorded with it, so a renamed root
        // keeps being dated by the name the registry knows rather than by the
        // name it now answers to.
        let root_identity = match command {
            Command::Builtin(_) | Command::Native(_) => None,
            Command::Object(_) => match self.registry_object_roots.get(name) {
                Some(identity) => Some(identity.clone()),
                None => return true,
            },
            _ => return true,
        };
        let origin = root_identity
            .or_else(|| identity.map(str::to_owned))
            .or_else(|| self.builtin_identities.get(name).cloned())
            .or_else(|| {
                let mut current = CommandSidecarKey::visible(name);
                for _ in 0..=self.imported_commands.len() + self.hidden.imports.len() {
                    let next = match &current {
                        CommandSidecarKey::Visible(visible) => self.imported_commands.get(visible),
                        CommandSidecarKey::Hidden(hidden) => self.hidden.imports.get(hidden),
                    };
                    let Some(next) = next else {
                        break;
                    };
                    current = next.origin.key.clone();
                }
                match current {
                    CommandSidecarKey::Visible(key) => self.builtin_identities.get(&key).cloned(),
                    CommandSidecarKey::Hidden(key) => {
                        self.hidden.builtin_identities.get(&key).cloned()
                    }
                }
            });
        // A host replacement has no stock release floor just because its
        // spelling happens to name a command introduced in a later C release.
        let Some(origin) = origin else {
            return true;
        };
        if !tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.native_execution_profile(),
        )
        .permits_builtin_math_function_command(&origin)
        {
            return false;
        }
        matches!(command, Command::Native(_)) || self.profile_admits_registry_builtin(&origin)
    }

    fn profile_admits_registry_builtin(&self, name: &str) -> bool {
        let Some(registry) = self.profile_registry else {
            return true;
        };
        if let Some(admitted) = tcl_registry::CommandRegistry::native_stock_registration_admission(
            tcl_native_model::InvocationDialect::of_profile(self.command_surface_profile),
            name,
        ) {
            return admitted;
        }
        registry.get(name).is_none()
            || registry
                .get_for_surface(name, self.command_surface_point)
                .is_some()
    }
}
