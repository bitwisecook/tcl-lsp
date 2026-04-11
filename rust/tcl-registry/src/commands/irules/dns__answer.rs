//! `DNS::answer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::answer",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns, inserts, removes, or clears all RRs from the answer section.",
            &["DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
