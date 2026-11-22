
#[test]
#[ignore = "probe"]
fn oo_body_probe() {
    let input = std::fs::read_to_string(std::env::var("CASES").unwrap()).unwrap();
    for line in input.lines() {
        let (dialect, src) = line.split_once('\t').unwrap();
        let src = src.replace("; ", "\n");
        let a = Analyser::new().analyse(&src, dialect);
        let codes: Vec<String> = a.diagnostics.iter().filter(|d| { let c = format!("{:?}", d.code); c.starts_with('E') || c == "W001" || c == "W002" || c == "W123" }).map(|d| format!("{:?}:{}", d.code, d.message.chars().take(60).collect::<String>())).collect();
        println!("VC {dialect} {} => {:?}", src.replace('\n', "; "), codes);
    }
}
