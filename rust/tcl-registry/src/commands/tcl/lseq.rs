//! `lseq` — generate a list of numeric values in a range (Tcl 9.0).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lseq ?start? ?op? end ?by step? ?count n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lseq",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(1, 5),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Generate a list of numeric values in a range.",
            synopsis: &[
                "lseq n",
                "lseq start end ?step?",
                "lseq start to end",
                "lseq start 'count' count",
                "lseq start 'by' step 'count' count",
            ],
            snippet: "Returns a list of numbers from start through end (inclusive) with optional step.  One-arg form yields 0..n-1.  Float and double values are supported.",
            source: "Tcl man page lseq.n (Tcl 9.0)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
