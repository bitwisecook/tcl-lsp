pub const ORDER_CASES: &[&str] = &[
    "lassign $name first [set first]; list $first $A",
    "catch {lassign $name first [error STOP]} caught; list $first $caught",
    "lassign $name first a([set first]); list $first $a(A)",
    "lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A",
];
