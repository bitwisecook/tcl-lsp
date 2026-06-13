//! `SSL::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return data back to the origin via SSL.",
            synopsis: &["SSL::respond DATA"],
            snippet: "Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
            source: "https://clouddocs.f5.com/api/irules/SSL__respond.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n              # Trigger collection of the decrypted payload once the SSL or DTLS handshake has been completed successfully\n              SSL::collect\n            }",
            return_value: "SSL::respond <data> Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::respond DATA" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
