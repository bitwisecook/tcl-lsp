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

//! What a `SpecTcl`-authored surface costs against a compiled-in one, per
//! phase: seeding the Jim core surface, loading the bundled packs, building
//! a thread's hook host, selecting a document's registry, and resolving a
//! command head — known and unknown — under `jim` and `tcl8.6`.
//!
//! `cargo run --profile ci -p tcl-spectcl --example surface_costs`

use std::hint::black_box;
use std::time::{Duration, Instant};

const RESOLVE_ROUNDS: usize = 200;
const SELECT_ROUNDS: usize = 2_000;

fn resident_kib() -> usize {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|statm| statm.split_whitespace().nth(1)?.parse::<usize>().ok())
        .map_or(0, |pages| pages * 4)
}

fn timed<T>(label: &str, work: impl FnOnce() -> T) -> T {
    let before = resident_kib();
    let start = Instant::now();
    let out = work();
    let elapsed = start.elapsed();
    let grown = resident_kib().saturating_sub(before);
    println!("{label:<44} {:>10.2} ms {grown:>8} KiB", ms(elapsed));
    out
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn per_call_ns(duration: Duration, calls: usize) -> f64 {
    duration.as_secs_f64() * 1e9 / f64::from(u32::try_from(calls).unwrap_or(u32::MAX))
}

fn resolve_costs(dialect: &str) {
    let environment = tcl_lsp_core::environment_for_dialect(dialect);
    let context = environment.default_context_registry();
    let store = tcl_spectcl::bundled::registry_for_dialect(dialect);
    let names = context.visible_command_names();
    let unknown: Vec<String> = (0..names.len()).map(|i| format!("user_proc_{i}")).collect();

    let start = Instant::now();
    for _ in 0..RESOLVE_ROUNDS {
        for name in &names {
            black_box(context.context().resolve_spec(&store, black_box(name)));
        }
    }
    let known = start.elapsed();

    let start = Instant::now();
    for _ in 0..RESOLVE_ROUNDS {
        for name in &unknown {
            black_box(context.context().resolve_spec(&store, black_box(name)));
        }
    }
    let missing = start.elapsed();

    let start = Instant::now();
    for _ in 0..RESOLVE_ROUNDS {
        for name in &names {
            black_box(context.resolve_command(black_box(name)));
        }
    }
    let assembled = start.elapsed();

    let start = Instant::now();
    for _ in 0..SELECT_ROUNDS {
        black_box(
            tcl_lsp_core::environment_for_dialect(black_box(dialect)).default_context_registry(),
        );
    }
    let select = start.elapsed();

    let start = Instant::now();
    for _ in 0..SELECT_ROUNDS {
        black_box(tcl_spectcl::bundled::registry_for_dialect(black_box(
            dialect,
        )));
    }
    let overlay = start.elapsed();

    let calls = RESOLVE_ROUNDS * names.len();
    println!(
        "{dialect:<8} names {:>5}  resolve_spec known {:>7.1} ns  unknown {:>7.1} ns  \
         resolve_command {:>7.1} ns  select ctx {:>8.1} ns  select overlay {:>8.1} ns",
        names.len(),
        per_call_ns(known, calls),
        per_call_ns(missing, calls),
        per_call_ns(assembled, calls),
        per_call_ns(select, SELECT_ROUNDS),
        per_call_ns(overlay, SELECT_ROUNDS),
    );
}

/// The native surfaces against their `SpecTcl` renderings
/// (`equivalence/`, `cargo xtask gen-equivalence-packs`): the same commands,
/// compiled in versus evaluated.
fn equivalence_costs() {
    timed("native: CommandRegistry::build_default", || {
        black_box(tcl_registry::registry::CommandRegistry::build_default());
    });
    for (dialect, file) in [
        ("tcl8.6", "equivalent_tcl8_6.tclspec"),
        ("f5-irules", "equivalent_f5_irules.tclspec"),
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("equivalence")
            .join(file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            println!("{file}: missing — run `cargo xtask gen-equivalence-packs`");
            continue;
        };
        let native = tcl_lsp_core::environment_for_dialect(dialect)
            .default_context_registry()
            .visible_command_names()
            .len();
        let pack = timed(&format!("SpecTcl {dialect}: evaluate pack"), || {
            tcl_spectcl::evaluate_pack(&source)
        });
        if pack.commands.len() != native {
            println!(
                "  stale: {} pack commands against {native} native — run `cargo xtask \
                 gen-equivalence-packs`",
                pack.commands.len()
            );
        }
    }
}

fn main() {
    tcl_spectcl::demand::enable();
    println!("{:<44} {:>13} {:>12}", "phase", "wall", "rss growth");
    timed(
        "core_surfaces::ensure (Jim seed)",
        tcl_spectcl::core_surfaces::ensure,
    );
    timed("bundled::load_embedded (no demand: manifests)", || {
        black_box(tcl_spectcl::bundled::load_embedded());
    });
    timed("bundled::packs (process-wide set)", || {
        black_box(tcl_spectcl::bundled::packs());
    });
    for dialect in ["tcl8.6", "jim"] {
        timed(&format!("first registry_for_dialect({dialect})"), || {
            black_box(tcl_spectcl::bundled::registry_for_dialect(dialect));
        });
    }
    timed("demand: first xilinx-eda-tcl registry", || {
        black_box(tcl_spectcl::bundled::registry_for_dialect("xilinx-eda-tcl"));
    });
    let packs = tcl_spectcl::bundled::packs();
    timed("hooks::publish (UPF now loaded)", || {
        tcl_spectcl::hooks::publish(packs);
    });
    timed("evaluator_generation on a fresh thread", || {
        std::thread::spawn(|| black_box(tcl_registry::pack_hooks::evaluator_generation()))
            .join()
            .expect("generation thread");
    });
    for n in 1..=2 {
        timed(&format!("ensure_thread_host on fresh thread #{n}"), || {
            std::thread::spawn(tcl_spectcl::hooks::ensure_thread_host)
                .join()
                .expect("hook host thread");
        });
    }
    timed(
        "ensure_thread_host warm (main thread)",
        tcl_spectcl::hooks::ensure_thread_host,
    );
    for dialect in ["tcl8.6", "jim"] {
        resolve_costs(dialect);
    }
    equivalence_costs();
    println!("final rss {} KiB", resident_kib());
}
