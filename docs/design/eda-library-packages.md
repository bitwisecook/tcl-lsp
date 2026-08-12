# EDA tools as library packages (not dialects)

A vendor EDA tool identity is a set of loaded **library packages** over a
base Tcl version, not a dialect of its own. There is no `XILINX`, `SYNOPSYS`,
`CADENCE`, `QUARTUS`, or `MENTOR` bit in `DialectSet`: an EDA profile carries
`vendor_bit: None` and a plain Tcl-version `availability_mask`, and its
command surface is gated by `required_package` instead.

## Why packages rather than a vendor bit

A vendor identity is fully expressed by two things a profile already carries:
the command packs it loads and the ambient library pins it declares. A
dedicated `DialectSet` bit adds nothing either does not already say, and
costs a bit of a fixed-width set plus a gate every command spec must be
tagged for.

Real-world research (Vivado/Quartus/DC-PT-ICC2-FM/Innovus-Genus/Questa; see
[contracts/dialect-detection.md](contracts/dialect-detection.md) and the
July-2026 EDA detection study) confirms the package framing is how the tools
actually work:

- **Quartus literally uses Tcl packages** — `package require ::quartus::project`
  / `load_package flow`; Intel even splits `::quartus::sdc` (industry SDC) from
  `::quartus::sdc_ext` (Altera-only).
- The other four are **ambient-shell** (no `package require`, no shebang):
  commands are built into `vivado`/`dc_shell`/`genus`/`vsim`. Detection is by
  command vocabulary — which the content-signature detector already does.
- **SDC is a shared, portable cross-vendor format** — the `sdc_base` pack is
  the right shared library, ambient in every EDA profile.

`DialectSet` is `u64` with twelve bits in use (five Tcl versions plus iRules,
iApps, Tk, Expect, BPF, tmsh, and the BIG-IP config surface). Bits 8–12 are
vacant and reserved for future dialects — `dialect_set.rs` notes that they
are the slots the vendor bits would have occupied, so nobody reintroduces
them there.

## Package taxonomy (per-tool granularity)

One package per **tool** (the unit a user actually loads/runs), plus the
shared `sdc`. **21 packages** across the five profiles.

Every `CommandSpec` in an EDA pack is stamped with its package as it is
built: `sdc_base_command_specs()` stamps `Some("sdc")` over the whole vector,
`eda_xilinx_command_specs()` stamps `Some("vivado")`, and the other four
crates route each spec through a `*_package_for(name)` match with the vendor's
bulk pack as the fallback arm.

### Shared
- **`sdc`** — the `sdc_base` pack (61 commands): SDC constraints + collection/query
  family (`create_clock`, `set_input_delay`, `set_false_path`, `get_cells`,
  `foreach_in_collection`, `report_timing`, `current_design`, …). Ambient in
  **every** EDA profile. Its specs are gated `ALL_TCL` rather than
  `TCL85_PLUS`, so they still resolve under the 8.4 Cadence mask; the
  `required_package` gate is what keeps them EDA-only.

### Xilinx — profile `xilinx-eda-tcl`, base tcl8.5
- **`vivado`** — all 64 commands of `eda_xilinx` (synth/impl/IP-integrator/project/hw):
  `synth_design`, `place_design`, `route_design`, `launch_runs`, `create_bd_*`,
  `create_project`, `write_bitstream`, `read_xdc`, `report_*`, `set_property`, …
  (Vitis/HLS and ISE are a future split; the current pack is Vivado.)

### Synopsys — profile `synopsys-eda-tcl`, base tcl8.6
- **`synopsys-dc`** (Design Compiler) — the fallback arm, so every
  `eda_synopsys` command not claimed below: analyze, elaborate, compile,
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

### Cadence — profile `cadence-eda-tcl`, base tcl8.4 (see base-version note)
- **`cadence-common`** (Common UI attribute DB, shared Genus/Innovus/Tempus):
  set_db, get_db, dbget, dbset, dbquery, dbshape
- **`cadence-genus`** (synthesis): syn_generic, syn_map, syn_opt, read_hdl,
  write_hdl, elaborate, write_design
