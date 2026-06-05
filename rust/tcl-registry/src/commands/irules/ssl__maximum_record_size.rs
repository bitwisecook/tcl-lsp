//! `SSL::maximum_record_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::maximum_record_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set the maximum egress record size.",
            synopsis: &["SSL::maximum_record_size (SSL_RECORD_SIZE)?"],
            snippet: "SSL::maximum_record_size\n  Returns the currently set maximum egress record size.\nSSL::maximum_record_size #####\n  Set the maximum egress record size.",
            source: "https://clouddocs.f5.com/api/irules/SSL__maximum_record_size.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::maximum_record_size 1234\n}",
            return_value: "SSL::maximum_record_size Returns the currently set maximum egress record size. SSL::maximum_record_size ##### There is no return value.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::maximum_record_size (SSL_RECORD_SIZE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
