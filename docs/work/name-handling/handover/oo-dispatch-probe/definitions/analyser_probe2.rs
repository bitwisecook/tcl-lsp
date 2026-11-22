
#[test]
#[ignore = "probe"]
fn oo_body_probe() {
    use std::collections::BTreeMap;
    let input = std::fs::read_to_string(std::env::var("ANALYSE_IN").unwrap()).unwrap();
    let mut groups: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
    for line in input.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 { continue; }
        groups.entry((f[0].to_string(), f[1].to_string())).or_default().push((f[2].to_string(), f[3].to_string()));
    }
    let mut out = String::new();
    let keep = ["E001", "E002", "E003", "E005", "W001", "W002", "W123", "W004"];
    for ((dialect, cmd), rows) in groups {
        let mut src = String::new();
        let mut first = 0usize;
        let ns = cmd.trim_start_matches("::").split("::").next().unwrap_or("").to_string();
        if cmd.trim_start_matches("::").contains("::") && ["http", "msgcat", "tcltest", "platform", "opt"].contains(&ns.as_str()) {
            src.push_str(&format!("package require {ns}\n"));
            first = 1;
        }
        if cmd.starts_with("::platform::shell") { src = "package require platform::shell\n".into(); first = 1; }
        let prefix = src.clone();
        for (sub, n) in &rows {
            let nn: usize = n.parse().unwrap_or(0);
            let mut words = vec![cmd.clone()];
            if !sub.is_empty() { words.push(sub.clone()); }
            for _ in 0..nn { words.push("__x__".into()); }
            let line_src = format!("{prefix}{}\n", words.join(" "));
            let analysis = Analyser::new().analyse(&line_src, &dialect);
            let mut c: Vec<String> = Vec::new();
            for d in &analysis.diagnostics {
                let code = format!("{:?}", d.code);
                if !keep.contains(&code.as_str()) { continue; }
                let line = line_src[..d.span.as_range().start.min(line_src.len())].matches('\n').count();
                if line == first && !c.contains(&code) { c.push(code); }
            }
            c.sort();
            out.push_str(&format!("{dialect}\t{cmd}\t{sub}\t{n}\t{}\n", c.join(",")));
        }
    }
    std::fs::write(std::env::var("ANALYSE_OUT").unwrap(), out).unwrap();
}
