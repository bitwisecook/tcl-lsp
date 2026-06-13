//! `IP::reputation` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::reputation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Looks up the supplied IP address in the IP intelligence (reputation) database and returns a TCL list containing reputation categories.",
            synopsis: &["IP::reputation (IP_ADDR)+"],
            snippet: "Performs a lookup of the supplied IP address against the IP reputation database. Returns a TCL list containing possible reputation categories:\n\nCategory                     Description\nBotnets                      IP addresses of computers that are infected with malicious software and are controlled as a group, and are now part of a botnet. Hackers can exploit botnets to send spam messages, launch various attacks, or cause target systems to behave in other unpredictable ways.\nCloud Provider Networks      IP addresses of cloud providers.",
            source: "https://clouddocs.f5.com/api/irules/IP__reputation.html",
            examples: "#Drop the packet after initial TCP handshake if the client has a bad reputation\nwhen CLIENT_ACCEPTED {\n    # Check if the IP reputation list for the client IP is not 0\n    if {[llength [IP::reputation [IP::client_addr]]] != 0}{\n        # Drop the connection\n        drop\n    }\n}",
            return_value: "Return a TCL list containing reputation categories.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IP::reputation (IP_ADDR)+" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
