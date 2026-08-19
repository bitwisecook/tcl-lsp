// tcl-lsp — TclOO unresolved-receiver provenance diagnostic.
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Companion to the `tcloo_dispatch` resolution-rate experiment.  Where that one
// measures *how many* object-dispatch sites resolve, this one asks *why the rest
// don't*: for every UNRESOLVED `$var method …` receiver, it categorises how the
// receiver variable is bound across the compilation unit.  The category
// histogram tells us which typing edge would move the needle next — the
// experiment that decides what Phase 2/3 should actually build.
//
//   cargo run --release -p tcl-lsp-core --example tcloo_diag -- <dir|file>...
//
// Categories (per receiver variable name, best signal wins):
//   constructor    set v [Class new|create …]        (should already resolve)
//   proc-return    set v [someproc …]                (resolves if proc→object)
//   method-return  set v [$obj m …] / [my m …]       (NOT modelled — candidate)
//   collection-get set v [dict get|lindex|… ]        (collection element typing)
//   alias          set v $other                      (aliasing edge)
//   param          v is a proc/method parameter      (interproc, incl cross-file)
//   cmd-return     set v [othercmd …]                (unmodelled command)
//   literal        set v foo / "…" / {…}             (not an object)
//   unbound        never assigned here               (instance var / global / xfile)

// Coverage-stat arithmetic: counts cast to f64 for percentages and to u32 for
// LSP positions; the magnitudes are tiny, so truncation/precision loss is moot.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::Statement;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_compiler::value_shapes::parse_command_substitution;
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::CommandRegistry;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: tcloo_diag <dir|file>...");
        std::process::exit(2);
    }
    let registry = CommandRegistry::build_default();
    let func_idx = tcl_lsp_core::semantic_tokens::legend_token_types()
        .iter()
        .position(|t| *t == "function")
        .expect("function token type in legend") as u32;
    let dialect = "tcl9.0";

    let mut paths = Vec::new();
    for d in &args {
        collect_tcl(Path::new(d), &mut paths);
    }

    // Category tallies over unresolved receivers, split by receiver form.
    let mut var_cats: HashMap<&'static str, usize> = HashMap::new();
    let mut cmd_cats: HashMap<&'static str, usize> = HashMap::new();
    let mut unresolved_var = 0usize;
    let mut unresolved_cmd = 0usize;
    let mut files = 0usize;
    let mut unbound_names: HashMap<String, usize> = HashMap::new();

    for path in &paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if !src.contains("oo::") && !src.contains("snit::") && !src.contains("itcl::") {
            continue;
        }
        files += 1;
        let cu = CompilationUnit::build_for(&src, &registry, false);
        let analysis = Analyser::new().analyse(&src, dialect);
        let st = tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
            &src,
            tcl_dialect::DialectProfile::by_name(dialect),
            &registry,
            Some(&cu),
            Some(&analysis),
        );
        let kinds = decode_kinds(&st.data);
        let line_starts = line_starts(&src);

        // Var-binding category map over the whole CU (name-keyed, object-
        // insensitive — the same economy the provenance passes use).
        let bindings = binding_categories(&cu, &registry);
        // Parameter names (proc + method).
        let params = param_names(&cu);

        let mut sites = Vec::new();
        collect_sites(&src, 0, dialect, &mut sites, 0);
        for site in sites {
            let (line, col) = byte_to_line_col(&line_starts, &src, site.method_start);
            let resolved = kinds.get(&(line, col)) == Some(&func_idx);
            if resolved {
                continue;
            }
            match site.form {
                "$var" => {
                    unresolved_var += 1;
                    let cat = categorise(&site.receiver, &bindings, &params);
                    *var_cats.entry(cat).or_default() += 1;
                    if cat == "unbound" {
                        *unbound_names.entry(site.receiver.clone()).or_default() += 1;
                    }
                }
                "[cmd]" => {
                    unresolved_cmd += 1;
                    let cat = cmd_head_category(&site.receiver);
                    *cmd_cats.entry(cat).or_default() += 1;
                }
                _ => {}
            }
        }
    }

    println!("# TclOO unresolved-receiver provenance diagnostic");
    println!("files with OO: {files}");
    println!();
    print_hist("$var receivers (unresolved)", unresolved_var, &var_cats);
    println!();
    print_hist("[cmd] receivers (unresolved)", unresolved_cmd, &cmd_cats);

    println!();
    println!("## top 30 unbound $var receiver names");
    println!();
    println!("| name | count |");
    println!("|---|--:|");
    let mut names: Vec<_> = unbound_names.iter().collect();
    names.sort_by_key(|(n, c)| (std::cmp::Reverse(**c), (*n).clone()));
    for (name, c) in names.into_iter().take(30) {
        println!("| `{name}` | {c} |");
    }
}

