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

//! The sandbox an embedder that hosts this interpreter as an engine builds a
//! body into: the commands it may call, and the release it runs at. The rules
//! are the bytecode VM's engine's, so a body sees the same surface on either
//! engine, and they live here rather than with the native engine so a host
//! that drives this runtime compiled to `wasm32` applies them too.

use crate::interp::Interp;

impl Interp {
    /// Select explicit Unicode names in the original global command table.
    /// Stock dependencies come from installed generations, independently of
    /// their current spelling. Host callbacks retain the first typed refusal.
    pub fn restrict_to(
        &mut self,
        allowed: &[&str],
        kept: &[String],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.admit_restriction_purpose()?;
        let kept = kept
            .iter()
            .filter_map(|name| {
                self.namespaces()
                    .resolve_generation(crate::namespace::GLOBAL, name.as_bytes())
            })
            .collect::<Vec<_>>();
        self.restrict_to_tokens(allowed, &kept)
    }

    /// Retain actual host/unit generations and authenticated stock dependencies.
    /// Displayed names never donate an implementation identity to a replacement.
    pub(crate) fn restrict_to_tokens(
        &mut self,
        allowed: &[&str],
        kept: &[u64],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.admit_restriction_purpose()?;
        let mut roots = kept.to_vec();
        for name in allowed {
            if let Some(generation) = self
                .namespaces()
                .resolve_generation(crate::namespace::GLOBAL, name.as_bytes())
            {
                if !roots.contains(&generation) {
                    roots.push(generation);
                }
            }
        }
        let retained = self.stock_implementation_generations(&roots);
        self.retain_command_tokens(&|generation, _| retained.contains(&generation))
    }

    /// Admit the authentic selected naming purpose before table or getter access.
    pub(crate) fn admit_restriction_purpose(
        &mut self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        if let Some(cause) = self.native_execution_refusal() {
            return Err(cause);
        }
        if self.name_policy_protocol().is_none() {
            let cause =
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "engine command restriction naming purpose",
                );
            self.refuse_native_access(cause);
            return Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                cause,
            ));
        }
        Ok(())
    }

    /// Decode the complete original list through checked actual getters before
    /// any restriction effects. An opaque Unicode adapter input is a host refusal.
    pub(crate) fn restriction_unicode_names(
        &mut self,
        original: *mut crate::obj::TclObj,
    ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueOps;
        if let Some(cause) = self.native_access_refusal() {
            return Err(cause.into());
        }
        if self.host_refusal_pending() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "earlier engine restriction host refusal",
            ));
        }
        if original.is_null() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original engine restriction list",
            ));
        }
        ValueOps::list_elements(self, &original)?
            .into_iter()
            .map(|value| {
                let bytes = ValueOps::native_string_bytes(self, &value)?;
                core::str::from_utf8(&bytes)
                    .map(str::to_owned)
                    .map_err(|error| {
                        tcl_syntax::raw_string::UnicodeAccessError {
                            valid_up_to: error.valid_up_to(),
                            error_len: error.error_len(),
                        }
                        .into()
                    })
            })
            .collect()
    }

    /// Retain the original getter cause outside the Tcl completion channel.
    pub(crate) fn refuse_restriction_input(&mut self, error: tcl_syntax::value::ValueError) {
        if let Some(cause) = error.native_access_refusal() {
            self.refuse_native_access(cause);
        } else {
            self.refuse_host_command(format!("engine restriction input failed: {error}"));
        }
    }

    /// Pin the interpreter to the profile `profile` names ([`release_profile`])
    /// and answer its canonical name; `None`, and nothing pinned, for a name
    /// that is no release the runtime runs.
    pub fn pin_release(&mut self, profile: &str) -> Option<&'static str> {
        let resolved = release_profile(profile)?;
        self.set_dialect_profile(resolved);
        Some(resolved.name)
    }
}

/// The catalogue profile `profile` names, resolved through the one dialect
/// ingress, when it is a release the runtime runs: `None` for a name that
/// resolves to no catalogue profile or to one with no Tcl release under it
/// (the lenient sink, `tk`, a vendor configuration surface).
#[must_use]
pub fn release_profile(profile: &str) -> Option<&'static tcl_dialect::DialectProfile> {
    tcl_registry::model::resolve_known_environment(profile)
        .and_then(|environment| environment.catalogue_profile())
        .filter(|resolved| resolved.runtime_base.is_some())
}

#[cfg(test)]
mod native_restriction_tests;
