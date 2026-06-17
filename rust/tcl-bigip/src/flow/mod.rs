//! PCAP flow extraction and session pairing for `f5 explain-flow`.
//!
//! Faithful port of the `dialects/f5/bigip/flow/` package: walk a capture into
//! per-5-tuple [`Flow`]s ([`packets::extract_flows`]), then pair them into
//! bidirectional [`Connection`]s and front/back [`Session`]s
//! ([`sessions::pair_sessions`]). The per-session *explanation* (which virtual
//! server matched, the iRule event chain, the policy trace) is built by the
//! driver in `f5-cli`, which consumes these types.

pub mod model;
pub mod packets;
pub mod sessions;

pub use model::{Connection, Flow, FlowKey, Session};
pub use packets::extract_flows;
pub use sessions::{pair_connections, pair_sessions};
