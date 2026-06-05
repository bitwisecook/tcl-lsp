//! `LSN::persistence-entry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::persistence-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Create or lookup LSN translation address.",
            synopsis: &["LSN::persistence-entry (delete|get) CLIENT_ADDR", "LSN::persistence-entry create (-override)? LSN_POOL CLIENT_ADDR TRANSLATION_ADDR (TIMEOUT)?"],
            snippet: "Create or lookup LSN translation address. Those commands are linked to CGNAT module introduced in 11.3. You need to license and provision this module to use this command.\n\nLSN::persistence-entry create [-override] <client_address>[:<client_port>] [<translation_address>[:<translation_port>]]\nLSN::persistence-entry get <client_address>[:<client_port>]\n\nv11.4+\nLSN::persistence-entry create [-override] <lsn_pool>  <client_address>[:<port>] <translation_address>[:<port>]]  [timeout]\n\nv11.5+\nLSN::persistence-entry delete <client_address>",
            source: "https://clouddocs.f5.com/api/irules/LSN__persistence-entry.html",
            examples: "when CLIENT_ACCEPTED {\n    set clientIP [IP::client_addr]\n}",
            return_value: "LSN::persistence-entry create",
        }),
        ..CommandSpec::DEFAULT
    }
}
