//! `FLOW::create_related` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::create_related",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Creates a related client side and server side flow.", &["FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
