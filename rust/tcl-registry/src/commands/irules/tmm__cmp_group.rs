//! `TMM::cmp_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the number (0-x) of the group of the CPU executing the rule.",
            synopsis: &["TMM::cmp_group"],
            snippet: "This command returns the number (0-x) of the group of the CPU currently\nexecuting the rule. Typically, a group refers to the blade number on a\nchassis system, and is always 0 on other platforms. New meanings may be\nadded for future platform architectures.\nThis is helpful if you believe one CPU is doing something it shouldn't\nand you want to isolate the issue rather than see an aggregate of all\nCPUs.\nTo determine the total number of TMM instances running, see the\nTMM::cmp_count page. To determine the CPU ID an iRule is current\nexecuting on within a blade, see the TMM::cmp_unit page.",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_group.html",
            examples: "# Note this example won't work in 10.1.0 - 10.2.2 and 11.0.x\n# as the iRule parser doesn't allow these commands in RULE_INIT\nwhen RULE_INIT {\n\n   # Check if we're running on the first CPU right now\nif { [TMM::cmp_unit] == 0 && [TMM::cmp_group] == 0 } {\n      # This execution is happening on the first TMM instance\n      # Conduct any initialization functionality just once here\n      log local0. \"some code\"\n   }\n}",
            return_value: "Returns the number (0-x) of the group of the CPU executing the rule.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TMM::cmp_group" },
        ],
        ..CommandSpec::DEFAULT
    }
}
