// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Empty registration-world observations without original object ownership.

use std::hash::{Hash, Hasher};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, Weak};

use crate::native_compilation::NativeInterpreterIdentity;

/// Same-capture absence of all global literal registrations.
/// A receipt retains only weak metadata, and supplies no object/type/value,
/// compiler or handler authority. Registration attempts, another capture and
/// retirement invalidate it before any native getter or owner release occurs.
#[derive(Clone)]
pub struct NativeEmptyLiteralWorld {
    interpreter: NativeInterpreterIdentity,
    world: u64,
    generation: u64,
    capture_epoch: u64,
    lifetime: Weak<Mutex<ObservationState>>,
}

impl NativeEmptyLiteralWorld {
    /// Check the actual original capture and live unchanged registration owner.
    #[must_use]
    pub fn is_current_for(
        &self,
        interpreter: NativeInterpreterIdentity,
        capture_epoch: u64,
    ) -> bool {
        self.interpreter == interpreter && self.capture_epoch == capture_epoch && self.is_current()
    }

    /// Whether this original capture still has its same live world generation.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.lifetime.upgrade().is_some_and(|state| {
            state
                .lock()
                .expect("native literal observation owner")
                .generation
                == Some(self.generation)
        })
    }
}

impl std::fmt::Debug for NativeEmptyLiteralWorld {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeEmptyLiteralWorld")
            .field("interpreter", &self.interpreter)
            .field("world", &self.world)
            .field("generation", &self.generation)
            .field("capture_epoch", &self.capture_epoch)
            .finish_non_exhaustive()
    }
}
impl PartialEq for NativeEmptyLiteralWorld {
    fn eq(&self, other: &Self) -> bool {
        (
            self.interpreter,
            self.world,
            self.generation,
            self.capture_epoch,
        ) == (
            other.interpreter,
            other.world,
            other.generation,
            other.capture_epoch,
        )
    }
}
impl Eq for NativeEmptyLiteralWorld {}
impl Hash for NativeEmptyLiteralWorld {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (
            self.interpreter,
            self.world,
            self.generation,
            self.capture_epoch,
        )
            .hash(state);
    }
}

struct ObservationState {
    generation: Option<u64>,
    interpreter: Option<NativeInterpreterIdentity>,
}

pub(super) struct NativeLiteralObservationOwner {
    world: u64,
    lifetime: Arc<Mutex<ObservationState>>,
}
impl Default for NativeLiteralObservationOwner {
    fn default() -> Self {
        static NEXT_WORLD: AtomicU64 = AtomicU64::new(1);
        Self {
            world: crate::checked_counter::allocate(&NEXT_WORLD)
                .expect("native literal world identities exhausted"),
            lifetime: Arc::new(Mutex::new(ObservationState {
                generation: Some(1),
                interpreter: None,
            })),
        }
    }
}
impl NativeLiteralObservationOwner {
    pub(super) fn invalidate(&self) {
        let mut state = self
            .lifetime
            .lock()
            .expect("native literal observation owner");
        state.generation = state.generation.and_then(|epoch| epoch.checked_add(1));
    }

    pub(super) fn capture(
        &self,
        interpreter: NativeInterpreterIdentity,
        capture_epoch: u64,
        empty: bool,
    ) -> Option<NativeEmptyLiteralWorld> {
        let mut state = self
            .lifetime
            .lock()
            .expect("native literal observation owner");
        state.generation = state.generation.and_then(|epoch| epoch.checked_add(1));
        let generation = state.generation?;
        match state.interpreter {
            Some(owner) if owner != interpreter => {
                state.generation = None;
                return None;
            }
            None => state.interpreter = Some(interpreter),
            _ => {}
        }
        empty.then(|| NativeEmptyLiteralWorld {
            interpreter,
            world: self.world,
            generation,
            capture_epoch,
            lifetime: Arc::downgrade(&self.lifetime),
        })
    }

    pub(super) fn retire(&self) {
        self.lifetime
            .lock()
            .expect("native literal observation owner")
            .generation = None;
    }
}
