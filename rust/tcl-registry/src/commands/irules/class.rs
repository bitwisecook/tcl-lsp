//! `class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "class",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Advanced access of classes.", &["class 'match' (((CLASS_SEARCH_OPTION) ('-all'))#)? ('--')? ITEM CLASS_OPERATOR CLASS_OBJ"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
