//! `DNS::additional` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::additional",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns, inserts, removes, or clears RRs from the additional section.",
            &["DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
