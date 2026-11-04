use tcl_syntax::expr::mathfunc::{MathResultClass, result_class, spec};

#[test]
fn shared_specs_own_result_classes() {
    for (name, class) in [
        ("ceil", MathResultClass::Float),
        ("floor", MathResultClass::Float),
        ("trunc", MathResultClass::Float),
        ("int", MathResultClass::Int),
        ("round", MathResultClass::Int),
        ("isnan", MathResultClass::Bool),
        ("signbit", MathResultClass::Bool),
        ("abs", MathResultClass::Numeric),
        ("max", MathResultClass::Numeric),
    ] {
        let row = spec(name).unwrap_or_else(|| panic!("missing spec {name}"));
        assert_eq!(row.result_class, class, "spec {name}");
        assert_eq!(result_class(name), class, "lookup {name}");
    }
    assert_eq!(result_class("unknown"), MathResultClass::Any);
}
