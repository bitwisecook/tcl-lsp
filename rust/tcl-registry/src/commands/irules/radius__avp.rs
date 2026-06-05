//! `RADIUS::avp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns or adds/changes/removes RADIUS attribute-value pairs.",
            synopsis: &["RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?", "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)"],
            snippet: "This command returns or adds/changes/removes RADIUS attribute-value pairs. Radius profile must be applied for access to this command.",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__avp.html",
            examples: "when RULE_INIT {\n        set static::secret \"linus\"\n    }",
            return_value: "RADIUS::avp attr [attr_type] Returns the value of the specified RADIUS attribute. optional attr_type = ( octet | ip4 | ip6 | integer | string)",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_CLOSED",
                "CLIENT_DATA",
                "SERVER_CLOSED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
