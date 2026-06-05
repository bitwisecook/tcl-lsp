//! `DNSMSG::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a field from the header of a dns_message.",
            synopsis: &["DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')"],
            snippet: "Takes a dns_message structure and field name, and returns the specified field value from the header.",
            source: "https://clouddocs.f5.com/api/irules/DNSMSG-header.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set rcode [DNSMSG::header $result rcode]\n}",
            return_value: "Returns a field from the header.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'cd')" },
        ],
        ..CommandSpec::DEFAULT
    }
}
