//! `DNSMSG::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Returns a field from the header of a dns_message.", &["DNSMSG::header DNS_MESSAGE ('rcode' | 'opcode' | 'id' | 'ra' | 'rd' | 'tc' | 'qr' | 'aa' | 'ad' | 'c"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
