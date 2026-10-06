// SPDX-License-Identifier: AGPL-3.0-or-later
//! Physical C namespace-name cache recipes, separate from written name lookup.

use tcl_dialect::TclVersion;

/// Native namespace lifecycle, independently of public name-table membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceLifecycle {
    /// Registered namespace with an intact native lifetime.
    Live,
    /// Deletion is deferred by an actual namespace activation.
    Dying,
    /// Native teardown has invalidated the namespace incarnation.
    Dead,
}

/// Physical primary produced by `namespace current`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceCurrentPrimary {
    /// Tcl 8.4 appends the native full name into a String object.
    String,
    /// Tcl 8.5 and 8.6 construct ordinary string storage without a primary.
    UntypedString,
    /// Tcl 9 constructs an actual resolved namespace-name primary.
    NamespaceName,
}

/// Actual namespace command result factory, independent of name lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceObjectProducer {
    /// The current-namespace command or its introspection opcode.
    Current,
    /// A selected physical parent namespace.
    Parent,
    /// A selected child namespace enumeration member.
    Child,
    /// A selected namespace-path enumeration member.
    Path,
    /// The original namespace-context member of a namespace-code result.
    CodeContext,
    /// The private namespace of an actually selected `TclOO` object.
    ObjectNamespace,
}

/// Pure native C recipe; this value alone authenticates no engine or object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeNamespaceNameRecipe(TclVersion);

impl NativeNamespaceNameRecipe {
    /// Select pure release behaviour without granting live cache authority.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self(version)
    }

    /// Original C release of this recipe.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.0
    }

    /// Whether the native resolved cache permits this actual lifecycle state.
    #[must_use]
    pub const fn permits_target(self, state: NativeNamespaceLifecycle) -> bool {
        match state {
            NativeNamespaceLifecycle::Live => true,
            NativeNamespaceLifecycle::Dying => matches!(self.0, TclVersion::V8_4),
            NativeNamespaceLifecycle::Dead => false,
        }
    }

    /// Whether an absolute input retains the actual global reference context.
    #[must_use]
    pub const fn absolute_references_global(self) -> bool {
        matches!(self.0, TclVersion::V8_4)
    }

    /// Whether a failed conversion installs an unresolved namespace primary.
    #[must_use]
    pub const fn missing_installs_unresolved(self) -> bool {
        matches!(self.0, TclVersion::V8_4)
    }

    /// Whether the original primary has a native string-update procedure.
    #[must_use]
    pub const fn has_string_updater(self) -> bool {
        matches!(self.0, TclVersion::V8_4)
    }

    /// Independently selected producer class of the actual current namespace.
    #[must_use]
    pub const fn current_primary(self) -> NativeNamespaceCurrentPrimary {
        match self.0 {
            TclVersion::V8_4 => NativeNamespaceCurrentPrimary::String,
            TclVersion::V8_5 | TclVersion::V8_6 => NativeNamespaceCurrentPrimary::UntypedString,
            TclVersion::V9_0 | TclVersion::V9_1 => NativeNamespaceCurrentPrimary::NamespaceName,
        }
    }
    /// Select the actual producer class from the reached namespace lifetime.
    /// Tcl 9 avoids installing nsName on a dying token; dead production is refused.
    #[must_use]
    pub const fn current_primary_for_state(
        self,
        lifecycle: NativeNamespaceLifecycle,
    ) -> Option<NativeNamespaceCurrentPrimary> {
        match lifecycle {
            NativeNamespaceLifecycle::Dead => None,
            NativeNamespaceLifecycle::Dying
                if matches!(self.0, TclVersion::V9_0 | TclVersion::V9_1) =>
            {
                Some(NativeNamespaceCurrentPrimary::UntypedString)
            }
            NativeNamespaceLifecycle::Live | NativeNamespaceLifecycle::Dying => {
                Some(self.current_primary())
            }
        }
    }
    /// Select one actual result factory without borrowing another producer's class.
    #[must_use]
    pub const fn result_primary(
        self,
        producer: NativeNamespaceObjectProducer,
        lifecycle: NativeNamespaceLifecycle,
    ) -> Option<NativeNamespaceCurrentPrimary> {
        if matches!(producer, NativeNamespaceObjectProducer::Current) {
            return self.current_primary_for_state(lifecycle);
        }
        if matches!(lifecycle, NativeNamespaceLifecycle::Dead)
            || (matches!(self.0, TclVersion::V8_4)
                && matches!(producer, NativeNamespaceObjectProducer::Path))
        {
            return None;
        }
        if matches!(self.0, TclVersion::V9_0 | TclVersion::V9_1)
            && matches!(lifecycle, NativeNamespaceLifecycle::Live)
        {
            Some(NativeNamespaceCurrentPrimary::NamespaceName)
        } else {
            Some(NativeNamespaceCurrentPrimary::UntypedString)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeNamespaceCurrentPrimary as Primary, NativeNamespaceLifecycle as State,
        NativeNamespaceNameRecipe as Recipe,
    };
    use tcl_dialect::TclVersion;

    #[test]
    fn actual_release_cache_and_producer_discriminators() {
        let versions = [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ];
        let producers = [
            Primary::String,
            Primary::UntypedString,
            Primary::UntypedString,
            Primary::NamespaceName,
            Primary::NamespaceName,
        ];
        for (index, (version, producer)) in versions.into_iter().zip(producers).enumerate() {
            let recipe = Recipe::for_tcl_version(version);
            assert!(recipe.permits_target(State::Live));
            assert!(!recipe.permits_target(State::Dead));
            assert_eq!(recipe.permits_target(State::Dying), index == 0);
            assert_eq!(recipe.absolute_references_global(), index == 0);
            assert_eq!(recipe.missing_installs_unresolved(), index == 0);
            assert_eq!(recipe.has_string_updater(), index == 0);
            assert_eq!(recipe.current_primary(), producer);
            assert_eq!(recipe.current_primary_for_state(State::Dead), None);
            assert_eq!(
                recipe.current_primary_for_state(State::Dying),
                Some(if index >= 3 {
                    Primary::UntypedString
                } else {
                    producer
                })
            );
            for purpose in [
                super::NativeNamespaceObjectProducer::Parent,
                super::NativeNamespaceObjectProducer::Child,
                super::NativeNamespaceObjectProducer::CodeContext,
            ] {
                assert_eq!(
                    recipe.result_primary(purpose, State::Live),
                    Some(if index >= 3 {
                        Primary::NamespaceName
                    } else {
                        Primary::UntypedString
                    })
                );
                assert_eq!(
                    recipe.result_primary(purpose, State::Dying),
                    Some(Primary::UntypedString)
                );
                assert_eq!(recipe.result_primary(purpose, State::Dead), None);
            }
            assert_eq!(
                recipe
                    .result_primary(super::NativeNamespaceObjectProducer::Path, State::Live)
                    .is_some(),
                index != 0
            );
        }
    }
}
