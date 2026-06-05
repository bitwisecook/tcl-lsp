//! `X509::subject_public_key_RSA_bits` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_RSA_bits",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the size of the subjectXs public RSA key of an X509 certificate.",
            synopsis: &["X509::subject_public_key_RSA_bits CERTIFICATE"],
            snippet: "Returns the size, in bits, of the subject’s public RSA key of the\nspecified X509 certificate. This command is only applicable when the\npublic key type is RSA. Otherwise, the command generates an error.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key_RSA_bits.html",
            examples: "when HTTP_REQUEST {\n  if { [info exist error_code] } {\n    if { $error_code > 0 } {\n      HTTP::redirect \"https://some_other_site/\"\n    }\n  }\n}",
            return_value: "Returns the size of the subject’s public RSA key of an X509 certificate.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::subject_public_key_RSA_bits CERTIFICATE" },
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
