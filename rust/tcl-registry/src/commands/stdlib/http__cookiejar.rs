//! `http::cookiejar` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cookiejar",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create or configure an HTTP cookie jar (TclOO class).",
            synopsis: &[
                "http::cookiejar create name ?filename?",
                "http::cookiejar new ?filename?",
            ],
            snippet: "A TclOO class implementing RFC 6265 cookie management.  Instances track cookies received in HTTP responses and automatically attach them to subsequent requests.",
            source: "Tcl stdlib cookiejar package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("cookiejar"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
