//! `QOE::video` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::video",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Returns a set of video QOE attributes from the current video connection.",
            synopsis: &["QOE::video"],
            snippet: "This command returns a set of video QOE attributes from the current video connection.",
            source: "https://clouddocs.f5.com/api/irules/QOE__video.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["QOE"],
            also_in: &["CLIENT_CLOSED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "QOE::video" },
        ],
        ..CommandSpec::DEFAULT
    }
}
