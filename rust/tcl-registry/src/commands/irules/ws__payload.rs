//! `WS::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Queries for or manipulates Websocket frame payload information.",
            synopsis: &["WS::payload (LENGTH | (OFFSET LENGTH))?", "WS::payload length", "WS::payload replace OFFSET LENGTH STRING"],
            snippet: "WS::payload <length>\n    Returns the content that the WS::collect command has collected thus far, up to the number of bytes specified. If you do not specify a size, the system returns the entire collected content.\n\nWS::payload <offset> <length>\n    Returns the content that the WS::collect command has collected thus far from the specified offset, up to the number of bytes specified.\n\nWS::payload length\n    Returns the size of the content that has been collected thus far, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/WS__payload.html",
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
            FormSpec { kind: FormKind::Default, synopsis: "WS::payload (LENGTH | (OFFSET LENGTH))?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
