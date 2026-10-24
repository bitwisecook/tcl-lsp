// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Single-use source rewrites for a concrete private Native consumer.
//!
//! This is a trusted backend implementation seam, not source-entry metadata.
//! Its only concrete production provider owns an inaccessible VM, compiler,
//! memory I/O and all cells/objects inside an owned execution worker. A provider
//! must close actual dispatch, input, observer and retirement boundaries; a
//! source profile, fresh-world flag or effect summary cannot implement that
//! contract. The ordinary compiler never obtains this scope implicitly.
//!
//! The boundary covers admitted execution and successful detached returns. An
//! unexpected host panic can invoke the process-global Rust panic hook or stderr
//! before join; this API neither promises panic freedom nor intercepts that hook.
//! A fatal host abort cannot promise retirement or join.

use std::cell::{Cell, RefCell};

use crate::native_compilation::{NativeCompilationEntry, NativeCompilationFrame};

/// A closed operation declined before exposing a native value or resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedSourceRefusal {
    /// Source or its complete operation footprint is unsupported.
    Source,
    /// The actual Native owner or independent observer state is unavailable.
    NativeContext,
    /// Source, owner, registry or command state changed, or the owner retired.
    Currency,
    /// Another selection superseded this rewrite, or it was already consumed.
    Selection,
    /// A declared source, operation, arithmetic or output bound was exceeded.
    Budget,
    /// Private execution failed; no result-object or interpreter escapes.
    Execution,
}

/// Detached observable bytes. There are no runtime values or callable handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSourceTranscript {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Completed private execution and passive source text for inspection only.
/// The source text supplies no licence to execute a rewrite in an open VM.
#[derive(Debug)]
pub struct ClosedSourceOutput {
    pub transcript: ClosedSourceTranscript,
    pub rewritten_source: String,
}

/// Independently recaptured actual state of the concrete private provider.
/// `Currency` includes the provider's complete registry and source-plan owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedNativeState<Currency> {
    pub entry: NativeCompilationEntry,
    pub currency: Currency,
}

/// Actual private execution backend, consumed rather than borrowed from an
/// existing interpreter. Implementations must own their VM, original compiler,
/// private I/O and all reachable values, enforce the entire checked footprint
/// (including compiled/inline paths), and admit no external callbacks or state.
///
/// `capture_closed_state` must inspect current actual state. It must also verify
/// all planned scalar roots are initially absent ordinary cells and the actual
/// private channel/dispatch boundary remains installed. `validate_rewrite`
/// independently checks the complete candidate footprint and original source
/// correspondence. These obligations cannot be discharged by a copied flag.
///
/// `execute_owned` must retire the entire VM/compiler/value world before it
/// returns detached bytes. Its enclosing concrete entry joins the owned worker
/// before returning to its caller. No generic `Host`, `Write`, `CompileService`,
/// native value or VM callback is accepted from that caller.
pub trait ClosedNativeBackend: Sized {
    type Currency: Clone + Eq;

    fn original_source(&self) -> &str;
    fn capture_closed_state(
        &self,
    ) -> Result<ClosedNativeState<Self::Currency>, ClosedSourceRefusal>;
    fn validate_rewrite(&self, candidate: &str) -> Result<(), ClosedSourceRefusal>;
    fn execute_owned(
        self,
        candidate: String,
    ) -> Result<ClosedSourceTranscript, ClosedSourceRefusal>;
}

/// Owns the concrete backend until one admitted execution consumes it.
/// No backend accessor exists, and a snapshot cannot construct this owner.
pub struct ClosedNativeConsumerOwner<B: ClosedNativeBackend> {
    backend: RefCell<Option<B>>,
    original: String,
    captured: ClosedNativeState<B::Currency>,
    selection: Cell<u64>,
}

impl<B: ClosedNativeBackend> ClosedNativeConsumerOwner<B> {
    /// Enter only from an actual private backend. Source consumers cannot
    /// substitute a profile, flag or copied Native entry for the owned backend.
    pub fn new(backend: B) -> Result<Self, ClosedSourceRefusal> {
        let captured = backend.capture_closed_state()?;
        Self::check_native_state(&captured.entry)?;
        let original = backend.original_source().to_owned();
        backend.validate_rewrite(&original)?;
        Ok(Self {
            backend: RefCell::new(Some(backend)),
            original,
            captured,
            selection: Cell::new(0),
        })
    }

