//! `BWC::priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to create a priority class for a bwc policy.",
            synopsis: &["BWC::priority (PRIORITY_CLASS WEIGHT)"],
            snippet: "A BWC policy instance or category can be mapped to a priority class of a priority group. This is part of the configuration and can be done via tmsh or GUI. Once a BWC instance has these mappings we can use the iRule defined below to change those. The syntax for this iRule is like below,\n\nBWC::priority <name1> <value1> [<name2> <value2>] [<name3> <value3>] [<name4> <value4>]\n\n    nameX - name of a priority class. valueX - weight of the priority class in percentage.\n\nIn the above iRule you can specify one or more traffic classes and specify the desired weight-percentage for that priority-class.",
            source: "https://clouddocs.f5.com/api/irules/BWC__priority.html",
            examples: "when SERVER_CONNECTED {\n\n    set my_policy bwc_policy set my_cat none set my_cookie [IP::remote_addr] switch -glob [TCP::remote_port] {\n        \"101\" {\n            set my_cat c1\n        }\n        \"102\" {\n            set my_cat c2\n        }\n    }\n    BWC::policy attach $my_policy $my_cookie if { $my_cat != \"none\" } {\n        log local0. \"BWC::color set $my_policy $my_cat\" BWC::color set $my_policy $my_cat BWC::priority \"tc1\" 60 \"tc2\" 40\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BWC::priority (PRIORITY_CLASS WEIGHT)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
