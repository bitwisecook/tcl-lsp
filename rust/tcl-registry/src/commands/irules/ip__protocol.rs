//! `IP::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::protocol",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP protocol value.",
            synopsis: &["IP::protocol"],
            snippet: "Returns the IP protocol value. This command replaces the BIG-IP 4.X variable ip_protocol.\nFor a list of the IP protocol numbers, see /etc/protocols or the L<IANA protocol number list|http://www.iana.org/assignments/protocol-numbers/protocol-numbers.xml>",
            source: "https://clouddocs.f5.com/api/irules/IP__protocol.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::protocol] == 6 } {\n     pool tcp_pool\n  } else {\n     pool slow_pool\n  }\n}",
            return_value: "IP protocol",
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
            FormSpec { kind: FormKind::Default, synopsis: "IP::protocol" },
        ],
        ..CommandSpec::DEFAULT
    }
}
