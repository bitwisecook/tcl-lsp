//! `SSL::forward_proxy` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "policy",
        arity: Arity::new(0, 1),
        detail: "Set or get forward proxy bypass policy.",
        synopsis: "SSL::forward_proxy policy ?bypass | intercept?",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "bypass",
                    detail: "Bypass SSL forward proxy.",
                },
                ArgValue {
                    value: "intercept",
                    detail: "Intercept SSL forward proxy.",
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cert",
        arity: Arity::new(0, 2),
        detail: "Retrieve the forged certificate or set cert options.",
        synopsis: "SSL::forward_proxy cert ?response_control? ?status?",
        pure: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "response_control",
                    detail: "Control response to cert errors.",
                },
                ArgValue {
                    value: "status",
                    detail: "Set server certificate status.",
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "verified_handshake",
        arity: Arity::new(0, 1),
        detail: "Enable, disable, or get verified handshake semantics.",
        synopsis: "SSL::forward_proxy verified_handshake ?enable | disable?",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "enable",
                    detail: "Enable verified handshake.",
                },
                ArgValue {
                    value: "disable",
                    detail: "Disable verified handshake.",
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(2),
        detail: "Insert a certificate extension while forging.",
        synopsis: "SSL::forward_proxy extension <oid> <value>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::forward_proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status.",
            synopsis: &[
                "SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                "SSL::forward_proxy verified_handshake (enable | disable) ?",
                "SSL::forward_proxy cert response_control (ignore | mask) ?",
                "SSL::forward_proxy extension (ARG ARG)",
            ],
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::forward_proxy <subcommand> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
