//! `XML::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates XML payload.",
            synopsis: &[
                "XML::payload (LENGTH | (OFFSET LENGTH))?",
                "XML::payload length",
                "XML::payload replace OFFSET LENGTH XML_PAYLOAD",
            ],
            snippet: "Queries for or manipulates XML payload.",
            source: "https://clouddocs.f5.com/api/irules/XML__payload.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "XML::payload (LENGTH | (OFFSET LENGTH))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
