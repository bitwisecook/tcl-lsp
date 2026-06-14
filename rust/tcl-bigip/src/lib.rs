//! Faithful Rust port of the F5 BIG-IP object model and config parser
//! (`dialects/f5/bigip/`).
//!
//! The parser turns BIG-IP `.conf` / `.scf` text into a [`model`] of
//! typed objects, mirroring the Python `parse_bigip_conf` ->
//! `BigipConfig` contract field-for-field so the `PyO3` layer can
//! reconstruct the existing Python dataclasses byte-identically. The
//! object/property *schema* is reused from `tcl_registry::bigip`.
//!
//! Port status: foundation layer (value types, parser helpers, minimal /
//! generic objects, source ranges). Rich per-kind typed objects and the
//! full driver land incrementally, each verified against the Python
//! parser by the differential-parity harness.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod apl;
pub mod canonical;
pub mod graph;
pub mod model;
pub mod parser;
pub mod range;
pub mod value;

pub use range::{Position, Range};