fn print_hist(title: &str, total: usize, cats: &HashMap<&'static str, usize>) {
    println!("## {title}: {total}");
    println!();
    println!("| category | count | share |");
    println!("|---|--:|--:|");
    let mut rows: Vec<_> = cats.iter().collect();
    rows.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (cat, c) in rows {
        let pct = if total == 0 {
            0.0
        } else {
            *c as f64 / total as f64 * 100.0
        };
        println!("| {cat} | {c} | {pct:.1}% |");
    }
}

/// A dispatch site with its receiver text (variable name for `$var`, inner
/// command head for `[cmd]`).
struct Site {
    form: &'static str,
    receiver: String,
    method_start: u32,
}

/// Categorise an unresolved `$var` receiver by how the variable is bound.
fn categorise(
    recv: &str,
    bindings: &HashMap<String, &'static str>,
    params: &HashSet<String>,
) -> &'static str {
    if let Some(cat) = bindings.get(recv) {
        return cat;
    }
    if params.contains(recv) {
        return "param";
    }
    "unbound"
}

/// Best binding category per variable name across the CU.  When a variable has
/// several bindings the most-informative object-typing signal wins.
fn binding_categories(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<String, &'static str> {
    fn rank(cat: &str) -> u8 {
        match cat {
            "constructor" => 8,
            "proc-return" => 7,
            "method-return" => 6,
            "collection-get" => 5,
            "alias" => 4,
            "cmd-return" => 3,
            "literal" => 1,
            _ => 0,
        }
    }
    // Known proc qualified names, for distinguishing proc-return from cmd-return.
    let procs: HashSet<&str> = cu
        .ir_module
        .procedures
        .values()
        .map(|p| p.qualified_name.as_str())
        .collect();
    let is_proc = |cmd: &str| {
        procs.contains(cmd) || {
            let q = format!("::{}", cmd.trim_start_matches("::"));
            procs.contains(q.as_str())
        }
    };

    let mut out: HashMap<String, &'static str> = HashMap::new();
    let mut consider = |name: &str, cat: &'static str| {
        let e = out.entry(name.to_owned()).or_insert(cat);
        if rank(cat) > rank(e) {
            *e = cat;
        }
    };

    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let Statement::AssignValue { name, value, .. } = stmt else {
                    continue;
                };
                let v = value.trim();
                // Aliasing `set a $b`.
                if v.starts_with('$') && !v.contains(char::is_whitespace) {
                    consider(name, "alias");
                    continue;
                }
                if !v.starts_with('[') {
                    consider(name, "literal");
                    continue;
                }
                let Some((cmd, cargs)) = parse_command_substitution(v) else {
                    consider(name, "cmd-return");
                    continue;
                };
                let verb = cargs.first().map(String::as_str);
                if cmd.starts_with('$') || cmd == "my" {
                    consider(name, "method-return");
                } else if registry.object_class(&cmd).is_some()
                    && (verb == Some("new") || verb == Some("create"))
                {
                    consider(name, "constructor");
                } else if matches!(
                    cmd.as_str(),
                    "dict" | "lindex" | "lassign" | "lrange" | "lsearch"
                ) {
                    consider(name, "collection-get");
                } else if is_proc(&cmd) {
                    consider(name, "proc-return");
                } else {
                    consider(name, "cmd-return");
                }
            }
        }
    }
    out
}

