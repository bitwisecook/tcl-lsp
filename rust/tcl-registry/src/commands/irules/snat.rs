//! `snat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snat",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Assigns the specified SNAT translation address to the current connection.",
            synopsis: &["snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))"],
            snippet: "Causes the system to assign the specified source address to the\nserverside connection(s). The assignment is valid for the duration of\nthe clientside connection or until 'snat none' is called. The iRule\nSNAT command overrides the SNAT configuration of the virtual server or\na SNAT pool. It does not override the 'Allow SNAT' setting of a pool.",
            source: "https://clouddocs.f5.com/api/irules/snat.html",
            examples: "# Apply SNAT autmap if the selected pool member IP address is 1.1.1.1\nwhen LB_SELECTED {\nIf { [IP::addr [LB::server addr] equals 1.1.1.1] } {\n     snat automap\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP", "MR"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
