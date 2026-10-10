// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Same-entry TclOO allocations and their independent bootstrap roles.

use crate::native_compilation::{
    NativeCompilationEntry, NativeInterpreterIdentity, NativeNamespaceContext,
};
use tcl_core_types::NameBytes;

/// Role assigned by an actual backend bootstrap allocation, never by a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeOoBootstrapRole {
    /// Foundation object root.
    ObjectRoot,
    /// Foundation ordinary class factory.
    ClassFactory,
    /// Configurable metaclass allocation.
    ConfigurableFactory,
    /// Configurable instance support class allocation.
    ConfigurableSupport,
}

/// Actual intrinsic method body retained by a backend method allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeOoIntrinsicMethod {
    /// Stock inherited object destruction.
    Destroy,
    /// Stock named-object manufacture.
    Create,
    /// Stock anonymous-object manufacture.
    New,
    /// Stock class definition constructor.
    ClassConstructor,
    /// Stock configurable constructor.
    ConfigurableConstructor,
    /// Stock configurable accessor dispatcher.
    Configure,
}

/// One retained own method record, independent of dispatch selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeOoMethodObservation {
    /// Original counted method table key; lifecycle methods have no key.
    pub name: Option<NameBytes>,
    /// Identity of the actual retained method allocation.
    pub allocation: u64,
    /// Actual record has visibility flags but no callable body.
    pub visibility_only: bool,
    /// Independently installed intrinsic handler, if present.
    pub intrinsic: Option<NativeOoIntrinsicMethod>,
    /// Current own method visibility.
    pub public: bool,
    /// Current true-private role.
    pub private: bool,
}

/// Own lifecycle state; inherited providers remain separate class rows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeOoLifecycleObservation {
    /// Actual own slot is absent.
    Absent,
    /// Actual own slot retains this method allocation.
    Present(NativeOoMethodObservation),
}

/// Current class and object facets from the same actual backend allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeOoClassObservation {
    /// Actual stable object allocation identity.
    pub object: u64,
    /// Actual object creation incarnation, unchanged by rename.
    pub creation: u64,
    /// Independently stamped bootstrap role, retained across rename.
    pub bootstrap_role: Option<NativeOoBootstrapRole>,
    /// Current public command token supplying this allocation.
    pub command_token: u64,
    /// Current callable implementation incarnation.
    pub implementation_generation: u64,
    /// Actual shared method/dispatch Foundation currency.
    pub foundation_epoch: u64,
    /// Actual receiver-specific dispatch currency.
    pub dispatch_epoch: u64,
    /// Actual class of the object facet.
    pub maker: u64,
    /// Actual ordered own superclass allocations.
    pub superclasses: Vec<u64>,
    /// Actual ordered own class mixins.
    pub class_mixins: Vec<u64>,
    /// Actual ordered own object mixins.
    pub object_mixins: Vec<u64>,
    /// Complete class filter inventory; absence retains unknown.
    pub class_filters: Option<Vec<NameBytes>>,
    /// Complete object filter inventory; absence retains unknown.
    pub object_filters: Option<Vec<NameBytes>>,
    /// Actual object-private namespace incarnation and geometry.
    pub namespace: NativeNamespaceContext,
    /// Actual stored class definition namespace; None is observed unset.
    /// An unavailable stored token makes the whole inventory unavailable.
    pub class_definition_namespace: Option<NativeNamespaceContext>,
    /// Actual stored object definition namespace; None is observed unset.
    /// An unavailable stored token makes the whole inventory unavailable.
    pub object_definition_namespace: Option<NativeNamespaceContext>,
    /// Actual own constructor slot, independently of its providers.
    pub constructor: NativeOoLifecycleObservation,
    /// Actual own destructor slot, independently of its providers.
    pub destructor: NativeOoLifecycleObservation,
    /// Actual class-side visibility overrides.
    pub class_exported: Vec<NameBytes>,
    /// Actual class-side unexported overrides.
    pub class_unexported: Vec<NameBytes>,
    /// Actual object-side visibility overrides.
    pub object_exported: Vec<NameBytes>,
    /// Actual object-side unexported overrides.
    pub object_unexported: Vec<NameBytes>,
    /// Complete actual own class-side method table.
    pub class_methods: Vec<NativeOoMethodObservation>,
    /// Complete actual own object-side method table.
    pub object_methods: Vec<NativeOoMethodObservation>,
}

/// Backend-captured OO state at a single original compilation boundary.
/// This grants no handler, body, compiler, variable or observer capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeOoClassInventory {
    /// Actual runtime and interpreter owner.
    pub interpreter: NativeInterpreterIdentity,
    /// Same-entry mutation currency.
    pub epoch: u64,
    /// Complete current class allocation rows.
    pub classes: Vec<NativeOoClassObservation>,
}

impl NativeCompilationEntry {
    /// Select the actual class behind this current command incarnation.
    /// Missing, foreign or conflicting allocations remain unavailable.
    #[must_use]
    pub fn original_oo_class(
        &self,
        token: u64,
        generation: u64,
    ) -> Option<&NativeOoClassObservation> {
        let inventory = self.oo_classes.as_ref()?;
        if inventory.interpreter != self.interpreter
            || inventory.epoch != self.epoch
            || !self.closed
        {
            return None;
        }
        let mut bindings = self.commands.iter().filter(|binding| {
            binding.token == token && binding.implementation_generation == generation
        });
        bindings.next()?;
        if bindings.next().is_some() {
            return None;
        }
        let mut matches = inventory.classes.iter().filter(|class| {
            class.command_token == token && class.implementation_generation == generation
        });
        let class = matches.next()?;
        if matches.next().is_some() || class.namespace.interpreter != self.interpreter {
            return None;
        }
        let namespace = self
            .retained_namespace_context(class.namespace.token)
            .ok()?;
        if namespace != class.namespace {
            return None;
        }
        Some(class)
    }
}
