// SPDX-License-Identifier: AGPL-3.0-or-later
//! Future executable-argument lookup coordinates, independent of timing,
//! installed prefix ownership and the command table at actual callback entry.

/// The frame whose namespace and command paths are used when an executable
/// argument runs. This metadata grants neither callback reach nor bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptLookupScope {
    /// The receiving invocation's current physical frame at callback entry.
    InvokingFrame,
    /// The interpreter's global frame when a deferred callback actually runs.
    GlobalFrame,
    /// The frame of the operation that triggers an installed trace or unknown handler.
    TriggerFrame,
}

impl ScriptLookupScope {
    /// Complete authorable scope catalogue; none is the conservative default.
    pub const ALL: [Self; 3] = [Self::InvokingFrame, Self::GlobalFrame, Self::TriggerFrame];
}
