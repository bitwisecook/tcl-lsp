//! BIG-IP config parser — Rust port of `dialects/f5/bigip/parser/`.

pub mod bespoke;
pub mod driver;
pub mod header;
pub mod helpers;
pub mod scalar;

pub use driver::{BigipConfig, Placed, parse_bigip_conf};

pub use header::{ObjectTypeIndex, parse_generic_header};
pub use helpers::{
    Block, Property, extract_blocks, parse_keyed_block_entries, parse_list_block, parse_properties,
    parse_properties_with_spans, tokenise_header,
};
