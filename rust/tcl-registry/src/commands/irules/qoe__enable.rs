//! `QOE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Enables the video QOE filter and allows processing video on a connection basis.",
            synopsis: &["QOE::enable"],
            snippet: "This command enables the video QOE filter and allows processing video on a connection basis.",
            source: "https://clouddocs.f5.com/api/irules/QOE__enable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CLASSIFICATION", "FASTHTTP", "QOE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
