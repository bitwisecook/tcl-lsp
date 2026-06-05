//! `XLAT::listen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a related ephemeral listener.",
            synopsis: &["XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+"],
            snippet: "Creates a related ephemeral listener and returns the TCL handle for the listener. bind address and port can be omitted. It is recommend that users don't set this, so the command can choose an IP:port based on the server address specified and also conforms to source translation config. If the server address is on the clientside, then bind IP::port will be a valid endpoint on the clientside and conforms to the source translation config on the clientside.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__listen.html",
            examples: "when SERVER_CONNECTED {\n    set listen [XLAT::listen -inherit-main-rules 30 {\n        proto [IP::protocol]\n        bind -allow [LINK::vlan_id],/Common/public1 -ip [serverside {IP::local_addr}]\n        server [IP::client_addr] 7000\n        allow [LB::server addr] 0\n        inherit-vs [virtual]\n    }]\n    log local0. \"LISTEN: $listen\"\n\n    # hairpin\n    set listen_hairpin [XLAT::listen -hairpin 30 {\n        proto [IP::protocol]\n        bind -allow [clientside {LINK::vlan_id}]",
            return_value: "Return the TCL handle to the created listener. String representaion of the handle: \"<local addr>%<local route domain id>,<local port>,<remote addr>%<remote route domain id>,<remote port>,<server addr>%<server route domain id>,<server port>,<vlan id>,<protocol number>\".",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
