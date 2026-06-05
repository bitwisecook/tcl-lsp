//! `MR::peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::peer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Defines a peer to use for routing a message to.",
            synopsis: &["MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))"],
            snippet: "The MR::peer command defines a peer to use for routing a message to. The peer may either refer to a named pool or a tuple (IP address, port and route domain iD). When creating a connection to a peer, the parameters of either a virtual server or a transport config object will be used. The peer object will only exist in the current connections connflow. When adding a route (via MR::route add), it will first look for a locally created peer object then for a peer object from the configuration. Once the current connection closes, the local peer object will go away.",
            source: "https://clouddocs.f5.com/api/irules/MR__peer.html",
            examples: "when CLIENT_ACCEPTED {\n    MR::peer self_peer config tc1 host \"[IP::remote_addr]:[TCP::remote_port]\"\n    GENERICMESSAGE::route add dest \"[IP::remote_addr]\" peer self_peer\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
