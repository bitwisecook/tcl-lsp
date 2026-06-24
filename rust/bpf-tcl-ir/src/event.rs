//! The BPF-native event space. Inspired by F5's `when <EVENT>` shape, but a
//! whole separate namespace — these events map to eBPF program types / attach
//! points, not F5 iRules events.

use crate::ir::ProgType;

/// Resolve an event name (as written in `when <EVENT> …`) to a program type.
/// Case-insensitive. Returns `None` for unknown events.
#[must_use]
pub fn event_to_prog_type(event: &str) -> Option<ProgType> {
    match event.to_ascii_uppercase().as_str() {
        "SOCKET_FILTER" | "SOCKET" => Some(ProgType::SocketFilter),
        _ => None,
    }
}

/// The known event names, for diagnostics and help text.
pub const KNOWN_EVENTS: &[&str] = &["SOCKET_FILTER"];
