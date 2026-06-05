//! `remote_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remote_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Use IP::remote_addr instead.",
            synopsis: &["remote_addr"],
            snippet: "Returns the IP address of the host on the far end of the connection. In the clientside context, this is the client IP address. In the serverside context this is the node IP address. You can also specify the IP::client_addr and IP::server_addr commands, respectively.\n\nIn BIG-IP 10.x with route domains enabled this command returns the remote IP address in the x.x.x.x%rd of the server or client (depending on the context) that is in any non-default route domain else it returns just the IP address as expected.\n\nThis command is equivalent to the BIG-IP 4.X variable remote_addr.",
            source: "https://clouddocs.f5.com/api/irules/remote_addr.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [IP::addr [IP::remote_addr] equals 206.0.0.0 mask 255.0.0.0] } {\n        pool clients_from_206\n    } else {\n        pool other_clients_pool\n    }\n}",
            return_value: "Returns the IP address of the host on the far end of the connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "remote_addr" },
        ],
        ..CommandSpec::DEFAULT
    }
}