fn param_names(cu: &CompilationUnit) -> HashSet<String> {
    let mut s = HashSet::new();
    for p in cu.ir_module.procedures.values() {
        s.extend(p.params.iter().cloned());
    }
    for m in cu.ir_module.methods.values() {
        s.extend(m.params.iter().cloned());
    }
    s
}

/// Category for a `[cmd] method …` receiver by its inner command head.
fn cmd_head_category(head: &str) -> &'static str {
    match head {
        "dict" | "lindex" | "lassign" | "lrange" | "lsearch" => "collection-get",
        "my" => "method-return",
        "self" => "self",
        _ if head.starts_with('$') => "method-return",
        _ => "cmd-return",
    }
}

fn collect_sites(source: &str, base: u32, dialect: &str, out: &mut Vec<Site>, depth: u32) {
    if depth > 40 {
        return;
    }
    for seg in segment_commands_with_offset_and_config(source, 0, LexerConfig::for_dialect(dialect))
    {
        if let (Some(head), Some(method_tok)) = (seg.argv.first(), seg.argv.get(1))
            && let Some((form, receiver)) = receiver_form(head, &seg)
        {
            out.push(Site {
                form,
                receiver,
                method_start: base + method_tok.span.start(),
            });
        }
        for (i, tok) in seg.argv.iter().enumerate() {
            if !seg.single_token_word.get(i).copied().unwrap_or(false) {
                continue;
            }
            let (co, close) = match tok.kind {
                TokenType::Str => (tok.content_offset as usize, 1usize),
                TokenType::Cmd => (tok.content_offset as usize, 0usize),
                _ => continue,
            };
            let s = tok.span.start() as usize + co;
            let e = (tok.span.end() as usize)
                .saturating_sub(close)
                .min(source.len());
            if e > s
                && let Some(inner) = source.get(s..e)
            {
                collect_sites(
                    inner,
                    base + u32::try_from(s).unwrap_or(0),
                    dialect,
                    out,
                    depth + 1,
                );
            }
        }
    }
}

/// `(form, receiver-text)` for an object-dispatch head, else `None`.
fn receiver_form(
    head: &tcl_lexer::Token,
    seg: &SegmentedCommand,
) -> Option<(&'static str, String)> {
    match head.kind {
        TokenType::Var => {
            // Strip the leading `$` (and `${…}`) to the bare variable name.
            let t = seg.texts.first()?.trim();
            let name = t.strip_prefix('$').map_or(t, |r| {
                r.strip_prefix('{')
                    .and_then(|x| x.strip_suffix('}'))
                    .unwrap_or(r)
            });
            Some(("$var", name.to_owned()))
        }
        TokenType::Cmd => {
            // Inner command head of the `[…]` receiver.
            let text = seg.texts.first()?;
            let (cmd, _) = parse_command_substitution(text)?;
            Some(("[cmd]", cmd))
        }
        TokenType::Esc if seg.texts.first().map(String::as_str) == Some("my") => {
            Some(("my", "my".to_owned()))
        }
        _ => None,
    }
}

fn decode_kinds(data: &[u32]) -> HashMap<(u32, u32), u32> {
    let mut out = HashMap::new();
    let (mut line, mut col) = (0u32, 0u32);
    for c in data.chunks(5) {
        let [dl, dc, _len, kind, _mods] = *c else {
            break;
        };
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.insert((line, col), kind);
    }
    out
}

fn line_starts(src: &str) -> Vec<usize> {
    let mut v = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

fn byte_to_line_col(line_starts: &[usize], src: &str, byte: u32) -> (u32, u32) {
    let byte = byte as usize;
    let line = line_starts
        .partition_point(|&s| s <= byte)
        .saturating_sub(1);
    let col = src
        .get(line_starts[line]..byte.min(src.len()))
        .map_or(0, |s| s.chars().map(|c| c.len_utf16() as u32).sum());
    (line as u32, col)
}

fn collect_tcl(p: &Path, out: &mut Vec<std::path::PathBuf>) {
    if p.is_dir() {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                collect_tcl(&e.path(), out);
            }
        }
    } else if p.extension().is_some_and(|x| x == "tcl") {
        out.push(p.to_path_buf());
    }
}
