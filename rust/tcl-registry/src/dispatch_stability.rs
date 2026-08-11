// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-owned dependencies for proving Tcl dispatch remains stable.
//!
//! Resolving a literal command against [`crate::CommandRegistry`] identifies
//! its static semantics; it does **not** prove that the live interpreter will
//! dispatch the same behaviour later. Tcl can rename or alias commands, alter
//! namespace lookup, install command traces, change interpreter policy, mutate
//! `TclOO` dispatch, or route a missing command through `unknown`. These typed
//! domains tell common analyses what must be guarded or otherwise proven
//! stable before specialising a resolved invocation.

/// One mutable Tcl domain on which live command dispatch can depend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DispatchDependencyDomain {
    /// The resolved command binding, including rename, deletion, and aliases.
    CommandBinding = 0,
    /// Namespace-relative lookup, imports, exports, and namespace paths.
    NamespaceLookup = 1,
    /// Command execution, rename, deletion, and step traces.
    CommandTraces = 2,
    /// Safe/sub-interpreter visibility, hidden commands, and policy state.
    InterpreterPolicy = 3,
    /// `TclOO` object/class methods, filters, mixins, and forwarding dispatch.
    ObjectDispatch = 4,
    /// Namespace and global `unknown` fallback handling.
    UnknownHandling = 5,
}

impl DispatchDependencyDomain {
    const ALL: [Self; 6] = [
        Self::CommandBinding,
        Self::NamespaceLookup,
        Self::CommandTraces,
        Self::InterpreterPolicy,
        Self::ObjectDispatch,
        Self::UnknownHandling,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A compact set of target-neutral dispatch-stability dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchDependencies(u8);

impl Default for DispatchDependencies {
    fn default() -> Self {
        Self::CONSERVATIVE
    }
}

impl DispatchDependencies {
    /// No refinable, operation-specific dispatch dependency.
    ///
    /// An explicit [`DispatchDependencyComposition::Replace`] using this set
    /// still retains [`Self::BASE`].
    pub const NONE: Self = Self(0);

    /// Dependencies that registry metadata cannot remove.
    ///
    /// A static command specification cannot prove that the live interpreter
    /// binding, namespace lookup environment, traces, or interpreter policy
    /// stayed unchanged. Those proofs must come from common flow analysis or
    /// guards, even for an otherwise precisely described command form.
    pub const BASE: Self = Self::from_domains(&[
        DispatchDependencyDomain::CommandBinding,
        DispatchDependencyDomain::NamespaceLookup,
        DispatchDependencyDomain::CommandTraces,
        DispatchDependencyDomain::InterpreterPolicy,
    ]);

    /// Conservative dependencies for an ordinary resolved Tcl command.
    ///
    /// This additionally includes `TclOO` and `unknown`, since an unstamped
    /// registry command has made no narrower dispatch claim. `unknown` is not
    /// irreducible: once binding and namespace lookup are proven unchanged, an
    /// existing resolved command cannot enter missing-command fallback.
    pub const CONSERVATIVE: Self = Self((1 << DispatchDependencyDomain::ALL.len()) - 1);

    /// Construct a set containing one domain.
    #[must_use]
    pub const fn one(domain: DispatchDependencyDomain) -> Self {
        Self(domain.bit())
    }

    /// Construct a set from a static domain list.
    #[must_use]
    pub const fn from_domains(domains: &[DispatchDependencyDomain]) -> Self {
        let mut bits = 0;
        let mut index = 0;
        while index < domains.len() {
            bits |= domains[index].bit();
            index += 1;
        }
        Self(bits)
    }

    /// Whether this set includes `domain`.
    #[must_use]
    pub const fn contains(self, domain: DispatchDependencyDomain) -> bool {
        self.0 & domain.bit() != 0
    }

    /// Whether no dependency is present.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return the union of two dependency sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Iterate over included domains in stable declaration order.
    pub fn iter(self) -> impl Iterator<Item = DispatchDependencyDomain> {
        DispatchDependencyDomain::ALL
            .into_iter()
            .filter(move |domain| self.contains(*domain))
    }
}

/// How a narrower command descriptor composes with inherited dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DispatchDependencyComposition {
    /// Add dependencies to the inherited set.
    Extend,
    /// Replace inherited dependencies with this exact set.
    Replace,
}

/// Static registry declaration of dispatch-stability dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchDependencyDescriptor {
    /// How this descriptor composes with command/subcommand facts.
    pub composition: DispatchDependencyComposition,
    /// Dependencies contributed or used as the exact replacement.
    pub dependencies: DispatchDependencies,
}

