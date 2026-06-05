//! `WS::masking` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::masking",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command determines the behavior of Websocket processing.",
            synopsis: &["WS::masking ( 'preserve' | 'remask' )"],
            snippet: "WS::masking preserve\n    The WebSockets module will not unmask the payload. Data received from the end-points will be sent untouched to other modules for further processing.\n\nWS::masking remask\n    The data received from the end-points is unmasked and sent to other modules for further processing. The client-to-server frame's payload is then masked with the specified mask before sending data out on the wire again.",
            source: "https://clouddocs.f5.com/api/irules/WS__masking.html",
            examples: "when WS_REQUEST {\n    WS::masking preserve\n    WS::masking remask\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "WS::masking ( 'preserve' | 'remask' )" },
        ],
        ..CommandSpec::DEFAULT
    }
}
