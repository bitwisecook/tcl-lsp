
#[test]
#[ignore = "probe"]
fn oo_body_probe() {
    let dirs = std::env::var("PROBE_DIRS").unwrap();
    let out_dir = std::env::var("PROBE_OUT").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut report = String::new();
    for dir in dirs.split(':') {
        let area = std::path::Path::new(dir).file_name().unwrap().to_string_lossy().to_string();
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("tcl") && !p.to_string_lossy().contains(".optimised.")).collect();
        files.sort();
        std::fs::create_dir_all(format!("{out_dir}/{area}")).unwrap();
        for path in files {
            let Ok(source) = std::fs::read_to_string(&path) else { continue };
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            for rel in ["tcl8.6", "tcl9.0"] {
                let a = Analyser::new().analyse(&source, rel);
                let mut codes: Vec<String> = a.diagnostics.iter().map(|d| {
                    let line = source[..d.span.as_range().start.min(source.len())].matches('\n').count() + 1;
                    format!("{:?}@{line}:{}", d.code, d.message.chars().take(50).collect::<String>().replace('\t', " "))
                }).collect();
                codes.sort();
                report.push_str(&format!("D\t{area}\t{name}\t{rel}\t{}\n", codes.join(" | ")));
                let profile = tcl_dialect::DialectProfile::find(rel).unwrap();
                let opts = crate::optimiser::optimise_with_dialect(&source, &registry, Some(profile));
                let mut spans: Vec<(usize, usize, String, String)> = opts.iter().filter(|o| !o.hint_only).map(|o| (o.span.as_range().start, o.span.as_range().end, o.replacement.clone(), format!("{:?}", o.code))).collect();
                spans.sort_by(|a, b| b.0.cmp(&a.0));
                let mut edited = source.clone();
                let mut last = usize::MAX;
                let mut applied = Vec::new();
                for (s, e, t, c) in &spans { if *e <= last && *e <= edited.len() { edited.replace_range(*s..*e, t); last = *s; applied.push(c.clone()); } }
                if !applied.is_empty() {
                    std::fs::write(format!("{out_dir}/{area}/{rel}-{name}"), &edited).unwrap();
                }
                report.push_str(&format!("O\t{area}\t{name}\t{rel}\t{}\n", applied.join(",")));
            }
        }
    }
    std::fs::write(format!("{out_dir}/report.tsv"), report).unwrap();
}
