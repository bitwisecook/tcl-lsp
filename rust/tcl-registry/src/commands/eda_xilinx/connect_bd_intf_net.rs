//! `connect_bd_intf_net` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "connect_bd_intf_net ?-intf_net net_name? intf_pin_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect_bd_intf_net",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Connect block design interface nets.",
            &["connect_bd_intf_net ?-intf_net net_name? intf_pin_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
