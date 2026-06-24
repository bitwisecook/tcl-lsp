//! BPF-Tcl CLI: compile / run / check `.bpftcl` programs.
//!
//! `compile` emits eBPF directly (`--emit asm|hex|raw`); `run` executes a
//! program under the userspace `rbpf` harness over a synthetic packet; `check`
//! type-checks the DSL subset and renders diagnostics with file positions.
#![forbid(unsafe_code)]

pub mod run;

use std::fmt::Write as _;
use std::process::ExitCode;

use bpf_tcl_codegen::ebpf::{EbpfObject, disasm, emit_program, write_object};
use bpf_tcl_ir::{BpfError, BpfProgramDecl, compile_module};
use tcl_lexer::SourceMap;

/// CLI entry point.
#[must_use]
pub fn run(args: impl Iterator<Item = String>) -> ExitCode {
    let mut args = args;
    let _argv0 = args.next();
    let cmd = args.next();
    let rest: Vec<String> = args.collect();
    match cmd.as_deref() {
        Some("check") => cmd_check(&rest),
        Some("compile") => cmd_compile(&rest),
        Some("run") => cmd_run(&rest),
        Some("help" | "-h" | "--help") => {
            usage();
            ExitCode::SUCCESS
        }
        None => {
            usage();
            ExitCode::FAILURE
        }
        Some(other) => {
            eprintln!("bpf-tcl: unknown command `{other}`");
            usage();
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "usage:\n  \
         bpf-tcl check   <file.bpftcl>\n  \
         bpf-tcl compile <file.bpftcl> [--emit asm|hex|raw|elf] [--program N] [-o OUT]\n  \
         bpf-tcl run     <file.bpftcl> --packet <HEX> [--program N]"
    );
}

fn read_source(path: &str) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("bpf-tcl: cannot read {path}: {e}");
        ExitCode::FAILURE
    })
}

fn print_error(path: &str, src: &str, e: &BpfError) {
    let sm = SourceMap::new(src);
    let pos = sm.position_at(e.span.start());
    eprintln!(
        "{path}:{}:{}: error[{}]: {}",
        pos.line + 1,
        pos.character.get() + 1,
        e.code.code(),
        e.msg
    );
}