- **`cadence-innovus`** (P&R + MMMC) — the fallback arm: init_design,
  place_opt_design, opt_design, ccopt_design, route_design, create_floorplan,
  edit_pin, add_endcap, add_filler, add_well_tap, create_route_rule,
  stream_out, write_def, write_gds, write_netlist, read_netlist,
  read_physical, read_library, time_design, report_area, report_gates,
  report_power, report_dp, check_design, verify_connectivity, verify_drc,
  verify_geometry, write_do_lec, write_sdc, read_mmmc, create_analysis_view,
  create_constraint_mode, create_delay_corner, set_analysis_view,
  check_timing_intent, report_constraint, report_timing, report_qor,
  report_analysis_coverage, update_timing
- **`cadence-xcelium`** (simulation): xrun, xelab, xsim

### Quartus — profile `intel-quartus-eda-tcl`, base tcl8.5 (Intel's real names)
- **`quartus-project`** — the fallback arm: project_new/open/close/exists,
  set_global_assignment, set_instance_assignment, set_io_assignment,
  set_location_assignment, get_global_assignment, get_instance_assignment,
  get_io_assignment, get_all_assignments, remove_all_assignments,
  export_assignments, set_parameter, get_names, get_name_info,
  make_connection, remove_connection, rename_node
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
- **`questa`** (ModelSim/Questa simulation) — the fallback arm: vsim, vlog,
  vcom, vlib, vmap, vopt, vcover, vdel, add_wave, add_list, add_log, wave,
  qwave, run, force, signal_force, signal_release, examine, describe, drivers,
  find, change, restart, resume, release, toggle, coverage, transcript,
  virtual_, readers, when, onbreak, bp, bc, bd, be, bl, init_signal_driver,
  init_signal_spy, qrun, qverilog, qvhdl
- **`questa-formal`**: formal_analyze, formal_compile, formal_verify
- **`calibre`** (DRC/LVS/PEX batch launch): calibre, calibre_drc, calibre_lvs,
  calibre_pex. (NOTE: Calibre *rule decks* are SVRF, not Tcl — `.svrf` must not
  be classified as Tcl; these are only the Tcl-shell launch commands.)

The finer Quartus splits (`-report`/`-device`/`-misc`) mirror Intel's real
`::quartus::*` packages.

## Availability semantics

`ProfileQueries::is_available` (`profile_queries.rs`) is:

```text
is_available(spec) =
      spec.supports_dialect(availability_mask)
   && (operators_as_commands || !OPERATOR_COMMAND)
   && package_available(self, spec.required_package)
```

`package_available` has exactly **two** satisfying clauses, not three — a
file's own `package require` is *not* one of them:

```text
package_available(profile, None)      = true
package_available(profile, Some(p))   = profile.is_ambient_package(p)
                                      || !vendor_ambient_packages().contains(p)
```

`vendor_ambient_packages()` is computed once from the catalog: every package
any profile pins `ambient: true` — the EDA tool surfaces and the F5 command
packs. So the gate is closed-world in one direction only:

- **No gate** (`required_package: None`) — always satisfied.
- **A hosted library** (Tk, tcllib, the stdlib packages — never ambient in any
  profile) — satisfied unconditionally. The command stays known and available;
  a missing `package require` is reported separately by W120 rather than by
  hiding the command. This is why `uri::split` and friends stay available
  under plain Tcl.
- **A closed-world vendor package** (ambient in *some* profile) — satisfied
  only when *this* profile ships it ambient, so a `vivado` command never
  resolves under a plain-Tcl or rival-vendor profile.

Because each EDA profile declares **all** its own tool packages ambient, the
EDA command surface is the same as it was under the vendor bits; the
granularity buys accurate provenance and sets up future per-tool detection.

## Profile shape

Each EDA profile in `rust/tcl-dialect/src/profile.rs`:

| Field | Value |
|-------|-------|
| `vendor_bit` | `None` |
| `availability_mask` | the single base-version bit (`TCL84`/`TCL85`/`TCL86`) |
| `base_layers` | `&[<that same version bit>]` |
| `grammar_union` | `DialectSet::ALL_TCL` |
| `libraries` | one `LibraryPin { ambient: true }` per tool package, plus `sdc` |

The pins carry `LibraryVersion::Keyed` versions (`VersionKey::SdcVersion` for
`sdc`, `VersionKey::ToolVersion` for the vendor packs), so an EDA profile
still participates in the versioned-library axis without a vendor bit.

