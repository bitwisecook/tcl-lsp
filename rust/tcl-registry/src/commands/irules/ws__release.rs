//! `WS::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases the data collected using WS::collect.",
            synopsis: &["WS::release"],
            snippet: "WS::release\n    Releases the data collected via WS::collect. Unless a subsequent WS::collect command was issued, there is no need to use the WS::release command inside of the WS_CLIENT_DATA and WS_SERVER_DATA events, since (in these cases) the data is implicitly released.",
            source: "https://clouddocs.f5.com/api/irules/WS__release.html",
            examples: "when WS_CLIENT_FRAME {\n    WS::collect frame 1000\n    set clen 1000\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "WS::release" },
        ],
        ..CommandSpec::DEFAULT
    }
}
