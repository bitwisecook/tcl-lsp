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

//! How a described command's executable behaviour arrives at run time.
//!
//! A [`RuntimeBacking`] is one fact per command, [`CommandSpec::runtime_backing`]
//! (`docs/design/compiler/registry-consumer-contracts.md` § *Four rungs of
//! codegen meeting `.tclspec`*, rung 4). It says which of four doors the
//! command's behaviour comes through — a shipped builtin, a Tcl body, a command
//! the host registered natively, or nothing at all — so that code generation
//! chooses which identity an artefact records for a call site (a command
//! binding, a procedure binding, a guard, or none) from the *fact*, never from
//! the command's name, and so that a runtime can be asked what it actually
//! loaded and be held to the answer.
//!
//! The default is [`RuntimeBacking::None`]: a spec that declares nothing claims
//! that nothing executes the command in the target runtime, and the drift gate
//! (`cargo xtask command-backing`) holds a runtime that registers such a
//! command to account.
//!
//! [`CommandSpec::runtime_backing`]: crate::CommandSpec::runtime_backing

/// Rung 4's per-command fact: how the command's behaviour reaches the runtime.
///
/// Only a shipped command, or a pack command the loader's tier gate admits it
/// on, ever carries anything but [`RuntimeBacking::None`] in a registry; the
/// take-shipped floor ([`crate::security_floor`]) keeps a shipped command's
/// backing through any override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeBacking {
    /// A shipped builtin, attested by its registry identity.
    ///
    /// `identity` is the spelling the registry knows the builtin by — the
    /// command's own name, with a leading `::` kept where the spec carries one,
    /// which is the identity the VM's builtin table is keyed by.
    ShippedBuiltin {
        /// The builtin's registry identity.
        identity: &'static str,
    },
    /// A Tcl body, with where the body text comes from.
    TclBody {
        /// The source of the body text.
        source: BodySource,
    },
    /// A command the host registered natively — a shimmed C command, an
    /// embedder's own handler — attested by a guard identity and never by a
    /// procedure definition.
    HostNative,
    /// Nothing executes this command in the target runtime.
    #[default]
    None,
}

impl RuntimeBacking {
    /// A shipped builtin known to the registry as `identity`.
    #[must_use]
    pub const fn shipped(identity: &'static str) -> Self {
        Self::ShippedBuiltin { identity }
    }

    /// A Tcl body read from `relative_path` in the package's own installed
    /// source.
    #[must_use]
    pub const fn package_source(relative_path: &'static str) -> Self {
        Self::TclBody {
            source: BodySource::PackageSource { relative_path },
        }
    }

    /// Whether the spec declares no backing at all (also what a spec that
    /// declares nothing carries).
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

/// Where a [`RuntimeBacking::TclBody`]'s body text comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodySource {
    /// A path into the package's own installed source, resolved through the
    /// host filesystem seam. The spec field is a pointer, so a library
    /// upgrade moves the body with it.
    PackageSource {
        /// The path, relative to the package's installed source.
        relative_path: &'static str,
    },
    /// Text carried in the pack.
    ///
    /// A library upgrade then diverges from it silently, so a pack that
    /// declares one is told so at load, and a site that rests on it turns
    /// plain on the first mismatch.
    PackText {
        /// The body text, as the pack wrote it.
        text: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_that_declares_nothing_claims_nothing_executes_it() {
        assert_eq!(RuntimeBacking::default(), RuntimeBacking::None);
        assert!(RuntimeBacking::default().is_none());
        assert!(!RuntimeBacking::shipped("lassign").is_none());
        assert!(!RuntimeBacking::HostNative.is_none());
        assert!(!RuntimeBacking::package_source("init.tcl").is_none());
    }

    #[test]
    fn the_constructors_build_the_variants_they_name() {
        assert_eq!(
            RuntimeBacking::shipped("::tcl::dict::get"),
            RuntimeBacking::ShippedBuiltin {
                identity: "::tcl::dict::get"
            }
        );
        assert_eq!(
            RuntimeBacking::package_source("parray.tcl"),
            RuntimeBacking::TclBody {
                source: BodySource::PackageSource {
                    relative_path: "parray.tcl"
                }
            }
        );
    }
}