    fn check_native_state(entry: &NativeCompilationEntry) -> Result<(), ClosedSourceRefusal> {
        if !entry.closed
            || entry.frame != NativeCompilationFrame::Global
            || entry.execution_point.is_none()
            || entry.invocation_policy.is_none()
            || entry.expression_policy.is_none()
            || !entry.variable_observers.permits_no_callbacks()
            || !entry
                .command_resolvers
                .is_some_and(|inventory| inventory.permits_no_callbacks(entry))
            || entry.namespace_variable_tables.is_none()
        {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        Ok(())
    }

    fn check_current(&self) -> Result<(), ClosedSourceRefusal> {
        let backend = self.backend.borrow();
        let backend = backend.as_ref().ok_or(ClosedSourceRefusal::Currency)?;
        let current = backend.capture_closed_state()?;
        Self::check_native_state(&current.entry)?;
        if current != self.captured || backend.original_source() != self.original {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(())
    }

    pub fn scope(&self) -> Result<ClosedNativeConsumerScope<'_, B>, ClosedSourceRefusal> {
        self.check_current()?;
        Ok(ClosedNativeConsumerScope { owner: self })
    }

    /// Consume the private backend exactly once. The `RefCell` borrow ends before
    /// native execution/retirement. Captured metadata is dropped by the enclosing
    /// worker before its detached output leaves the thread.
    pub fn execute(
        &self,
        rewrite: ClosedSourceRewrite<'_, B>,
    ) -> Result<ClosedSourceOutput, ClosedSourceRefusal> {
        if !std::ptr::eq(self, rewrite.owner) || rewrite.selection != self.selection.get() {
            return Err(ClosedSourceRefusal::Selection);
        }
        self.check_current()?;
        {
            let backend = self.backend.borrow();
            backend
                .as_ref()
                .ok_or(ClosedSourceRefusal::Currency)?
                .validate_rewrite(&rewrite.source)?;
        }
        self.check_current()?;
        let backend = self
            .backend
            .borrow_mut()
            .take()
            .ok_or(ClosedSourceRefusal::Currency)?;
        let source = rewrite.source;
        let transcript = backend.execute_owned(source.clone())?;
        Ok(ClosedSourceOutput {
            transcript,
            rewritten_source: source,
        })
    }
}

/// Borrowed live permission to propose rewrites for this private source only.
/// It exposes no Native entry, object, cell, interpreter or generic optimiser grant.
pub struct ClosedNativeConsumerScope<'owner, B: ClosedNativeBackend> {
    owner: &'owner ClosedNativeConsumerOwner<B>,
}

impl<'owner, B: ClosedNativeBackend> ClosedNativeConsumerScope<'owner, B> {
    #[must_use]
    pub fn source(&self) -> &str {
        &self.owner.original
    }

    /// Selected actual policy is data for checked preparation, not execution authority.
    #[must_use]
    pub fn profile(&self) -> &tcl_dialect::DialectProfileKey {
        &self.owner.captured.entry.profile
    }

    #[must_use]
    pub fn execution_point(&self) -> tcl_dialect::model::DialectPoint {
        self.owner
            .captured
            .entry
            .execution_point
            .expect("checked native point")
    }

    /// Candidate remains inside the owning consumer and must pass its complete
    /// original/candidate footprint check. Selecting again retires earlier choices.
    pub fn select(
        &self,
        candidate: String,
    ) -> Result<ClosedSourceRewrite<'owner, B>, ClosedSourceRefusal> {
        self.owner.check_current()?;
        {
            let backend = self.owner.backend.borrow();
            backend
                .as_ref()
                .ok_or(ClosedSourceRefusal::Currency)?
                .validate_rewrite(&candidate)?;
        }
        self.owner.check_current()?;
        let selection = self
            .owner
            .selection
            .get()
            .checked_add(1)
            .ok_or(ClosedSourceRefusal::Selection)?;
        self.owner.selection.set(selection);
        Ok(ClosedSourceRewrite {
            owner: self.owner,
            selection,
            source: candidate,
        })
    }
}

/// Single selected source, borrowed from its real private consumer lifetime.
/// It cannot outlive that owner, become a CU entry or carry native object state.
pub struct ClosedSourceRewrite<'owner, B: ClosedNativeBackend> {
    owner: &'owner ClosedNativeConsumerOwner<B>,
    selection: u64,
    source: String,
}

impl<B: ClosedNativeBackend> ClosedSourceRewrite<'_, B> {
    /// Passive inspection only; ordinary source execution has no grant from this text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}
