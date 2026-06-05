//! `connect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Establishes a sideband connection.",
            &["connect info ("],
            "F5 iRules",
        )),
        // GAP-D2: sideband `connect` is a network sink (SSRF, T104);
        // the address-bearing arg positions are not pinned. Mirrors
        // `irules/connect.py` (`taint_network_sink_args=()`).
        taint_network_sink_args: Some(&[]),
        ..CommandSpec::DEFAULT
    }
}
