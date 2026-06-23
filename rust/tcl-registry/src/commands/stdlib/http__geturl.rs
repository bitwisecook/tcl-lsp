//! `http::geturl` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::geturl",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Retrieve a URL — the primary command for the http package.",
            synopsis: &["http::geturl url ?options?"],
            snippet: "Retrieves the resource at *url* and returns a token that can be passed to the other ``http::`` commands.  Options include ``-query``, ``-headers``, ``-handler``, ``-command``, ``-timeout``, ``-type``, ``-method``, ``-keepalive`` and more.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        // `url` (arg 0) is a network-address arg — SSRF sink
        // (T104); `-headers` can carry credentials.
        taint_network_sink_args: Some(&[0]),
        credential_options: &["-headers"],
        required_package: Some("http"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
