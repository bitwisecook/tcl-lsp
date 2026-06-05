//! `use` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "use",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "A BIG-IP 4.X statement.",
            synopsis: &["use clone pool POOL_OBJ (member IP_ADDR)?", "use nexthop ((IP_ADDR) | ((VLAN_OBJ) (IP_ADDR | MAC_ADDR | transparent)?))", "use node (IP_TUPLE | (IP_ADDR (PORT)?))", "use pool POOL_OBJ (member (IP_TUPLE | (IP_ADDR (PORT)?)))?"],
            snippet: "This is a BIG-IP 4.X statement, provided for backward-compatibility. The use statement must be paired with certain BIG-IP 9.X commands such as node, pool, rateclass, snat, and snatpool.\n\nThe use command is not required on BIG-IP 9.X systems.",
            source: "https://clouddocs.f5.com/api/irules/use.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"aol\" } {\n        use pool aol_pool\n    } else {\n        use pool all_pool\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
