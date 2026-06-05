//! `SSL::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases the collected plaintext data.",
            synopsis: &["SSL::release (LENGTH)?"],
            snippet: "Releases the collected plaintext data to the next layer/filter up.",
            source: "https://clouddocs.f5.com/api/irules/SSL__release.html",
            examples: "when SERVERSSL_DATA {\n    # Do something with the decrypted data\n    set payload [SSL::payload]\n\n    # Release the payload\n    SSL::release\n}",
            return_value: "SSL::release [<length>] Releases the collected plaintext data to the next layer/filter up.",
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
            FormSpec { kind: FormKind::Default, synopsis: "SSL::release (LENGTH)?" },
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
