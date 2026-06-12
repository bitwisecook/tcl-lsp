//! `virtual` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "virtual",
        traits: Traits::CSE_CANDIDATE.union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to.",
            synopsis: &["virtual", "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?"],
            snippet: "Returns the name of the associated virtual server that the connection\nis flowing through. In 9.4.0 and higher, it can be also used to route\nthe connection to another virtual server and an optional IP address\nand port, without leaving the BIG-IP.",
            source: "https://clouddocs.f5.com/api/irules/virtual.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"Current virtual server name: [virtual name]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "virtual" },
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
