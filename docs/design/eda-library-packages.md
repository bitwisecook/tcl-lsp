# EDA tools as library packages (not dialects)

Status: proposed (design). Supersedes the 5 EDA *vendor-bit* dialects with a
base-Tcl-version dialect + `required_package`-gated command libraries.

> **Where the packs live now.** The model below is unchanged — an EDA shell is
> a base Tcl version plus `sdc_base` and its vendor's library, gated by
> `required_package` — but the libraries are no longer Rust. `sdc_base` and the
> five vendor packs (346 commands) are **bundled `.tclspec` loadables** under
> `specs/`, shipped beside the server executable and read by the `SpecTcl`
> loader (`docs/design/spec-packs.md`: "the EDA vendor libraries ship as
> bundled `.tclspec` loadables … so the loader path is exercised in production
> from day one"). `CommandRegistry::load_eda_packs` and
> `commands/{sdc_base,eda_*}/` are gone; `tcl_spectcl::bundled` is what puts a
> vendor library into a profile registry, and it applies the same ambient-
> package filter this document specifies, so a profile still sees `sdc` plus
> its own vendor and no rival's.

## Motivation

Today a vendor EDA identity (e.g. "this is a Xilinx script") is encoded three
redundant ways:

1. a **vendor bit** in `DialectSet` (`XILINX`/`SYNOPSYS`/`CADENCE`/`QUARTUS`/
   `MENTOR`) — the availability gate, on every command spec and in the profile
   `availability_mask` (`TCL85|XILINX`);
2. the **loaded packs** — `load_dialect(XILINX)` loaded `sdc_base` +
   `eda_xilinx` (registry.rs), driven by `profile.base_layers`;
3. the **ambient library pins** — each EDA profile already declares `sdc` +
   its vendor tool (`vivado`, `synopsys-dc`, …) as `ambient: true`.

Real-world research (Vivado/Quartus/DC-PT-ICC2-FM/Innovus-Genus/Questa; see
`docs/design/dialect-detection.md` and the July-2026 EDA detection study)
confirms the package framing is how the tools actually work:

- **Quartus literally uses Tcl packages** — `package require ::quartus::project`
  / `load_package flow`; Intel even splits `::quartus::sdc` (industry SDC) from
  `::quartus::sdc_ext` (Altera-only).
- The other four are **ambient-shell** (no `package require`, no shebang):
  commands are built into `vivado`/`dc_shell`/`genus`/`vsim`. Detection is by
  command vocabulary — which the content-signature detector already does.
- **SDC is a shared, portable cross-vendor format** — the `sdc_base` pack is
  already the right shared library.

So the vendor *bit* (#1) is redundant with #2 + #3. This design removes it,
and makes the ambient-package set the availability gate.

## Package taxonomy (per-tool granularity)

One package per **tool** (the unit a user actually loads/runs), plus the
shared `sdc`. Grounded in the current command inventory.

### Shared
- **`sdc`** — the `sdc_base` pack (~60 cmds): SDC constraints + collection/query
  family (`create_clock`, `set_input_delay`, `set_false_path`, `get_cells`,
  `foreach_in_collection`, `report_timing`, `current_design`, …). Ambient in
  **every** EDA profile.

### Xilinx — profile `xilinx-eda-tcl`, base tcl8.5
- **`vivado`** — all of `eda_xilinx` (synth/impl/IP-integrator/project/hw):
  `synth_design`, `place_design`, `route_design`, `launch_runs`, `create_bd_*`,
  `create_project`, `write_bitstream`, `read_xdc`, `report_*`, `set_property`, …
  (Vitis/HLS and ISE are a future split; the current pack is Vivado.)

### Synopsys — profile `synopsys-eda-tcl`, base tcl8.6
- **`synopsys-dc`** (Design Compiler): analyze, elaborate, compile,
  compile_ultra, link, characterize, group, ungroup, uniquify,
  optimize_netlist, insert_clock_gating, insert_dft, set_clock_gating_style,
  set_scan_configuration, create_cell, create_net, create_port, connect_net,
  disconnect_net, remove_cell, remove_design, size_cell, swap_cell,
  current_instance, read_ddc, read_verilog, read_vhdl, read_db, read_file,
  write, write_file, report_design, report_hierarchy, report_reference,
  report_cell, report_net, report_clock_gating, report_congestion,
  report_bottleneck, report_status, report_qor, check_design, check_library,
  set_technology, set_operating_conditions
- **`synopsys-pt`** (PrimeTime): get_timing_paths, update_timing,
  report_analysis_coverage, report_delay_calculation
- **`synopsys-icc2`** (IC Compiler II): place_opt, clock_opt, route_opt,
  route_auto, create_floorplan, initialize_floorplan, read_def, read_lef,
  write_def, write_gds
- **`synopsys-fm`** (Formality): set_reference_design,
  set_implementation_design, match, verify
- **`synopsys`** (common shell, ambient in all Synopsys tools): set_app_var,
  set_host_options, printvar, read_sdc, write_sdc

### Cadence — profile `cadence-eda-tcl`, base tcl8.6 (see base-version note)
- **`cadence-common`** (Common UI attribute DB, shared Genus/Innovus/Tempus):
  set_db, get_db, dbget, dbset, dbquery, dbshape
- **`cadence-genus`** (synthesis): syn_generic, syn_map, syn_opt, read_hdl,
  write_hdl, elaborate, write_design
- **`cadence-innovus`** (P&R + MMMC): init_design, place_opt_design, opt_design,
  ccopt_design, route_design, create_floorplan, edit_pin, add_endcap,
  add_filler, add_well_tap, create_route_rule, stream_out, write_def, write_gds,
  write_netlist, read_netlist, read_physical, read_library, time_design,
  report_area, report_gates, report_power, report_dp, check_design,
  verify_connectivity, verify_drc, verify_geometry, write_do_lec, write_sdc,
  read_mmmc, create_analysis_view, create_constraint_mode, create_delay_corner,
  set_analysis_view, check_timing_intent, report_constraint, report_timing,
  report_qor, report_analysis_coverage, update_timing
- **`cadence-xcelium`** (simulation): xrun, xelab, xsim

### Quartus — profile `intel-quartus-eda-tcl`, base tcl8.5 (Intel's real names)
- **`quartus-project`**: project_new/open/close/exists, set_global_assignment,
  set_instance_assignment, set_io_assignment, set_location_assignment,
  get_global_assignment, get_instance_assignment, get_io_assignment,
  get_all_assignments, remove_all_assignments, export_assignments,
  set_parameter, get_names, get_name_info, make_connection, remove_connection,
  rename_node
- **`quartus-flow`**: execute_flow, execute_module
- **`quartus-sta`**: create_timing_netlist, update_timing_netlist,
  delete_timing_netlist, check_timing, report_timing, report_clock_fmax_summary,
  report_datasheet, report_min_pulse_width, report_ucp, read_sdc
- **`quartus-sdc-ext`**: derive_clocks, derive_pll_clocks
- **`quartus-report`**: load_report, save_report, get_report_panel_data,
  get_report_panel_id, get_report_panel_row_index, get_number_of_columns,
  get_number_of_rows
- **`quartus-device`**: device_lock, device_unlock, open_device, close_device,
  get_part_info, get_part_list
- **`quartus-misc`**: load_package

### Mentor/Siemens — profile `mentor-eda-tcl`, base tcl8.6 (see base-version note)
- **`questa`** (ModelSim/Questa simulation): vsim, vlog, vcom, vlib, vmap, vopt,
  vcover, vdel, add_wave, add_list, add_log, wave, qwave, run, force,
  signal_force, signal_release, examine, describe, drivers, find, change,
  restart, resume, release, toggle, coverage, transcript, virtual_, readers,
  when, onbreak, bp, bc, bd, be, bl, init_signal_driver, init_signal_spy, qrun,
  qverilog, qvhdl
- **`questa-formal`**: formal_analyze, formal_compile, formal_verify
- **`calibre`** (DRC/LVS/PEX batch launch): calibre, calibre_drc, calibre_lvs,
  calibre_pex. (NOTE: Calibre *rule decks* are SVRF, not Tcl — `.svrf` must not
  be classified as Tcl; these are only the Tcl-shell launch commands.)

**21 packages total.** The finer Quartus splits (`-report`/`-device`/`-misc`)
mirror Intel's real `::quartus::*` packages; they can collapse into a single
`quartus` if preferred.

## Availability semantics (the one real code change)

`ProfileQueries::is_available` currently gates only on the mask. New rule:

```
is_available(spec) =
      spec.supports_dialect(availability_mask)
   && (operators_as_commands || !OPERATOR_COMMAND)
   && package_loaded(spec.required_package)      // NEW
```
where
```
package_loaded(None)      = true
package_loaded(Some(p))   = self.is_ambient_package(p)   // profile ships it (EDA/F5)
                          || file `package require`d p    // explicit load (handled by caller ctx)
                          || !registry_knows_package(p)   // permissive: never flag 3rd-party pkgs
```
The permissive last clause preserves today's behaviour for tcllib/stdlib
(`uri::split` etc. stay available) — only packages a *profile declares ambient*
become a positive gate. Because each EDA profile declares **all** its tool
packages ambient, the EDA command surface is **unchanged** by the flip
(behaviour-preserving); granularity gives accurate provenance + sets up future
per-tool detection.

## Profile changes

Each EDA profile:
- `availability_mask`: `TCL8x | VENDOR` → **`TCL8x`** (same as plain tcl8.5/8.6).
- drop `vendor_bit` (→ `None`), or repurpose — profiles stay, distinguished only
  by their ambient package set.
- `base_layers`: `[VENDOR]` → load the tool packs by package identity.
- `libraries`: declare every tool package ambient (keeps W120 suppression +
  becomes the availability gate). Keyed version pins preserved.
- `grammar_union`: drop the vendor bit → `ALL_TCL`.

## Detection hardening (research-driven)

- `dialect_from_extension`: add `.do`→mentor, `.qsf`/`.qpf`/`.qip`→quartus
  (these are Tcl-syntax), `.globals`/`.enc`/`.invs_setup.tcl`→cadence,
  `.synopsys_dc.setup`/`.synopsys_pt.setup`→synopsys, `.mmmc`/`.view`→cadence.
  **Do NOT** map `.svrf` (Calibre SVRF is not Tcl).
- content signatures: add `set_db`/`get_db` (Cadence), `package require
  ::quartus::`/`load_package` (Quartus), `compile_ultra`/`dc_shell`/
  `.synopsys_*.setup` (Synopsys), the `wave.do` cluster (`onerror {resume}` +
  `quietly WaveActivateNextPane`) (Mentor); drop weak shared-SDC markers
  (`link_design`, `set_max_area`) as sole Synopsys tells.
- PrimeTime pure-SDC files are inherently ambiguous → acceptable to fall to a
  neutral `sdc` classification.

## Base Tcl version reconciliation (owner-decided)

Final base versions: **Xilinx 8.5, Quartus 8.5, Synopsys 8.6, Mentor 8.6,
Cadence 8.4.**

- Xilinx/Quartus 8.5 and Synopsys 8.6 (modern gtclsh) kept as-is — within the
  research ranges.
- **Mentor → 8.6** (owner decision): modern Questa/ModelSim embeds Tcl 8.6
  (bundled `tcl8.6` library paths). Older ModelSim shipped 8.4/8.5, but the
  current-tool default is 8.6 — Questa scripts get TclOO + the 8.6 core.
- **Cadence → 8.4-safe** (owner decision): real Innovus/Genus scripts
  systematically avoid dict/lassign/`{*}` (the 8.5 additions) and no public
  source pins a newer interpreter, so the analyser assumes an 8.4 core — no
  `{*}` expansion, no `::tcl::mathop` command heads, no TclOO. `sdc_base`
  widened from `TCL85_PLUS` to `ALL_TCL` so it still resolves under the 8.4
  Cadence mask (the `required_package` gate keeps it EDA-only).

Verified against the `dialect_surface` differential guard: the only surface
deltas are Mentor +43 (8.6 features: `oo::*`, `lmap`, `coroutine`, `throw`, …)
and Cadence −228 / +4 (loses the 8.5+ core + `::tcl::mathop` heads, gains four
8.4-era `test*proc` commands 8.6 had dropped); no other profile changed.

## Phased, behaviour-preserving execution (differential-guarded)

Capture a baseline first: for every EDA profile, the sorted set of available
command names + the resolved `required_package`/version for each. Re-run after
each phase; the surface must be **identical** until an intended change.

- **P1 — packages, additive.** Set `required_package` on every EDA command to its
  tool package (table above). No availability change yet (mask still gates).
  Declare all tool packages ambient on each profile. Teach `is_available` the
  new `package_loaded` clause. Surface unchanged (both gates pass).
- **P2 — drop the vendor bit from data.** EDA command `dialects: Some(VENDOR)` →
  `Some(base_version)`; sdc_base `Some(5-vendor-union)` → `Some(base_version)`;
  profile masks `TCL8x|VENDOR` → `TCL8x`. Availability now flows through
  package_loaded + base version. Differential-verify unchanged.
- **P3 — remove the 5 bits** from `DialectSet` (defs, parse, canonical_name,
  member_names, width comment) + `registry.rs` load arms + `expr_grammar_base_version`
  + tests. Compile-error-driven cleanup.
- **P4 — detection hardening** (extensions + signatures + `.svrf` exclusion).
- **P5 — base-version reconciliation** (Mentor→8.6; Cadence decision).
- **P6 — regen** (tree-sitter/tmLanguage/Zed grammars, AI prompts), all
  `cargo xtask` drift gates, full `cargo test`/clippy/fmt, differential report.

Each phase is its own commit; push after green.
