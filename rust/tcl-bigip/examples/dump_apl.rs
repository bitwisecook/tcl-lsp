//! Parse an F5 iApp APL file and print its canonical JSON document.
//!
//! Used by the differential-parity harness: `dump_apl <path>` prints the
//! JSON that `_rust_bridge.rebuild_apl` reconstructs the
//! `AplModel` from.

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_apl <path>");
    let src = std::fs::read_to_string(&path).expect("read source");
    let model = tcl_bigip::apl::parse_apl(&src);
    let json = tcl_bigip::apl::model_to_canonical(&model);
    println!("{}", serde_json::to_string(&json).expect("serialise"));
}
