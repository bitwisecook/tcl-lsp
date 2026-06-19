//! Verb handlers for the `f5-query` CLI.
//!
//! Each handler mirrors a `_run_*` function in `tooling/f5/verbs/*`. Most f5
//! verbs await their BIG-IP engine ports (analysis/emit live in
//! `dialects/f5/bigip/*`); the ones here need only file I/O + the existing
//! `tcl-bigip` parser.

pub mod cleanup;
pub mod convert;
pub mod diff;
pub mod emit;
pub mod enrich_pcapng;
pub mod enrich_wireshark;
pub mod explain;
pub mod explain_flow;
pub mod extract;
pub mod fetch;
pub mod graph;
pub mod grep;
pub mod irule;
pub mod merge;
pub mod pcap_remap;
pub mod pull;
pub mod push;
pub mod query;
pub mod redact;
pub mod registry_dump;
pub mod remote;
pub mod rename;
pub mod scf;
pub mod secrets;
pub mod split;
pub mod stats;
pub mod tmsh;
pub mod unredact;
pub mod validate;
