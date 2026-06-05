//! `snatpool` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snatpool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Assigns the specified SNAT pool or SNAT pool member to the current connection.",
            synopsis: &["snatpool SNAT_POOL_OBJ (member IP_ADDR)?"],
            snippet: "Causes the pool of addresses identified by <snatpool_name> to be used\nas translation addresses to create a SNAT.",
            source: "https://clouddocs.f5.com/api/irules/snatpool.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TCP::local_port] == 531 } {\n     snatpool chat_snatpool\n}\n  elseif { [TCP::local_port] == 25 } {\n     snatpool smtp_snatpool member 10.20.30.40\n }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
