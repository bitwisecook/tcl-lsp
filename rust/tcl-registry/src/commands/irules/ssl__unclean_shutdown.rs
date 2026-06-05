//! `SSL::unclean_shutdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::unclean_shutdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the value of the Unclean Shutdown setting.",
            synopsis: &["SSL::unclean_shutdown (enable | disable)"],
            snippet: "Sets the value of the Unclean Shutdown setting. This command only affects the current connection, and only affects the current context (e.g., when run in a client-side context, it only affects the current client-side connection).",
            source: "https://clouddocs.f5.com/api/irules/SSL__unclean_shutdown.html",
            examples: "# Note that for this iRule, unclean shutdown should be disabled in the clientssl profile\nwhen HTTP_REQUEST {\n    if { [HTTP::header \"User-Agent\"] contains \"MSIE\" } {\n        SSL::unclean_shutdown enable\n    }\n}",
            return_value: "SSL::unclean_shutdown <\"enable\" | \"disable\"> Sets the value of the current client-side or server-side SSL connection’s Unclean Shutdown setting.",
        }),
        ..CommandSpec::DEFAULT
    }
}
