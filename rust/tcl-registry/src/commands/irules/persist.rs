//! `persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "persist",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the connection persistence type.",
            synopsis: &["persist none", "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?", "persist source_addr (IPV4_MASK)? (TIMEOUT)?", "persist simple (IPV4_MASK)? (TIMEOUT)?"],
            snippet: "Causes the system to use the named persistence type to persist the\nconnection. Also allows direct inspection and manipulation of the\npersistence table.",
            source: "https://clouddocs.f5.com/api/irules/persist.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n   # Persist the client connection based on the SSL session ID\n    persist ssl\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: true,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
