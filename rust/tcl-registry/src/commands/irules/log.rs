//! `log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Write a message to BIG-IP logging facilities.",
            &["log ?facility.level? message"],
            "F5 iRules",
        )),
        // GAP-D2: tainted data in a log message → log injection /
        // forging (IRULE3003). Mirrors `irules/log.py`.
        taint_log_sink: Some("IRULE3003"),
        ..CommandSpec::DEFAULT
    }
}
