//! `RADIUS::rtdom` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::rtdom",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command overwrites the default route-domain ID in RADIUS scenario with given value",
            synopsis: &["RADIUS::rtdom (ROUTE_DOMAIN)?"],
            snippet: "This command overwrites the default route-domain ID in RADIUS scenario with given value",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__rtdom.html",
            examples: "when CLIENT_ACCEPTED {\n        if { [RADIUS::code] == 4 } {\n            set rd 0\n            # Extract the APN information from the AVP\n            set called_station_id [RADIUS::avp 30 \"string\"]\n            if {$called_station_id == \"station1\"} {\n                set rd 1\n            } elseif {$called_station_id == \"station2\"} {\n                set rd 2\n            }\n            # Overwrite the default route domain value with the new value.\n            RADIUS::rtdom $rd\n        }\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RADIUS::rtdom (ROUTE_DOMAIN)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
