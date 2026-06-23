//! Parse a BIG-IP `.conf` and print its canonical JSON document.
//!
//! Used by the differential-parity harness: `dump_canonical <path>
//! [partition]` prints the JSON that `_rust_bridge.rebuild`
//! reconstructs the dataclasses from.

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: dump_canonical <path> [partition]");
    let partition = args.next().unwrap_or_else(|| "Common".to_owned());
    let src = std::fs::read_to_string(&path).expect("read source");
    let config = tcl_bigip::parser::parse_bigip_conf(&src, &partition);
    let json = tcl_bigip::canonical::config_to_canonical(&config);
    println!("{}", serde_json::to_string(&json).expect("serialise"));
}