fn cmd_check(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("check: missing <file.bpftcl>");
        return ExitCode::FAILURE;
    };
    let src = match read_source(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_module(&src) {
        Ok(m) => {
            println!("ok: {} program(s)", m.programs.len());
            for d in &m.programs {
                let attach = d
                    .attach
                    .as_ref()
                    .map_or_else(String::new, |a| format!(", attach {} {}", a.kind, a.target));
                println!(
                    "  when {} (priority {}, {} slots, {} blocks{attach})",
                    d.event,
                    d.priority,
                    d.program.num_slots,
                    d.program.blocks.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            print_error(path, &src, &e);
            ExitCode::FAILURE
        }
    }
}

fn cmd_compile(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut emit = "asm".to_string();
    let mut program: Option<usize> = None;
    let mut out: Option<String> = None;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--emit" => emit = it.next().cloned().unwrap_or_default(),
            "--program" => program = it.next().and_then(|s| s.parse().ok()),
            "-o" => out = it.next().cloned(),
            other => {
                if path.is_none() {
                    path = Some(other.to_string());
                } else {
                    eprintln!("compile: unexpected argument `{other}`");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let Some(path) = path else {
        eprintln!("compile: missing <file.bpftcl>");
        return ExitCode::FAILURE;
    };
    let src = match read_source(&path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let module = match compile_module(&src) {
        Ok(m) => m,
        Err(e) => {
            print_error(&path, &src, &e);
            return ExitCode::FAILURE;
        }
    };

    let selected: Vec<(usize, _)> = match program {
        Some(i) => {
            let Some(d) = module.programs.get(i) else {
                eprintln!("compile: no program #{i}");
                return ExitCode::FAILURE;
            };
            vec![(i, d)]
        }
        None => module.programs.iter().enumerate().collect(),
    };
    if selected.is_empty() {
        eprintln!("compile: no programs found");
        return ExitCode::FAILURE;
    }
    if emit == "elf" && selected.len() > 1 {
        eprintln!("compile: --emit elf needs --program N (one program per object)");
        return ExitCode::FAILURE;
    }

    for (i, d) in selected {
        let obj = match emit_program(&d.program) {
            Ok(o) => o,
            Err(e) => {
                print_error(&path, &src, &e);
                return ExitCode::FAILURE;
            }
        };
        if let Err(msg) = emit_object(&emit, i, d, &obj, out.as_deref()) {
            eprintln!("compile: {msg}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn cmd_run(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut packet_hex: Option<String> = None;
    let mut program = 0usize;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--packet" => packet_hex = it.next().cloned(),
            "--program" => program = it.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            other => {
                if path.is_none() {
                    path = Some(other.to_string());
                } else {
                    eprintln!("run: unexpected argument `{other}`");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let Some(path) = path else {
        eprintln!("run: missing <file.bpftcl>");
        return ExitCode::FAILURE;
    };
    let src = match read_source(&path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let module = match compile_module(&src) {
        Ok(m) => m,
        Err(e) => {
            print_error(&path, &src, &e);
            return ExitCode::FAILURE;
        }
    };
    let Some(decl) = module.programs.get(program) else {
        eprintln!("run: no program #{program}");
        return ExitCode::FAILURE;
    };
    let obj = match emit_program(&decl.program) {
        Ok(o) => o,
        Err(e) => {
            print_error(&path, &src, &e);
            return ExitCode::FAILURE;
        }
    };

    let mut packet = match &packet_hex {
        Some(h) => {
            let Some(p) = parse_hex(h) else {
                eprintln!("run: invalid --packet hex");
                return ExitCode::FAILURE;
            };
            p
        }
        None => Vec::new(),
    };

    match run::run_socket_filter(&obj, &mut packet) {
        Ok(v) => {
            // A socket-filter verdict is the low 32 bits of r0 (bytes to accept).
            let verdict = u32::try_from(v & 0xffff_ffff).unwrap_or(u32::MAX);
            if verdict == 0 {
                println!("drop (r0=0)");
            } else {
                println!("accept {verdict} bytes (r0={v})");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("run: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Render one compiled program in the requested `--emit` format.
fn emit_object(
    emit: &str,
    idx: usize,
    decl: &BpfProgramDecl,
    obj: &EbpfObject,
    out: Option<&str>,
) -> Result<(), String> {
    match emit {
        "asm" => {
            println!(
                "// program #{idx}: when {} priority {} ({} insns)",
                decl.event,
                decl.priority,
                obj.insns.len()
            );
            print!("{}", disasm(&obj.insns));
        }
        "hex" => println!("{}", to_hex(&obj.raw)),
        "raw" => write_bytes(&obj.raw, out)?,
        "elf" => {
            let bytes = write_object(obj, &elf_symbol(&decl.event)).map_err(|e| e.to_string())?;
            write_bytes(&bytes, out)?;
        }
        other => return Err(format!("unknown --emit `{other}` (want asm|hex|raw|elf)")),
    }
    Ok(())
}

/// Write bytes to `out` (a path) or stdout.
fn write_bytes(bytes: &[u8], out: Option<&str>) -> Result<(), String> {
    if let Some(o) = out {
        std::fs::write(o, bytes).map_err(|e| format!("cannot write {o}: {e}"))?;
    } else {
        use std::io::Write;
        let _ = std::io::stdout().write_all(bytes);
    }
    Ok(())
}

/// The exported program symbol name for an event (the `STT_FUNC` in the ELF).
fn elf_symbol(event: &str) -> String {
    event.to_ascii_lowercase()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return None;
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).ok())
        .collect()
}
