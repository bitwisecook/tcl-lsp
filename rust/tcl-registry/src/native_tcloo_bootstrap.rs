// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original stock `TclOO` factory/bootstrap roles, separately from live tokens.
//!
//! tclOO.c installs the configurable support class and its definition namespace
//! vectors. `TclOO_Configurable_Constructor` reselects the named support class
//! for each factory call and stores its class pointer in the new class's mixins.
//! These recipes describe that transfer; callers must independently retain the
//! actual selected factory/support allocation and current namespace tables.

use crate::{
    InvocationDialect,
    definer::{DefinitionBodyGrammar, DefinitionReceiver},
};
use tcl_dialect::{TclVersion, model::Family};

/// Whether a bare stock class-definition `self` returns its class name.
/// This pure release recipe grants no definition frame or object identity.
#[must_use]
pub fn class_definition_self_returns_name(dialect: InvocationDialect) -> Option<bool> {
    let version = dialect.tcl_version?;
    (dialect.family() == Some(Family::Tcl) && version >= TclVersion::V8_6)
        .then_some(version >= TclVersion::V9_0)
}

/// The intrinsic constructor selected on the original stock factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeClassFactoryConstructor {
    /// Invoke the optional definition script. C9 also creates the class delegate.
    ClassDefinition,
    /// Resolve/install the configurable support mixin, then invoke the next
    /// class-definition constructor. This is an actual native method, even
    /// though `info class constructor` cannot display a source body for it.
    ConfigurableThenClassDefinition,
}

/// Registry-selected original stock factory recipe, with no live table grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeClassFactoryRecipe {
    constructor: NativeClassFactoryConstructor,
    version: TclVersion,
}

impl NativeClassFactoryRecipe {
    pub(crate) fn select(identity: &str, dialect: InvocationDialect) -> Option<Self> {
        if dialect.family() != Some(Family::Tcl) {
            return None;
        }
        let version = dialect.tcl_version?;
        let constructor = match identity {
            "oo::class" if version >= TclVersion::V8_6 => {
                NativeClassFactoryConstructor::ClassDefinition
            }
            "oo::configurable" if version >= TclVersion::V9_0 => {
                NativeClassFactoryConstructor::ConfigurableThenClassDefinition
            }
            _ => return None,
        };
        Some(Self {
            constructor,
            version,
        })
    }

    /// Actual intrinsic constructor role, independently of source bodies.
    #[must_use]
    pub const fn constructor(self) -> NativeClassFactoryConstructor {
        self.constructor
    }

    /// Exact supported release that selected this bootstrap recipe.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }

    /// The factory's independently selected source declaration grammar.
    #[must_use]
    pub const fn grammar(self) -> &'static DefinitionBodyGrammar {
        match self.constructor {
            NativeClassFactoryConstructor::ClassDefinition => &crate::definer::TCLOO_GRAMMAR,
            NativeClassFactoryConstructor::ConfigurableThenClassDefinition => {
                &crate::definer::TCLOO_CONFIGURABLE_GRAMMAR
            }
        }
    }

    /// Stock fallback definition namespace, independently of stored fields.
    /// A current class may select an explicitly retained different namespace.
    #[must_use]
    pub const fn definition_namespace(self, receiver: DefinitionReceiver) -> &'static str {
        match receiver {
            DefinitionReceiver::Instance => "::oo::define",
            DefinitionReceiver::Class => "::oo::objdefine",
        }
    }

    /// The original factory's independently stored definition namespace.
    /// Configurable factories retain their class namespace before any class is
    /// created; ordinary factories and object-side fields begin unset.
    #[must_use]
    pub const fn stored_factory_definition_namespace(
        self,
        receiver: DefinitionReceiver,
    ) -> Option<&'static str> {
        match (self.support(), receiver) {
            (Some(support), DefinitionReceiver::Instance) => {
                Some(support.definition_namespace(receiver))
            }
            _ => None,
        }
    }

    /// Original named support class that each configurable factory invocation
    /// must resolve. Existing created classes retain the selected allocation.
    #[must_use]
    pub const fn support(self) -> Option<NativeConfigurableSupportRecipe> {
        match self.constructor {
            NativeClassFactoryConstructor::ClassDefinition => None,
            NativeClassFactoryConstructor::ConfigurableThenClassDefinition => {
                Some(NativeConfigurableSupportRecipe {
                    version: self.version,
                })
            }
        }
    }
}

