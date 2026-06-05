//! `X509::pem2der` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::pem2der",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an X509 certificate in DER format.",
            synopsis: &["X509::pem2der <cert-in-pem>"],
            snippet: "Returns the specified X509 certificate, in its entirety, in DER format.",
            source: "https://clouddocs.f5.com/api/irules/X509__pem2der.html",
            examples: "when HTTP_REQUEST {\n    if { [SSL::cert count] > 0 } {\n        SSL::c3d cert [X509::pem2der [X509::whole [SSL::cert 0]]]\n    }\n}",
            return_value: "Returns an X509 certificate in DER format.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::pem2der <cert-in-pem>" },
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
