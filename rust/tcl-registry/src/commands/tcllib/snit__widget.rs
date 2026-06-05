//! `snit::widget` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::widget name definition",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::widget",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
hover: Some(HoverSnippet {
    summary: "Define a new snit megawidget type.",
    synopsis: &["snit::widget name definition"],
    snippet: "Like snit::type but creates a Tk megawidget. Automatically delegates to a hull widget.",
    source: "tcllib snit package",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        arg_roles: &[(1, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
