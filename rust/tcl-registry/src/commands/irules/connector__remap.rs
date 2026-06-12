//! `CONNECTOR::remap` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::remap",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set client/server IP/Port from connector.",
            synopsis: &["CONNECTOR::remap server_addr IP_ADDR", "CONNECTOR::remap client_addr IP_ADDR", "CONNECTOR::remap client_port PORT", "CONNECTOR::remap server_port PORT"],
            snippet: "CONNECTOR::remap client_addr\n    Set the client IP address from connector profile.\nCONNECTOR::remap server_addr\n    Set the server IP address from connector profile.\nCONNECTOR::remap client_port\n    Set the client port from connector profile.\nCONNECTOR::remap server_port\n    Set the server port from connector profile.",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__remap.html",
            examples: "when CONNECTOR_OPEN {\n                if {([CONNECTOR::profile] eq \"/Common/connector_profile_1\")} {\n                    CONNECTOR::remap client_addr 10.10.10.2\n                    log local0. \"Remap client IP address from connector to 10.10.10.2\"\n                    CONNECTOR::remap client_port 333\n                    log local0. \"Remap client port from connector to 333\"\n                    CONNECTOR::remap server_addr 20.20.20.2",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CONNECTOR::remap server_addr IP_ADDR" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
