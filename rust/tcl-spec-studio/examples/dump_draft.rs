fn main() {
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap();
    let dialect = args.next().unwrap_or_else(|| "tcl9.1".to_string());
    let v = tcl_spec_studio::load_command(&name, &dialect).expect("command");
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}