Because the profiles carry no vendor bit, consumers that used to ask
"is this a vendor shell?" key off something else. `DialectProfile::hosts_tk()`
is the worked example: it answers `false` for the EDA shells because they pin
no Tk library, not because they carry a vendor bit.

The packs load by **profile name**, not by a `DialectSet` bit:
`CommandRegistry::load_eda_packs(profile_name)` matches the five EDA profile
names, inserts `sdc_base_command_specs()`, then inserts that vendor's specs.
It is a no-op for any other name, and `load_dialect` deliberately has no arm
for the EDA shells. The per-profile registry cache (`cache.rs`) calls it
after the base-version dialect has been loaded.

## Detection

### By filename

`dialect_from_extension` (`rust/tcl-registry/src/dialects.rs`) checks two
multi-component filename conventions before falling back to the trailing
extension:

- `*.synopsys_dc.setup`, `*.synopsys_pt.setup` → `synopsys-eda-tcl`
- `*.invs_setup.tcl`, `*.genus_setup.tcl` → `cadence-eda-tcl`

Then by extension: `.xdc` → Xilinx, `.sdc` → Synopsys, `.do` → Mentor,
`.qsf` / `.qpf` / `.qip` → Quartus, `.globals` → Cadence.

`.svrf` is **deliberately not mapped**: Calibre rule decks are a declarative
DSL, not Tcl, so the extension falls through to content detection and the
caller's default rather than being forced to an EDA Tcl dialect.

### By content

The content-signature table lists each profile's **proprietary** commands
only. Shared SDC verbs (`create_clock`, `set_input_delay`, `link_design`,
`set_max_area`, `get_ports`, …) are excluded, because they appear in every
vendor's constraint files and would misclassify a portable `.sdc`. A
PrimeTime pure-SDC file is inherently ambiguous and is left to fall through
rather than be guessed at.

| Profile | Markers |
|---------|---------|
| `xilinx-eda-tcl` | `synth_design`, `launch_runs`, `create_bd_design`, `write_bitstream`, `create_project`, `read_xdc` |
| `synopsys-eda-tcl` | `compile_ultra`, `dc_shell`, `pt_shell`, `icc2_shell`, `fm_shell`, `set_svf`, `set_app_var` |
| `cadence-eda-tcl` | `set_db`, `get_db`, `syn_generic`, `place_opt_design`, `innovus`, `genus`, `init_design` |
| `intel-quartus-eda-tcl` | `quartus_`, `::quartus::`, `project_new`, `set_global_assignment`, `set_location_assignment`, `execute_flow` |
| `mentor-eda-tcl` | `vsim`, `vlog`, `vcom`, `vlib`, `vmap`, `vopt`, `questa` |

Markers match at word boundaries (`contains_token`); a marker ending in a
non-word byte (`::quartus::`) or in `_` (`quartus_`) is a command *prefix*
form and imposes no right-hand boundary.

## Base Tcl versions

| Profile | Base | Rationale |
|---------|------|-----------|
| `xilinx-eda-tcl` | 8.5 | within the research range for Vivado |
| `intel-quartus-eda-tcl` | 8.5 | within the research range for Quartus |
| `synopsys-eda-tcl` | 8.6 | modern gtclsh |
| `mentor-eda-tcl` | 8.6 | modern Questa/ModelSim embeds Tcl 8.6 (bundled `tcl8.6` library paths). Older ModelSim shipped 8.4/8.5, but the current-tool default is 8.6, so Questa scripts get TclOO and the 8.6 core |
| `cadence-eda-tcl` | 8.4 | real Innovus/Genus scripts systematically avoid `dict`, `lassign`, and `{*}` (the 8.5 additions) and no public source pins a newer interpreter, so the analyser assumes an 8.4 core — no `{*}` expansion, no `::tcl::mathop` command heads, no TclOO |

`tcloo` is `true` only for the two 8.6-based profiles (`synopsys-eda-tcl`,
`mentor-eda-tcl`); `leading_zero_is_octal` is `Ternary::Yes` for all five,
since none loads a Tcl-9 version bit.

## Guarding the surface

`cargo run -q --example dialect_surface` (in `rust/tcl-registry`) prints one
`profile<TAB>command` line for every command that resolves under every
catalog profile, profiles in catalog order and commands sorted within each.
It is the differential guard for any availability-affecting change to this
model: capture it, make the change, re-run, and diff — the surface must be
identical except where the change is intended.
