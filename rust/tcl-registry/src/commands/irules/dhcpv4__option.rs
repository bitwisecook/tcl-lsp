//! `DHCPv4::option` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command retrieves,sets or deletes the option by id number.",
            synopsis: &["DHCPv4::option (delete)? OPTION (VALUE)?"],
            snippet: "This command retrieves,sets or deletes the option by id number\n\nDetails (syntax);\nDHCPv4::option <id>\nDHCPv4::option <id> <value>\nDHCPv4::option delete <id>",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__option.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Option [DHCPv4::option 18]\"\n    }",
            return_value: "This command returns value by option id number when retrieving",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::option (delete)? OPTION (VALUE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
