//! `X509::serial_number` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::serial_number",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the serial number of an X509 certificate.",
            synopsis: &["X509::serial_number CERTIFICATE"],
            snippet: "Returns the serial number of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__serial_number.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Certificate Serial Number: [X509::serial_number cert_x]\"\n}",
            return_value: "Returns the serial number of an X509 certificate.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::serial_number CERTIFICATE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
