//! `IP::tos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::tos",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns (or sets) the ToS value encoded within a packet.",
            synopsis: &["IP::tos (clientside | serverside)? (IP_TOS)?"],
            snippet: "Returns (or sets) the ToS value encoded within a packet. The Type of Service (ToS) standard is a means by which network equipment can identify and treat traffic differently based on an identifier. As traffic enters the site, the BIG-IP system can apply a rule that sends the traffic to different pools of servers based on the ToS level within a packet, or can set the ToS value on traffic matching specific patterns.",
            source: "https://clouddocs.f5.com/api/irules/IP__tos.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::tos] == 64 } {\n     pool telnet_pool\n  } else {\n     pool slow_pool\n }\n}",
            return_value: "Returns the ToS value encoded within a packet",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IP::tos (clientside | serverside)? (IP_TOS)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
