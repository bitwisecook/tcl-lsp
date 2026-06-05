//! `send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends data on an existing sideband connection.",
            synopsis: &["send (("],
            snippet: "This command sends data on an existing sideband connection (established with connect). It is one of several commands that make up the ability to create sideband connections from iRules.\n\nArguments\n\n    <connection> is the connection identifier returned from connect\n\n    <data> is the data to send\n\n    -timeout ms specifies the amount of time to wait for the data to be sent. The default is an immediate timeout.\n\n    -status varname will save the result of the send command into varname. The possible status values are:\n        1. sent - the data was sent successfully\n        2.",
            source: "https://clouddocs.f5.com/api/irules/send.html",
            examples: "when LB_SELECTED {\n    # Save some data to send\n    set dest \"10.0.16.1:8888\"\n    set data \"GET /mypage/myindex2.html HTTP/1.0\\r\\n\\r\\n\"\n\n    # Open a new TCP connection to $dest\n    set conn_id [connect -protocol TCP -timeout 30000 -idle 30 $dest]\n\n    # Send the data with a 1000ms timeout on the connection identifier received from the connect command\n    set send_bytes [send -timeout 1000 -status send_status $conn_id $data]\n\n    # Log the number of bytes sent and the send status",
            return_value: "Sends data on a specified sideband connection, and returns an integer representing the amount of data that was sent.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "send ?options? ?--? connection data" },
        ],
        options: &[
            OptionSpec { name: "-timeout", takes_value: true, value_hint: "MSEC", detail: "Time in ms to wait for data to be sent.", dialects: None },
            OptionSpec { name: "-status", takes_value: true, value_hint: "VARIABLE", detail: "Save send status into variable.", dialects: None },
            OptionSpec { name: "--", takes_value: false, value_hint: "", detail: "", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
