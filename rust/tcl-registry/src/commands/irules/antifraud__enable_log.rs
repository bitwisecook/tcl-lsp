//! `ANTIFRAUD::enable_log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable_log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables Anti-Fraud TMM logs for the current transaction.",
            synopsis: &["ANTIFRAUD::enable_log (LOG_LEVEL)?"],
            snippet: "ANTIFRAUD::enable_log\n                Enables Anti-Fraud TMM logs at 'Informational' (default) log level for the current transaction.\n\n            ANTIFRAUD::enable_log LOG_LEVEL ;\n                Enables Anti-Fraud TMM logs at 'LOG_LEVEL' (can be any of: 'Error'/'Warning'/'Notice'/'Informational'/'Debug') log level for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable_log.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Enable-log\" ] } {\n                    ANTIFRAUD::enable_log\n                    log local0. \"Logs enabled\"\n                }\n            }",
            return_value: "ANTIFRAUD::enable_log No return value (enables Anti-Fraud TMM logs at default log level for the current transaction).",
        }),
        ..CommandSpec::DEFAULT
    }
}