/// Audited initial support-class state; this is not an object/table identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeConfigurableSupportRecipe {
    version: TclVersion,
}

impl NativeConfigurableSupportRecipe {
    /// Exact supported release of this original support recipe.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }

    /// Independently registered named support-class slot.
    #[must_use]
    pub const fn command(self) -> &'static str {
        "::oo::configuresupport::configurable"
    }

    /// Exact source bootstrap creates this class without an own constructor.
    /// Callers must still prove that the selected allocation is unchanged.
    #[must_use]
    pub const fn own_constructor_absent(self) -> bool {
        true
    }

    /// Exact source bootstrap creates this class without an own destructor.
    #[must_use]
    pub const fn own_destructor_absent(self) -> bool {
        true
    }

    /// Initial superclass of the support class. Later topology is independent.
    #[must_use]
    pub const fn superclass(self) -> &'static str {
        "::oo::object"
    }

    /// Persistent script definition namespace selected by this provider.
    #[must_use]
    pub const fn definition_namespace(self, receiver: DefinitionReceiver) -> &'static str {
        match receiver {
            DefinitionReceiver::Instance => "::oo::configuresupport::configurableclass",
            DefinitionReceiver::Class => "::oo::configuresupport::configurableobject",
        }
    }

    /// Ordered bootstrap namespace path, separately from the selected namespace.
    #[must_use]
    pub const fn definition_path(self, receiver: DefinitionReceiver) -> &'static [&'static str] {
        match receiver {
            DefinitionReceiver::Instance => &["::oo::define"],
            DefinitionReceiver::Class => &["::oo::objdefine"],
        }
    }

    /// Fixed registrations present at this audited bootstrap, with no compiler
    /// hook or current implementation claim implied by their names.
    #[must_use]
    pub const fn binding_names(self) -> &'static [&'static str] {
        &[
            "::oo::configuresupport::configurable",
            "::oo::configuresupport::configurableclass::property",
            "::oo::configuresupport::configurableclass::properties",
            "::oo::configuresupport::configurableobject::property",
            "::oo::configuresupport::configurableobject::properties",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_configurable_bootstrap_roles_require_selected_stock_release() {
        // Implementation contract: naming.tcloo.original-configurable-factory-support
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configurable-factory-support.md
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let ordinary = registry.native_class_factory_recipe(
                "::oo::class",
                dialect,
                tcl_dialect::model::InvocationRealm::default(),
            );
            assert_eq!(ordinary.is_some(), version >= TclVersion::V8_6);
            let configurable = registry.native_class_factory_recipe(
                "::oo::configurable",
                dialect,
                tcl_dialect::model::InvocationRealm::default(),
            );
            assert_eq!(configurable.is_some(), version >= TclVersion::V9_0);
            if let Some(recipe) = configurable {
                assert_eq!(
                    recipe.constructor(),
                    NativeClassFactoryConstructor::ConfigurableThenClassDefinition
                );
                let support = recipe.support().unwrap();
                assert!(support.own_constructor_absent() && support.own_destructor_absent());
                assert_eq!(support.superclass(), "::oo::object");
                assert_eq!(
                    support.definition_namespace(DefinitionReceiver::Instance),
                    "::oo::configuresupport::configurableclass"
                );
                assert_eq!(
                    support.definition_path(DefinitionReceiver::Instance),
                    &["::oo::define"]
                );
                assert_eq!(
                    support.definition_namespace(DefinitionReceiver::Class),
                    "::oo::configuresupport::configurableobject"
                );
                assert_eq!(
                    support.definition_path(DefinitionReceiver::Class),
                    &["::oo::objdefine"]
                );
            }
            assert!(
                registry
                    .native_class_factory_recipe(
                        "oo::abstract",
                        dialect,
                        tcl_dialect::model::InvocationRealm::default()
                    )
                    .is_none()
            );
        }
    }
}
