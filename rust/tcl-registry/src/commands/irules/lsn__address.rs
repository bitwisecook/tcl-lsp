//! `LSN::address` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Explicitly set the translation address regardless of the configured LSN pool.",
            synopsis: &["LSN::address TRANSLATION_ADDR"],
            snippet: "Explicitly set the translation address regardless of the configured LSN pool.\n\nThe LSN::address command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n\nAgruments:\n    LSN::address - Set the explicit translation IPv4 or IPv6 address for the connection in the current context.",
            source: "https://clouddocs.f5.com/api/irules/LSN__address.html",
            examples: "when HTTP_REQUEST {\n    LSN::address 10.0.0.1\n}",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "LSN::address TRANSLATION_ADDR" },
        ],
        ..CommandSpec::DEFAULT
    }
}
