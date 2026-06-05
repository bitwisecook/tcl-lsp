//! `SSL::extensions` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::extensions",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or manipulates SSL extensions.",
            synopsis: &["SSL::extensions (count |", "SSL::extensions insert OPAQUE_EXT"],
            snippet: "Returns or manipulates SSL extensions.",
            source: "https://clouddocs.f5.com/api/irules/SSL__extensions.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n    set my_ext \"Hello world!\"\n    set my_ext_type 62965\n    SSL::extensions insert [binary format S1S1a* $my_ext_type [string length $my_ext] $my_ext]\n}",
            return_value: "SSL::extensions Returns the extensions sent by the peer as a single opaque byte array. Valid in all SSL handshake events (those other than *SSL_DATA).",
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
            FormSpec { kind: FormKind::Default, synopsis: "SSL::extensions ?options?" },
        ],
        options: &[
            OptionSpec { name: "-index", takes_value: true, value_hint: "EXT_NUMBER", detail: "Return extension at specified index.", dialects: None },
            OptionSpec { name: "-type", takes_value: true, value_hint: "EXT_TYPE_VALUE", detail: "Return extension matching specified type value.", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
