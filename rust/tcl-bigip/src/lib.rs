//! F5 BIG-IP object model and config parser.
//!
//! The parser turns BIG-IP `.conf` / `.scf` text into a [`model`] of
//! typed objects via `parse_bigip_conf` -> `BigipConfig`. The
//! object/property *schema* is reused from `tcl_registry::bigip`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod apl;
pub mod canonical;
pub mod cleanup;
pub mod convert;
pub mod f5_trailer;
pub mod flow;
pub mod graph;
pub mod grep;
pub mod irule_context;
pub mod jsonfmt;
pub mod lint;
pub mod model;
pub mod parser;
pub mod pcap_enrich;
pub mod pcap_remap;
pub mod pcapng;
pub mod policy_eval;
pub mod range;
pub mod redact;
pub mod secrets;
pub mod stats;
pub mod tmsh_emit;
pub mod validator;
pub mod value;
pub mod wireshark_profile;

pub use range::{Position, Range};
