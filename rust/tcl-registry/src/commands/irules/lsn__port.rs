//! `LSN::port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Explicitly set the translation address regardless of the configured LSN pool.",
            synopsis: &["LSN::port TRANSLATION_PORT"],
            snippet: "Explicitly set the translation address regardless of the configured LSN pool.\n\nThe LSN::port command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n\nLSN::port <translation_port>",
            source: "https://clouddocs.f5.com/api/irules/LSN__port.html",
            examples: "",
            return_value: "LSN::port",
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
            FormSpec { kind: FormKind::Default, synopsis: "LSN::port TRANSLATION_PORT" },
        ],
        ..CommandSpec::DEFAULT
    }
}
