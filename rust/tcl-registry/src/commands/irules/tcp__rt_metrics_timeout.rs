//! `TCP::rt_metrics_timeout` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rt_metrics_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets cmetrics cache entry lifetime (timeout).",
            &["TCP::rt_metrics_timeout TIMEOUT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
