//! Parity checks for the value model, `jsonfmt`, and `output` render modes.
//!
//! The expected strings are captured verbatim from the query DSL's reference
//! output (`output.render` / `json.dumps`) — see the `gen`-comment beside
//! each — and asserted against this crate. Self-contained: no Python at test
//! time.

use indexmap::IndexMap;
use tcl_bigip_query::output::render;
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{jsonfmt, value};

fn obj(pairs: &[(&str, Value)]) -> Value {
    let mut m = IndexMap::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Object(m)
}

fn s(text: &str) -> Value {
    Value::Str(text.to_string())
}

#[test]
fn json_pretty_matches_python() {
    let v = Value::List(vec![
        Value::Int(1),
        Value::Float(2.0),
        s("é"),
        Value::Bool(true),
        Value::Null,
        Value::List(vec![Value::Int(1), s("x")]),
        obj(&[("a", Value::Int(1))]),
    ]);
    // json.dumps([...], indent=2)
    let expected = "[\n  1,\n  2.0,\n  \"\\u00e9\",\n  true,\n  null,\n  [\n    1,\n    \"x\"\n  ],\n  {\n    \"a\": 1\n  }\n]";
    assert_eq!(jsonfmt::to_pretty(&v), expected);
}

#[test]
fn json_compact_matches_python() {
    let v = obj(&[
        ("a", Value::Int(1)),
        ("b", Value::List(vec![Value::Int(1), Value::Int(2)])),
    ]);
    assert_eq!(jsonfmt::to_compact(&v), r#"{"a":1,"b":[1,2]}"#);
}

#[test]
fn float_repr_matches_python() {
    assert_eq!(jsonfmt::py_float_repr(0.1), "0.1");
    assert_eq!(jsonfmt::py_float_repr(1.0), "1.0");
    assert_eq!(jsonfmt::py_float_repr(1.5), "1.5");
    assert_eq!(jsonfmt::py_float_repr(100.0), "100.0");
    assert_eq!(jsonfmt::py_float_repr(-3.25), "-3.25");
}

#[test]
fn render_json_mode_matches_python() {
    let values = vec![Value::Int(1), Value::Float(2.0), s("é")];
    assert_eq!(
        render(&values, "json").unwrap(),
        "[\n  1,\n  2.0,\n  \"\\u00e9\"\n]\n"
    );
}

#[test]
fn render_raw_mode_matches_python() {
    let values = vec![Value::Int(1), s("a"), Value::Bool(true), Value::Null];
    assert_eq!(render(&values, "raw").unwrap(), "1\na\ntrue\nnull\n");
}

#[test]
fn render_auto_scalar_list_matches_python() {
    let values = vec![Value::List(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ])];
    assert_eq!(render(&values, "auto").unwrap(), "1\n2\n3\n");
}

#[test]
fn render_table_matches_python() {
    let values = vec![
        obj(&[("name", s("a")), ("port", Value::Int(80))]),
        obj(&[("name", s("bb")), ("port", Value::Int(443))]),
    ];
    let expected = "+------+------+\n| name | port |\n+------+------+\n| a    | 80   |\n| bb   | 443  |\n+------+------+\n";
    assert_eq!(render(&values, "table").unwrap(), expected);
}

#[test]
fn render_table_lineart_matches_python() {
    let values = vec![obj(&[("k", s("v"))])];
    let expected = "┌───┐\n│ k │\n├───┤\n│ v │\n└───┘\n";
    assert_eq!(render(&values, "table-lineart").unwrap(), expected);
}

#[test]
fn truthiness_and_ordering() {
    assert!(value::truthy(&Value::Int(1)));
    assert!(!value::truthy(&Value::Int(0)));
    assert!(!value::truthy(&s("")));
    assert!(value::truthy(&s("x")));
    // jq ordering: null < false < true < numbers < strings < arrays < objects
    assert_eq!(
        value::sort_cmp(&Value::Null, &Value::Bool(false)),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        value::sort_cmp(&Value::Bool(true), &Value::Int(0)),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        value::sort_cmp(&Value::Int(5), &s("a")),
        std::cmp::Ordering::Less
    );
    // Python ==: 1 == 1.0, True == 1
    assert!(value::py_eq(&Value::Int(1), &Value::Float(1.0)));
    assert!(value::py_eq(&Value::Bool(true), &Value::Int(1)));
    assert!(!value::py_eq(&s("1"), &Value::Int(1)));
}
