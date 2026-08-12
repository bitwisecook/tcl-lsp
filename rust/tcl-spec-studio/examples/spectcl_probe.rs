//! Throwaway probe used while building the SpecTcl renderer. Not committed.

use std::collections::BTreeMap;

use serde_json::Value;
use tcl_spec_studio::{BROWSABLE_DIALECTS, command_names, draft, load_command, schema};

fn brace_safe(text: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                None | Some('\n') => return false,
                Some(_) => {}
            },
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let what = args.next().unwrap_or_else(|| "defaults".to_owned());
    match what.as_str() {
        "defaults" => {
            let d = draft::default_command_draft();
            println!("{}", serde_json::to_string_pretty(&Value::Object(d)).unwrap());
            let s = draft::default_subcommand_draft();
            println!("--- SUB ---");
            println!("{}", serde_json::to_string_pretty(&Value::Object(s)).unwrap());
        }
        "unrenderable" => {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut total = 0usize;
            for (dialect, _) in BROWSABLE_DIALECTS {
                for name in command_names(dialect) {
                    let Some(v) = load_command(name, dialect) else {
                        continue;
                    };
                    total += 1;
                    for key in v[draft::UNRENDERABLE_KEY].as_array().unwrap_or(&vec![]) {
                        *counts.entry(key.as_str().unwrap_or("?").to_owned()).or_default() += 1;
                    }
                }
            }
            println!("total {total}");
            for (k, n) in counts {
                println!("{n:6}  {k}");
            }
        }
        "nondefault" => {
            // Which non-default keys occur, and how often.
            let base = Value::Object(draft::default_command_draft());
            let subbase = Value::Object(draft::default_subcommand_draft());
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut subcounts: BTreeMap<String, usize> = BTreeMap::new();
            for (dialect, _) in BROWSABLE_DIALECTS {
                for name in command_names(dialect) {
                    let Some(v) = load_command(name, dialect) else {
                        continue;
                    };
                    for f in schema::COMMAND_FIELDS {
                        if v.get(f.key) != base.get(f.key) {
                            *counts.entry(f.key.to_owned()).or_default() += 1;
                        }
                    }
                    for sub in v["subcommands"].as_array().unwrap_or(&vec![]) {
                        for f in schema::SUBCOMMAND_FIELDS {
                            if sub.get(f.key) != subbase.get(f.key) {
                                *subcounts.entry(f.key.to_owned()).or_default() += 1;
                            }
                        }
                    }
                }
            }
            println!("== command");
            for (k, n) in counts {
                println!("{n:6}  {k}");
            }
            println!("== subcommand");
            for (k, n) in subcounts {
                println!("{n:6}  {k}");
            }
        }
        "dump" => {
            let name = args.next().unwrap();
            let dialect = args.next().unwrap_or_else(|| "tcl9.1".to_owned());
            let v = load_command(&name, &dialect).unwrap();
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
        "prefixnorole" => {
            for (dialect, _) in BROWSABLE_DIALECTS {
                for name in command_names(dialect) {
                    let Some(v) = load_command(name, dialect) else {
                        continue;
                    };
                    let roles: Vec<u64> = v["arg_roles"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|e| e["index"].as_u64().unwrap())
                        .collect();
                    let mut sorted = roles.clone();
                    sorted.sort_unstable();
                    if roles != sorted {
                        println!("{dialect}/{name}: arg_roles not sorted {roles:?}");
                    }
                    for p in v["command_prefixes"].as_array().unwrap() {
                        let i = p["index"].as_u64().unwrap();
                        if !roles.contains(&i) {
                            println!("{dialect}/{name}: prefix at {i} with no role");
                        }
                    }
                }
            }
        }
        "rt" => {
            let name = args.next().unwrap();
            let dialect = args.next().unwrap_or_else(|| "tcl9.1".to_owned());
            let v = load_command(&name, &dialect).unwrap();
            let d = v.as_object().unwrap().clone();
            let text = tcl_spec_studio::render_spectcl::render(&d);
            println!("{text}");
            let pack = tcl_spec_studio::spectcl::load_pack(&text);
            for n in &pack.notices {
                println!("NOTICE {n}");
            }
            let back = pack.command(&name).map(|c| Value::Object(draft::from_command_spec(c.spec)));
            let Some(back) = back else { println!("NOT LOADED"); return };
            for f in schema::COMMAND_FIELDS {
                if back.get(f.key) != v.get(f.key) {
                    println!("DIFF {}\n   rendered: {}\n   shipped : {}", f.key, back[f.key], v[f.key]);
                }
            }
            if back[draft::UNRENDERABLE_KEY] != v[draft::UNRENDERABLE_KEY] {
                println!("DIFF __unrenderable\n   rendered: {}\n   shipped : {}", back[draft::UNRENDERABLE_KEY], v[draft::UNRENDERABLE_KEY]);
            }
        }
        "quote" => {
            let src = "speclib p 1.0 {\ncommand demo {\n  subcommand a { detail \"q \\[b\\] $c and \\{ brace\" }\n  subcommand b { detail {braced \\[b\\] $c} }\n  subcommand c { detail \"tail\" }\n}\n}\n";
            println!("SOURCE:\n{src}");
            let pack = tcl_spec_studio::spectcl::load_pack(src);
            for n in &pack.notices { println!("NOTICE {n}"); }
            for c in &pack.commands {
                for s in c.spec.subcommands {
                    println!("SUB {} detail={:?}", s.name, s.detail);
                }
            }
        }
        "unwritable" => {
            // Which prose strings no Tcl word can carry, and what they look like.
            for (dialect, _) in BROWSABLE_DIALECTS.iter() {
                for name in command_names(dialect) {
                    let Some(v) = load_command(name, dialect) else { continue };
                    let d = v.as_object().unwrap().clone();
                    let (_, losses) = tcl_spec_studio::render_spectcl::render_pack_reporting(&[d], "p");
                    for l in &losses {
                        if l.reason.contains("unbalanced") {
                            println!("{dialect}/{name}: {}", l.key);
                            if let Some(h) = v["hover"].as_object() {
                                for (k, hv) in h {
                                    if let Some(t) = hv.as_str() {
                                        if !brace_safe(t) {
                                            println!("    {k}: {:?}", &t.chars().take(160).collect::<String>());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        other => println!("unknown probe {other}"),
    }
}
