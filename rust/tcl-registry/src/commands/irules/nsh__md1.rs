//! `NSH::md1` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::md1",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets/Get the MD1 context for NSH.",
            synopsis: &["NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?"],
            snippet: "Set: MD1 context for NSH. Offset, length and data string as arguments.\n            Get: MD1 context from NSH. Only offset and length as arguments.",
            source: "https://clouddocs.f5.com/api/irules/NSH__md1.html",
            examples: "ntext for NSH.\n            when CLIENT_ACCEPTED {\n                set str {1234567890123456}\n                NSH::md1 serverside_egress 1 16 [binary format a* $str]\n                set myctx1 [NSH::md1 serverside_egress 1 16]\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
