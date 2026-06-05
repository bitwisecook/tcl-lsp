//! `IP::stats` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::stats",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Supplies information about the number of packets or bytes being sent or received in a given connection.",
            synopsis: &["IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?"],
            snippet: "This command supplies information about the number of packets or bytes being sent or received in a given connection.\n\nIP::stats\nReturns a list with Packets In, Packets Out, Bytes In, Bytes Out & Age\n\nIP::stats pkts in\nReturns number of packets received\n\nIP::stats pkts out\nReturns number of packets sent\n\nIP::stats pkts\nReturns a Tcl list of packets in and packets out\n\nIP::stats bytes in\nReturns number of bytes received\n\nIP::stats bytes out\nReturns number of bytes sent\n\nIP::stats bytes\nReturns Tcl list of bytes in and bytes out\n\nIP::stats age\nReturns the age of the connection in milliseconds",
            source: "https://clouddocs.f5.com/api/irules/IP__stats.html",
            examples: "# The following example calculates and logs response time:\nwhen HTTP_REQUEST {\n    set reqAge [IP::stats age]\n    set reqURI [HTTP::uri]\n    set reqClient [IP::remote_addr]:[TCP::remote_port]\n}",
            return_value: "number of packets or bytes being sent or received in a given connection",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IP::stats ?pkts|bytes|in|out|age? ?in|out?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
