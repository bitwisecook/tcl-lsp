//! `LSN::persistence` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::persistence",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set the translation address and port selection mode for the current connection, and the translation entry timeout.",
            synopsis: &["LSN::persistence none (TIMEOUT)?", "LSN::persistence (address | address-port) TIMEOUT"],
            snippet: "Set the translation address and port selection mode for the current connection, and the translation entry timeout.\n\nLSN::persistence <none|address|address-port|strict-address-port> <timeout>",
            source: "https://clouddocs.f5.com/api/irules/LSN__persistence.html",
            examples: "",
            return_value: "LSN::persistence none",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["LSN"],
            also_in: &[
                "AUTH_RESULT",
                "AUTH_WANTCREDENTIAL",
                "CACHE_REQUEST",
                "CACHE_UPDATE",
                "CLIENTSSL_CLIENTCERT",
                "CLIENTSSL_HANDSHAKE",
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "HTTP_CLASS_FAILED",
                "HTTP_CLASS_SELECTED",
                "HTTP_REQUEST",
                "HTTP_REQUEST_DATA",
                "LB_SELECTED",
                "MR_INGRESS",
                "RTSP_REQUEST",
                "RTSP_REQUEST_DATA",
                "SIP_REQUEST",
                "STREAM_MATCHED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LSN::persistence none (TIMEOUT)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LsnState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
