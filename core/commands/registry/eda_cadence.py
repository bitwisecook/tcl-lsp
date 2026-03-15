"""Cadence EDA Tcl commands — Genus, Innovus, Tempus, Xcelium.

Vendor-specific commands for the ``cadence-eda-tcl`` dialect.
SDC base commands are provided separately by ``eda_sdc_base``.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "Cadence Genus / Innovus / Tempus"
_DIALECT = frozenset({"cadence-eda-tcl"})


def _cad(
    name: str,
    summary: str,
    synopsis: str = "",
    arity: Arity | None = None,
) -> CommandSpec:
    syn = synopsis or f"{name} ?options? ?args ...?"
    return CommandSpec(
        name=name,
        dialects=_DIALECT,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(syn,),
            source=_SOURCE,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=syn),),
        validation=ValidationSpec(arity=arity or Arity()),
    )


def cadence_command_specs() -> tuple[CommandSpec, ...]:
    """Return Cadence-specific command specs."""
    return (
        # --- Genus (synthesis) ---
        _cad("read_hdl", "Read HDL source files (Verilog/SystemVerilog/VHDL).",
             "read_hdl ?-language language? ?-define define_list? file_list"),
        _cad("read_library", "Read technology library files.",
             "read_library ?-liberty? ?-lef? file_list"),
        _cad("elaborate", "Elaborate the design hierarchy.",
             "elaborate ?design_name? ?-parameters params?"),
        _cad("syn_generic", "Perform technology-independent synthesis.",
             "syn_generic ?-effort effort?"),
        _cad("syn_map", "Map the design to target technology.",
             "syn_map ?-effort effort?"),
        _cad("syn_opt", "Perform post-map optimization.",
             "syn_opt ?-effort effort? ?-incremental?"),
        _cad("write_hdl", "Write synthesized HDL netlist.",
             "write_hdl ?-lec? ?design_name? > file"),
        _cad("write_sdc", "Write SDC constraints.",
             "write_sdc > file"),
        _cad("write_design", "Write a design database.",
             "write_design ?-innovus? ?-base_name name?"),
        _cad("write_do_lec", "Write a Conformal LEC do file.",
             "write_do_lec ?-revised_design design? ?-logicEquivalence? > file"),

        # --- Database access (Stylus Common UI) ---
        _cad("set_db", "Set a database attribute value.",
             "set_db object_or_attr value", Arity(2)),
        _cad("get_db", "Get a database attribute value.",
             "get_db object_or_attr ?-regexp? ?-foreach script?"),
        _cad("report_dp", "Report datapath resources.",
             "report_dp ?-all?"),
        _cad("report_gates", "Report gate-level statistics.",
             "report_gates ?-power?"),

        # --- Design checks (Genus/Innovus common) ---
        _cad("report_qor", "Report quality-of-results summary.",
             "report_qor ?-summary?"),
        _cad("check_design", "Check the design for issues.",
             "check_design ?-all? ?-type type?"),
        _cad("check_timing_intent", "Verify timing intent completeness.",
             "check_timing_intent ?-verbose?"),

        # --- Innovus (place & route) ---
        _cad("init_design", "Initialize the design for implementation.",
             "init_design"),
        _cad("read_netlist", "Read a gate-level netlist.",
             "read_netlist file_name", Arity(1, 1)),
        _cad("read_physical", "Read physical data (LEF/DEF).",
             "read_physical ?-lef file_list? ?-def file?"),
        _cad("read_mmmc", "Read Multi-Mode Multi-Corner (MMMC) configuration.",
             "read_mmmc file_name", Arity(1, 1)),
        _cad("create_floorplan", "Create a floorplan.",
             "create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-core_margins_by die|core? margins"),
        _cad("create_constraint_mode", "Create a constraint mode for MMMC.",
             "create_constraint_mode -name name -sdc_files file_list", Arity(1)),
        _cad("create_delay_corner", "Create a delay corner for MMMC.",
             "create_delay_corner -name name ?-library_set lib_set? ?-rc_corner rc_corner?", Arity(1)),
        _cad("create_analysis_view", "Create an analysis view combining mode and corner.",
             "create_analysis_view -name name -constraint_mode mode -delay_corner corner", Arity(1)),
        _cad("set_analysis_view", "Set active analysis views for setup and hold.",
             "set_analysis_view ?-setup view_list? ?-hold view_list?"),
        _cad("place_opt_design", "Perform placement and optimization.",
             "place_opt_design ?-effort effort? ?-incremental?"),
        _cad("ccopt_design", "Perform clock tree synthesis (CTS).",
             "ccopt_design ?-cts? ?-post_cts_opt?"),
        _cad("route_design", "Route the design.",
             "route_design ?-global_detail?"),
        _cad("opt_design", "Optimize the design (post-route).",
             "opt_design ?-post_route? ?-setup? ?-hold? ?-drv?"),
        _cad("add_endcap", "Add endcap cells.",
             "add_endcap ?-pre_endcap cell? ?-post_endcap cell?"),
        _cad("add_filler", "Add filler cells.",
             "add_filler ?-cell cell_list? ?-prefix prefix?"),
        _cad("add_well_tap", "Add well-tap cells.",
             "add_well_tap ?-cell cell? ?-cell_interval interval?"),
        _cad("edit_pin", "Place or move pins.",
             "edit_pin ?-pin pin_list? ?-edge edge? ?-layer layer? ?-start coord? ?-end coord? ?-fixedPin? ?-snap?"),
        _cad("create_route_rule", "Create a non-default routing rule.",
             "create_route_rule -name name ?-widths list? ?-spacings list?", Arity(1)),

        # --- Verification ---
        _cad("verify_connectivity", "Verify design connectivity.",
             "verify_connectivity ?-type type? ?-error n?"),
        _cad("verify_geometry", "Verify design geometry (DRC).",
             "verify_geometry ?-error n?"),
        _cad("verify_drc", "Run design rule checking.",
             "verify_drc ?-limit n?"),

        # --- Timing (Tempus / Innovus) ---
        _cad("time_design", "Perform timing analysis.",
             "time_design ?-pre_cts? ?-post_cts? ?-post_route? ?-hold? ?-report_prefix prefix?"),
        _cad("report_timing", "Report timing paths.",
             "report_timing ?-from from? ?-to to? ?-through through? ?-max_paths n? ?-nworst n?"),
        _cad("update_timing", "Update incremental timing.",
             "update_timing ?-full?"),
        _cad("report_constraint", "Report constraint violations.",
             "report_constraint ?-all_violators? ?-drv_violation_type type?"),
        _cad("report_analysis_coverage", "Report timing analysis coverage.",
             "report_analysis_coverage"),

        # --- Output ---
        _cad("write_def", "Write a DEF file.",
             "write_def ?-version version? file_name"),
        _cad("write_netlist", "Write a gate-level netlist.",
             "write_netlist file_name ?-top_module_first?"),
        _cad("write_gds", "Write a GDSII stream file.",
             "write_gds ?-merge gds_files? ?-map_file map? file_name"),
        _cad("stream_out", "Stream out a GDSII file (legacy).",
             "stream_out file_name ?-mapFile map?"),

        # --- Xcelium (simulation) ---
        _cad("xrun", "Run Xcelium compilation and simulation in a single step.",
             "xrun ?-sv? ?-access access_type? ?-define define? ?-top top_module? ?-input cmd_file? file_list"),
        _cad("xelab", "Elaborate a design for Xcelium simulation.",
             "xelab ?-access access_type? ?-top top_module? ?-snapshot snap_name?"),
        _cad("xsim", "Run Xcelium simulation on an elaborated snapshot.",
             "xsim ?-R? ?-input cmd_file? snapshot_name"),

        # --- Legacy database access ---
        _cad("dbGet", "Get a design database object attribute (legacy).",
             "dbGet object_spec.attribute ?-regexp pattern? ?-e?"),
        _cad("dbQuery", "Query design database objects (legacy).",
             "dbQuery ?-area box? ?-objType type?"),
        _cad("dbSet", "Set a design database object attribute (legacy).",
             "dbSet object_spec.attribute value"),
        _cad("dbShape", "Access shape data from the design database (legacy).",
             "dbShape ?-shape type? ?-net net_name?"),

        # --- Power ---
        _cad("report_area", "Report design area.",
             "report_area ?-physical? ?-verbose?"),
        _cad("report_power", "Report power consumption.",
             "report_power ?-leakage? ?-dynamic? ?-view view_name?"),
    )
