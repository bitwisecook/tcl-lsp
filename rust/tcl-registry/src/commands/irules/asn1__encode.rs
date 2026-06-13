//! `ASN1::encode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Encodes ASN.1 records.",
            synopsis: &["ASN1::encode ('BER' | 'DER') FORMAT (VALUE)*", "ASN1::encode ('insert' | 'replace') ELEMENT OFFSET FORMAT (VALUE)*"],
            snippet: "This command is used to encode ASN.1 records. Data is formatted according to formatString.\n\nformatString can have the following characters:\n\n    a - Octet String\n    B - Bit String\n    b - Boolean\n    e - Enum\n    i - Integer\n    t - Tag of next element\n    ? - Don't output the component if the corresponding value is empty\n    ?hex-tag - Denotes that the specifier which follows is for an optional component. This is used for encoding or decoding an ASN.1 Set or Sequence which contains nested OPTIONAL or DEFAULT components. hex-tag, is a two-character hex byte of the expected tag.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__encode.html",
            examples: "# LDAP String Modify\nappend base_mod $base \",dc=supercalafragalisticexpialadoshus\"\nASN1::encode replace $ele 1 \"a\" $base_mod\n\n# LDAP Encode/Rewrite - The size field is 4 elements forward from $ele\nASN1::encode replace $ele 4 \"i\" [incr size 2]\n\n# LDAP Encode/Rewrite - The time field is 5 elements forward from $ele\nASN1::encode replace $ele 5 \"i\" [expr $time + 100]\n\n# Encode an LDAP SearchRequest Extensible Match filter where RuleId and Type are optional,",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASN1::encode ('BER' | 'DER') FORMAT (VALUE)*" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
