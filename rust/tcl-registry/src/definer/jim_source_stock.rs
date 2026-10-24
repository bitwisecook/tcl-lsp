// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite authored Jim class publications, independently of Native execution.

/// Generated command role in the stock Jim class source recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceJimClassHelper {
    /// The last inherited stock parent.
    BaseClass,
    /// The class's ordinary procedure dispatcher.
    Dispatcher,
    /// Deferred object construction.
    New,
    /// Deferred finalisation hook.
    Finalize,
    /// The method procedure creator.
    Method,
    /// Sorted instance variable names.
    Variables,
    /// The retained variable dictionary.
    ClassVariables,
    /// The retained class name.
    ClassName,
    /// Deferred command inventory.
    Methods,
    /// The default constructor wrapper.
    DefaultConstructor,
    /// Alias to the default constructor wrapper.
    ConstructorAlias,
    /// The destruction wrapper.
    Destroy,
    /// The variable-read wrapper.
    Get,
    /// The evaluation wrapper.
    Evaluate,
}

impl SourceJimClassHelper {
    /// Exact stock script suffix; omission denotes the class name itself.
    #[must_use]
    pub const fn publication_suffix(self) -> Option<&'static [u8]> {
        Some(match self {
            Self::BaseClass => b"baseclass",
            Self::Dispatcher => return None,
            Self::New => b"new",
            Self::Finalize => b"finalize",
            Self::Method => b"method",
            Self::Variables => b"vars",
            Self::ClassVariables => b"classvars",
            Self::ClassName => b"classname",
            Self::Methods => b"methods",
            Self::DefaultConstructor => b"defaultconstructor",
            Self::ConstructorAlias => b"constructor",
            Self::Destroy => b"destroy",
            Self::Get => b"get",
            Self::Evaluate => b"eval",
        })
    }
}

/// Symbolic object-dispatch command produced by stock Jim construction.
/// It supplies no exact name, implementation allocation or actual receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceJimObjectCommandFootprint {
    /// A generated reference name published in the root command namespace.
    GlobalGeneratedReference,
}

impl SourceJimObjectCommandFootprint {
    /// A necessary fragment of every generated Jim object name. This broad
    /// test only withdraws absence; it never identifies an allocated object.
    #[must_use]
    pub fn may_be_named_by(self, name: &[u8]) -> bool {
        match self {
            Self::GlobalGeneratedReference => {
                tcl_syntax::naming::NativeNameProtocol::Jim084.could_name_generated_reference(name)
            }
        }
    }
}

/// Closed source-effect layout of the stock Jim class procedure. Selecting this
/// layout supplies neither an installed provider nor successful execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceJimStockClassRecipe;

impl SourceJimStockClassRecipe {
    pub(crate) fn for_original_layout(
        layout: &crate::resolved_invocation::AuthoredSourceJimClassLists,
    ) -> Option<Self> {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        let policy = layout.policy?;
        (layout.grammar.family == super::DefinerFamily::JimClass
            && policy.authority() == NamePolicyAuthority::AuthoredSimulation
            && policy.recipe() == NativeNameProtocol::Jim084)
            .then_some(Self)
    }

    /// Complete empty-base publication order, including the ordinary alias allocation.
    #[must_use]
    pub const fn publications(self) -> &'static [SourceJimClassHelper] {
        use SourceJimClassHelper as H;
        &[
            H::Dispatcher,
            H::New,
            H::Finalize,
            H::Method,
            H::Variables,
            H::ClassVariables,
            H::ClassName,
            H::Methods,
            H::DefaultConstructor,
            H::ConstructorAlias,
            H::Destroy,
            H::Get,
            H::Evaluate,
        ]
    }

    /// Sorted method suffixes returned by the complete empty-base stock inventory.
    #[must_use]
    pub const fn inherited_aliases(self) -> &'static [SourceJimClassHelper] {
        use SourceJimClassHelper as H;
        &[
            H::ClassName,
            H::ClassVariables,
            H::ConstructorAlias,
            H::DefaultConstructor,
            H::Destroy,
            H::Evaluate,
            H::Finalize,
            H::Get,
            H::Method,
            H::Methods,
            H::New,
            H::Variables,
        ]
    }

    /// Additional current implementations used while inheriting this inventory.
    #[must_use]
    pub const fn inheritance_dependencies(self) -> &'static [&'static str] {
        &["lsort", "lmap", "info", "lindex", "split"]
    }

    /// Class, supported single-base inheritance and method-creator calls only
    /// publish definitions. Their reference operations remain inside New.
    #[must_use]
    pub const fn publication_preserves_references(self) -> bool {
        true
    }

    /// Symbolic command name introduced by stock object construction.
    #[must_use]
    pub const fn object_command_footprint(self) -> SourceJimObjectCommandFootprint {
        SourceJimObjectCommandFootprint::GlobalGeneratedReference
    }

    /// Core implementations used by the conditional stock object-call prelude.
    /// Their identity does not prove object construction or quiet GC effects.
    #[must_use]
    pub const fn method_entry_dependencies(self) -> &'static [&'static str] {
        &[
            "exists", "uplevel", "lindex", "info", "set", "foreach", "upvar", "unset", "eval",
            "lsort", "dict", "ref",
        ]
    }

    /// Wrapper locals that cannot also be an incoming formal or instance link.
    #[must_use]
    pub const fn method_entry_reserved_locals(self) -> &'static [&'static [u8]] {
        &[b"self", b"__", b"__body"]
    }

    /// Local written by the selected wrapper before its original body.
    #[must_use]
    pub const fn method_entry_self_name(self) -> &'static str {
        "self"
    }

    /// Direct stock commands called while installing an empty-base class.
    /// Consumers must prove their current selected implementations separately.
    #[must_use]
    pub const fn direct_dependencies(self) -> &'static [&'static str] {
        &[
            "set", "foreach", "dict", "proc", "if", "exists", "tailcall", "alias", "return",
        ]
    }
}
