//! Explicit storage policy for an authored multi-worker iRules simulator.
//!
//! This contract supplies no appliance or native interpreter capability. A
//! backend must enroll actual worker namespaces and resolve actual cells.

/// Independently selected authored static storage contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthoredTmmStaticPolicy {
    /// Reached initialisation writes publish to every live worker. Ordinary
    /// globals and subsequent event writes remain owned by their worker.
    RuleInitPublication,
}

/// The event owner selects the context independently of variable spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TmmStaticExecutionContext {
    InitialisationBroadcast,
    ExecutingWorker,
}

/// Explicit worker topology selected by the simulator, independently of the
/// existence of unrelated child interpreters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AuthoredTmmWorkerTopology {
    #[default]
    RootInterpreter,
    EnrolledChildren,
}

impl AuthoredTmmStaticPolicy {
    #[must_use]
    pub const fn publishes(self, context: TmmStaticExecutionContext) -> bool {
        matches!(context, TmmStaticExecutionContext::InitialisationBroadcast)
    }
}

/// Actual recipient namespace and observer generation retained by the authored
/// compilation contract. No variable contents or native object are issued.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredTmmStaticRecipient {
    pub namespace: crate::native_compilation::NativeNamespaceContext,
    pub observer_epoch: u64,
}

/// Explicit authored publication context at an actual compilation boundary.
/// Missing event context retains possible outward callbacks for static stores;
/// the physical interpreter's local observer table cannot close those effects.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredTmmStaticCompilationContext {
    pub policy: AuthoredTmmStaticPolicy,
    pub context: Option<TmmStaticExecutionContext>,
    pub namespace: crate::native_compilation::NativeNamespaceContext,
    pub recipients: Vec<AuthoredTmmStaticRecipient>,
    pub outward_observers: crate::native_compilation::NativeVariableObserverPresence,
}

/// Independently authored storage scope for the measured event's counted keys.
/// The frame owner supplies worker and domain identity; naming evidence supplies
/// no storage, namespace, trace or native object authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredObservedFrameStorageContext {
    pub policy: tcl_syntax::naming::ObservedBigIpNamePolicy,
    pub tmm: u64,
    pub domain: u64,
}
