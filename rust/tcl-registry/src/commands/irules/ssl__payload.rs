//! `SSL::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns and manipulates plaintext data collected via SSL::collect.",
            synopsis: &["SSL::payload (length |"],
            snippet: "The SSL::payload commands allow you to return and manipulate the data collected via the SSL::collect command. This data is in plaintext format.",
            source: "https://clouddocs.f5.com/api/irules/SSL__payload.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0. \"[IP::client_addr]:[TCP::client_port]: SSL handshake completed, collecting SSL payload\"\n    SSL::collect\n}",
            return_value: "SSL::payload length Returns the amount of plaintext data collected by the SSL::collect command.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::payload (length |" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
