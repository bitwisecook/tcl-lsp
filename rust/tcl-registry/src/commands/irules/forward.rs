//! `forward` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "forward",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the connection to forward IP packets.",
            synopsis: &["forward"],
            snippet: "Sets the connection to forward IP packets. This is strict forwarding\nand will bypass any pool configured on the virtual server.\nThe request will be forwarded out the appropriate interface according\nto the routes in the LTM routing table. No destination address or port\ntranslation is performed.",
            source: "https://clouddocs.f5.com/api/irules/forward.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [class match [IP::client_addr] equals my_hosts_class]} {\n    snat 192.168.100.12\n  } else {\n    forward\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "forward" },
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
