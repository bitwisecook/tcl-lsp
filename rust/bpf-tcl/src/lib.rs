// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! BPF-Tcl CLI: compile / run / check `.bpftcl` programs.
//!
//! `compile` emits eBPF directly (`--emit asm|hex|raw`); `run` executes a
//! program under the userspace `rbpf` harness over a synthetic packet; `check`
//! type-checks the DSL subset and renders diagnostics with file positions.
#![forbid(unsafe_code)]

pub mod run;

use std::fmt::Write as _;
use std::process::ExitCode;

use bpf_tcl_codegen::ebpf::{
    EbpfObject, TargetAbi, disasm, emit_program, emit_program_for_target, write_object,
};
use bpf_tcl_ir::{
    BpfError, BpfModule, BpfProgramDecl, DeploymentPlan, ProgType, compile_module, event_chains,
};
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
        Some("plan") => cmd_plan(&rest),
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
         bpf-tcl compile <file.bpftcl> [--target rbpf|kernel-xdp|kernel-socket] [--emit asm|hex|raw|elf] [--program N] [-o OUT]\n  \
         bpf-tcl run     <file.bpftcl> --packet <HEX> [--program N] [--repeat N]\n  \
         bpf-tcl plan    <file.bpftcl> [--name NAME]   (loader dry-run: programs, pins, attach targets, kernel features)"
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
                // Show slot reuse when the liveness allocator saved any: N
                // physical slots reused from M computed values.
                let reuse = if d.program.raw_slot_count > d.program.num_slots {
                    format!(" (reused from {})", d.program.raw_slot_count)
                } else {
                    String::new()
                };
                println!(
                    "  when {} (priority {}, {} slots{reuse}, {} bytes stack, {} blocks{attach})",
                    d.event,
                    d.priority,
                    d.program.num_slots,
                    d.program.num_slots * 8,
                    d.program.blocks.len()
                );
                if !d.program.maps.is_empty() {
                    for m in &d.program.maps {
                        println!(
                            "    map {} ({} key={}B val={}B max={} {:?})",
                            m.name,
                            m.kind.as_str(),
                            m.key_size,
                            m.value_size,
                            m.max_entries,
                            m.concurrency
                        );
                    }
                }
                // The resolved event contract (registry-described schema).
                if let Some(spec) = bpf_tcl_ir::event::event_spec(&d.event) {
                    println!(
                        "    event contract: ctx={} caps=[{}] default={} output={} kernel={}",
                        spec.context.struct_name,
                        spec.capability_names().join(","),
                        spec.default_verdict.verb(),
                        spec.output_label(),
                        spec.kernel_summary(),
                    );
                }
            }
            print_composition(&m);
            ExitCode::SUCCESS
        }
        Err(e) => {
            print_error(path, &src, &e);
            ExitCode::FAILURE
        }
    }
}

/// Print the resolved handler-composition chains: per event, the handlers in
/// ascending-priority (run-first) order.
fn print_composition(module: &BpfModule) {
    let chains = event_chains(module);
    let has_multi = chains.iter().any(|c| c.len() > 1);
    if !has_multi {
        return;
    }
    println!("composition:");
    for chain in chains {
        if chain.len() < 2 {
            continue;
        }
        let order = chain
            .priorities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" -> ");
        println!(
            "  {} chain ({} handlers, priority {order}; lower runs first, terminal verdict stops)",
            chain.event,
            chain.len()
        );
    }
}

