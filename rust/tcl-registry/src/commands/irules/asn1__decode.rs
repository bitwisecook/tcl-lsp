//! `ASN1::decode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::decode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Decodes ASN.1 records.",
            synopsis: &["ASN1::decode ELEMENT FORMAT (VAR_NAME)*"],
            snippet: "This command is used to decode ASN.1 records. element specifies the\ndata to decode. It can be either a byte array (like is returned from\nTCP::payload), or an element object returned by one of the other\ncommands. The bytes are decoded according to formatString, and the\nresults are stored in variables whose names are provided as command\narguments. If a variable with a name specified doesn't exist, it will\nbe created in the current scope. If the variable does exist, it's value\nwill be overwritten. The command returns the number of elements\ndecoded.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__decode.html",
            examples: "ASN1::decode $ele \"?a?aa?b\" ruleId type matchValue dnAttrs\nif {![info exists ruleId] && ![info exists type]} {\n  log local0. \"ERR: extensibleMatch must contain either a matchingRule or type component\"\n}\n# Handle default value for dnAttributes component\nif {![info exists dnAttrs]} {\n  set dnAttrs 0\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASN1::decode ELEMENT FORMAT (VAR_NAME)*",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        ..CommandSpec::DEFAULT
    }
}
