//! `LSN::inbound` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disable inbound mapping for translation address and port associated with the current connection.",
            synopsis: &["LSN::inbound disable"],
            snippet: "Disable inbound mapping for translation address and port associated with the current connection.",
            source: "https://clouddocs.f5.com/api/irules/LSN__inbound.html",
            examples: "when HTTP_REQUEST {\n    LSN::inbound disable\n}",
            return_value: "LSN::inbound disable - Inbound connections can be permitted for a particular LSN pool to provide end-point independent filtering, described in RFC 4787.",
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
            FormSpec { kind: FormKind::Default, synopsis: "LSN::inbound disable" },
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
