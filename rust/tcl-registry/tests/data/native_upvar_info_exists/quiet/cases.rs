pub const QUIET_CASES: &[&str] = &[
    "upvar #0 ::g alias; trace add variable alias read traceRead; info exists alias",
    "upvar #0 ::g alias; trace add variable alias read traceFail; info exists alias",
    "upvar 0 a(k) alias; trace add variable alias read traceFail; info exists alias",
];
