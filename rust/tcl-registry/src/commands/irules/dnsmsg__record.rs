//! `DNSMSG::record` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::record",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the specified field from a resource record object.",
            &["DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