fn cmd_plan(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut name = "bpftcl".to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--name" => {
                let Some(value) = it.next() else {
                    eprintln!("plan: --name expects a deployment name");
                    return ExitCode::FAILURE;
                };
                name.clone_from(value);
            }
            other => {
                if path.is_none() {
                    path = Some(other.to_string());
                } else {
                    eprintln!("plan: unexpected argument `{other}`");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let Some(path) = path else {
        eprintln!("plan: missing <file.bpftcl>");
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
    let plan = DeploymentPlan::from_module(&name, &module);
    println!(
        "deployment `{}` (dry-run; no kernel state is touched)",
        plan.name
    );
    println!("{} program(s):", plan.programs.len());
    for (i, prog) in plan.programs.iter().enumerate() {
        let attach = prog
            .attach
            .as_ref()
            .map_or_else(|| "none".to_owned(), |a| format!("{} {}", a.kind, a.target));
        let kernel = bpf_tcl_ir::event::event_spec(&prog.event)
            .map_or_else(String::new, |s| format!(", kernel {}", s.kernel_summary()));
        println!(
            "  #{i} {} (priority {}, {:?}), attach {attach}{kernel}",
            prog.event, prog.priority, prog.prog_type
        );
        println!("     pin: {}", prog.pin);
        let maps = &module.programs[i].program.maps;
        for map in maps {
            println!(
                "     map {} ({} key={}B val={}B max={})",
                map.name,
                map.kind.as_str(),
                map.key_size,
                map.value_size,
                map.max_entries
            );
        }
    }
    println!(
        "next: `load` (verifier-load, no attach) → `test-run` → `attach` (create links/pins) \
         → `status` → `detach`. These require a privileged loader on a live kernel; the lifecycle \
         state machine is modelled and tested in `bpf-tcl-ir::loader`."
    );
    ExitCode::SUCCESS
}

fn cmd_compile(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut emit = "asm".to_string();
    let mut program: Option<usize> = None;
    let mut out: Option<String> = None;
    let mut target_abi = TargetAbi::RbpfFixedMbuff;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--emit" => emit = it.next().cloned().unwrap_or_default(),
            "--target" => {
                let Some(value) = it.next() else {
                    eprintln!("compile: --target expects rbpf or kernel-xdp");
                    return ExitCode::FAILURE;
                };
                target_abi = match value.as_str() {
                    "rbpf" => TargetAbi::RbpfFixedMbuff,
                    "kernel-xdp" => TargetAbi::KernelXdp,
                    "kernel-socket" | "kernel-socket-filter" => TargetAbi::KernelSocketFilter,
                    other => {
                        eprintln!(
                            "compile: unknown --target `{other}` (want rbpf|kernel-xdp|kernel-socket)"
                        );
                        return ExitCode::FAILURE;
                    }
                };
            }
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
        let obj = match emit_program_for_target(&d.program, target_abi) {
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
    let mut repeat = 1usize;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--packet" => packet_hex = it.next().cloned(),
            "--program" => program = it.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            "--repeat" => {
                let Some(value) = it.next().and_then(|s| s.parse::<usize>().ok()) else {
                    eprintln!("run: --repeat expects a positive integer");
                    return ExitCode::FAILURE;
                };
                if value == 0 {
                    eprintln!("run: --repeat expects a positive integer");
                    return ExitCode::FAILURE;
                }
                repeat = value;
            }
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

    match run::run_socket_filter_repeated(&obj, &mut packet, repeat) {
        Ok((verdict, maps)) => {
            println!("{}", format_verdict(obj.prog_type, verdict));
            for (def, values) in obj.maps.iter().zip(maps) {
                let mut entries: Vec<(u64, u64)> = values.into_iter().collect();
                entries.sort_unstable_by_key(|(key, _)| *key);
                if entries.is_empty() {
                    println!("map {}: <empty>", def.name);
                } else {
                    let rendered = entries
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("map {}: {rendered}", def.name);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("run: {e}");
            ExitCode::FAILURE
        }
    }
}

fn format_verdict(prog_type: ProgType, value: u64) -> String {
    let verdict = u32::try_from(value & 0xffff_ffff).unwrap_or(u32::MAX);
    match prog_type {
        ProgType::SocketFilter if verdict == 0 => "drop (r0=0)".to_owned(),
        ProgType::SocketFilter => format!("accept {verdict} bytes (r0={value})"),
        ProgType::Xdp => match verdict {
            0 => "XDP_ABORTED (r0=0)".to_owned(),
            1 => "XDP_DROP (r0=1)".to_owned(),
            2 => "XDP_PASS (r0=2)".to_owned(),
            3 => "XDP_TX (r0=3)".to_owned(),
            4 => "XDP_REDIRECT (r0=4)".to_owned(),
            _ => format!("unknown XDP verdict {verdict} (r0={value})"),
        },
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
                "// program #{idx}: when {} priority {} (target {}, {} insns)",
                decl.event,
                decl.priority,
                obj.target_abi.as_str(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_format_respects_program_type() {
        assert_eq!(
            format_verdict(ProgType::SocketFilter, 2),
            "accept 2 bytes (r0=2)"
        );
        assert_eq!(format_verdict(ProgType::Xdp, 1), "XDP_DROP (r0=1)");
        assert_eq!(format_verdict(ProgType::Xdp, 2), "XDP_PASS (r0=2)");
        assert_eq!(format_verdict(ProgType::Xdp, 3), "XDP_TX (r0=3)");
    }
}
