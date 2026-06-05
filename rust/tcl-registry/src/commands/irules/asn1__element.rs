//! `ASN1::element` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::element",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns ASN1.1 record elements.",
            synopsis: &["ASN1::element init ('BER' | 'DER')", "ASN1::element next ELEMENT (NUM_ELEMENTS)?", "ASN1::element byte_offset ELEMENT (OFFSET)?", "ASN1::element tag ELEMENT"],
            snippet: "This command returns ASN1.1 record elements.\n\nASN1::element init encodingType\n\n     * Returns an element (Tcl_Obj) handle used by the remaining commands.\n       encodingType specifies the encoding type that subsequent commands\n       should use (BER|DER).\n\nASN1::element next element ?numberOfElements?\n\n     * Returns the next element found after element. If numberOfElements\n       is specified, the command will move ahead that many elements,\n       otherwise, the default is 1.\n\nASN1::element byte_offset element ?offset?\n\n     * Returns the byte offset within the payload.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__element.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASN1::element init ('BER' | 'DER')" },
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
