//! `http::geturl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::geturl",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Retrieve a URL — the primary command for the http package.",
            &["http::geturl url ?options?"],
            "F5",
        )),
        // GAP-D2: `url` (arg 0) is a network-address arg — SSRF sink
        // (T104); `-headers` can carry credentials. Mirrors
        // `stdlib/http_.py`.
        taint_network_sink_args: Some(&[0]),
        credential_options: &["-headers"],
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
