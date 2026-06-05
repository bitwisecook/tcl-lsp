//! `table` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "table",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides enhanced access to the session table.",
            synopsis: &["table set (((-mustexist | -excl) -notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?", "table add ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?", "table replace ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?", "table lookup ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY"],
            snippet: "The table command is a superset of the session command, with improved syntax for general purpose use. Please see the table command article series for detailed information on its use.\n\nThis command is not available to GTM.\n\nIf the table command is used on the standby system in a HA pair, the command will perform a no-op because the content of the standby unit's session db should be updated only through mirroring.",
            source: "https://clouddocs.f5.com/api/irules/table.html",
            examples: "when RULE_INIT {\n    set static::maxquery 100\n    set static::holdtime 600\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: true,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
