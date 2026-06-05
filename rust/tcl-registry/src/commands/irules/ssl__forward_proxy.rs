//! `SSL::forward_proxy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::forward_proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status.",
            synopsis: &["SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)", "SSL::forward_proxy verified_handshake (enable | disable) ?", "SSL::forward_proxy cert response_control (ignore | mask) ?", "SSL::forward_proxy extension (ARG ARG)"],
            snippet: "This command sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate if the policy or cert subcommands are specified. If verified-handshake subcommand is specified, the command enables, disables or retrieves the verified_handshake behavior for the SSL handshake. If response_control subcommand is specified, the command ignore or mask the server side certificate errors while forging client certificate. If extension subcommand is specified, the command inserts an extension while forging a certificate.",
            source: "https://clouddocs.f5.com/api/irules/SSL__forward_proxy.html",
            examples: "when CLIENTSSL_SERVERHELLO_SEND {\n    log local0. 'bypassing'\n    SSL::forward_proxy policy bypass\n}",
            return_value: "SSL::forward_proxy policy <[bypass] | [intercept]> This command sets the policy of SSL Forward Proxy Bypass feature to \"bypass\" or \"intercept\"",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::forward_proxy <subcommand> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