impl DispatchDependencyDescriptor {
    /// Extend inherited dependencies.
    #[must_use]
    pub const fn extend(dependencies: DispatchDependencies) -> Self {
        Self {
            composition: DispatchDependencyComposition::Extend,
            dependencies,
        }
    }

    /// Replace inherited dependencies with an exact refined set.
    #[must_use]
    pub const fn replace(dependencies: DispatchDependencies) -> Self {
        Self {
            composition: DispatchDependencyComposition::Replace,
            dependencies,
        }
    }

    const fn apply(self, inherited: DispatchDependencies) -> DispatchDependencies {
        match self.composition {
            DispatchDependencyComposition::Extend => inherited.union(self.dependencies),
            DispatchDependencyComposition::Replace => self.dependencies,
        }
    }
}

/// Borrow-free command → subcommand → form descriptor chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResolvedDispatchDependencies {
    /// Parent command descriptor.
    pub command: Option<DispatchDependencyDescriptor>,
    /// Resolved subcommand descriptor.
    pub subcommand: Option<DispatchDependencyDescriptor>,
    /// Matched form descriptor.
    pub form: Option<DispatchDependencyDescriptor>,
}

impl ResolvedDispatchDependencies {
    /// Resolve the refinable chain, then restore irreducible Tcl dependencies.
    ///
    /// An unstamped command begins conservatively. A `Replace` descriptor can
    /// refine operation-specific dependencies such as `TclOO` and `unknown`, but
    /// cannot remove [`DispatchDependencies::BASE`]. Consequently, static name
    /// resolution is never mistaken for a live-dispatch proof.
    #[must_use]
    pub const fn resolve(self) -> DispatchDependencies {
        let mut dependencies = DispatchDependencies::CONSERVATIVE;
        if let Some(descriptor) = self.command {
            dependencies = descriptor.apply(dependencies);
        }
        if let Some(descriptor) = self.subcommand {
            dependencies = descriptor.apply(dependencies);
        }
        if let Some(descriptor) = self.form {
            dependencies = descriptor.apply(dependencies);
        }
        dependencies.union(DispatchDependencies::BASE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unstamped_resolution_is_conservative() {
        let resolved = ResolvedDispatchDependencies::default().resolve();
        assert_eq!(DispatchDependencies::default(), resolved);
        assert_eq!(resolved, DispatchDependencies::CONSERVATIVE);
        assert_eq!(resolved.iter().count(), 6);
    }

    #[test]
    fn replacement_refines_and_extension_adds() {
        let object = DispatchDependencies::one(DispatchDependencyDomain::ObjectDispatch);
        let unknown = DispatchDependencies::one(DispatchDependencyDomain::UnknownHandling);
        let resolved = ResolvedDispatchDependencies {
            command: Some(DispatchDependencyDescriptor::replace(object)),
            subcommand: Some(DispatchDependencyDescriptor::extend(unknown)),
            form: None,
        }
        .resolve();
        assert_eq!(resolved, DispatchDependencies::CONSERVATIVE);

        let irreducible = ResolvedDispatchDependencies {
            command: Some(DispatchDependencyDescriptor::replace(
                DispatchDependencies::NONE,
            )),
            subcommand: None,
            form: None,
        }
        .resolve();
        assert_eq!(irreducible, DispatchDependencies::BASE);
    }

    #[test]
    fn form_is_applied_after_subcommand_and_command() {
        let object = DispatchDependencies::one(DispatchDependencyDomain::ObjectDispatch);
        let unknown = DispatchDependencies::one(DispatchDependencyDomain::UnknownHandling);
        let resolved = ResolvedDispatchDependencies {
            command: Some(DispatchDependencyDescriptor::replace(object)),
            subcommand: Some(DispatchDependencyDescriptor::extend(unknown)),
            form: Some(DispatchDependencyDescriptor::replace(object)),
        }
        .resolve();
        assert_eq!(resolved, DispatchDependencies::BASE.union(object));
        assert!(!resolved.contains(DispatchDependencyDomain::UnknownHandling));
    }
}
