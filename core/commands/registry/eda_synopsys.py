"""Synopsys EDA Tcl commands — Design Compiler, PrimeTime, ICC2, Formality.

Vendor-specific commands for the ``synopsys-eda-tcl`` dialect.
SDC base commands are provided separately by ``eda_sdc_base``.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "Synopsys Design Compiler / PrimeTime / ICC2"
_DIALECT = frozenset({"synopsys-eda-tcl"})


def _syn(
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


def synopsys_command_specs() -> tuple[CommandSpec, ...]:
    """Return Synopsys-specific command specs."""
    return (
        # --- Design entry ---
        _syn(
            "analyze",
            "Analyze HDL source files for syntax and semantic errors.",
            "analyze -format format ?-library lib? ?-define define_list? ?-work work? file_list",
        ),
        _syn(
            "elaborate",
            "Elaborate a design from analyzed HDL.",
            "elaborate design_name ?-library lib? ?-architecture arch? ?-parameters params? ?-update?",
            Arity(1),
        ),
        _syn(
            "read_file", "Read a design file into memory.", "read_file ?-format format? file_list"
        ),
        _syn(
            "read_verilog",
            "Read Verilog source files.",
            "read_verilog ?-define define_list? file_list",
        ),
        _syn("read_vhdl", "Read VHDL source files.", "read_vhdl ?-library lib? file_list"),
        _syn("read_ddc", "Read a Synopsys DDC database.", "read_ddc file_name", Arity(1, 1)),
        _syn("read_db", "Read a .db technology library.", "read_db file_name", Arity(1, 1)),
        _syn("read_sdc", "Read SDC constraint file.", "read_sdc ?-echo? file_name", Arity(1)),
        # --- Design navigation ---
        _syn("link", "Link the current design to library cells.", "link ?-force?"),
        _syn(
            "uniquify",
            "Make each instance of a subdesign unique.",
            "uniquify ?-force? ?-dont_skip_empty_designs?",
        ),
        _syn(
            "ungroup",
            "Dissolve hierarchy of specified cells.",
            "ungroup ?-all? ?-flatten? ?-start_level n? ?-simple_names? ?cells?",
        ),
        _syn(
            "remove_design",
            "Remove a design from memory.",
            "remove_design ?-all? ?-designs? ?design_list?",
        ),
        _syn(
            "current_instance",
            "Navigate into a hierarchical instance.",
            "current_instance ?instance_name?",
        ),
        # --- Compilation ---
        _syn(
            "compile",
            "Compile (synthesize) the current design.",
            "compile ?-map_effort effort? ?-area_effort effort? ?-incremental_mapping? ?-exact_map? ?-no_design_rule? ?-scan?",
        ),
        _syn(
            "compile_ultra",
            "Compile with advanced optimizations.",
            "compile_ultra ?-incremental? ?-retime? ?-scan? ?-no_autoungroup? ?-no_boundary_optimization? ?-gate_clock? ?-timing_high_effort_script?",
        ),
        _syn(
            "optimize_netlist",
            "Perform incremental netlist optimization.",
            "optimize_netlist -area",
        ),
        # --- Writing output ---
        _syn(
            "write",
            "Write design data to a file.",
            "write ?-format format? ?-hierarchy? ?-output file? ?design_list?",
        ),
        _syn(
            "write_sdc",
            "Write SDC constraints to a file.",
            "write_sdc ?-nosplit? ?-version version? ?file_name?",
        ),
        _syn(
            "write_file",
            "Write design to file in specified format.",
            "write_file ?-format format? ?-hierarchy? ?-output file?",
        ),
        # --- Variables & environment ---
        _syn(
            "set_app_var",
            "Set a Synopsys application variable.",
            "set_app_var variable_name value",
            Arity(2, 2),
        ),
        _syn("printvar", "Print the value of an application variable.", "printvar ?variable_name?"),
        _syn(
            "set_host_options",
            "Configure multi-core and host settings.",
            "set_host_options ?-max_cores n?",
        ),
        # --- Reporting ---
        _syn(
            "report_cell",
            "Report cell-level information.",
            "report_cell ?-nosplit? ?-connections? ?cell_list?",
        ),
        _syn(
            "report_net",
            "Report net information.",
            "report_net ?-nosplit? ?-connections? ?-verbose? ?net_list?",
        ),
        _syn(
            "report_reference",
            "Report library cell references.",
            "report_reference ?-nosplit? ?-hierarchy?",
        ),
        _syn("report_hierarchy", "Report design hierarchy.", "report_hierarchy ?-nosplit? ?-full?"),
        _syn(
            "report_qor", "Report Quality of Results metrics.", "report_qor ?-nosplit? ?-summary?"
        ),
        _syn(
            "report_clock_gating",
            "Report clock gating statistics.",
            "report_clock_gating ?-nosplit? ?-verbose?",
        ),
        # --- Design checks ---
        _syn(
            "check_design",
            "Check the design for consistency problems.",
            "check_design ?-summary? ?-no_warnings?",
        ),
        _syn(
            "check_library",
            "Check libraries for consistency issues.",
            "check_library ?library_list?",
        ),
        # --- Clock gating ---
        _syn(
            "set_clock_gating_style",
            "Specify clock gating implementation style.",
            "set_clock_gating_style ?-sequential_cell cell_type? ?-positive_edge_logic gate_type? ?-negative_edge_logic gate_type? ?-control_point before|after? ?-control_signal scan_enable? ?-minimum_bitwidth n? ?-max_fanout n?",
        ),
        _syn("insert_clock_gating", "Insert clock gating logic.", "insert_clock_gating ?-global?"),
        # --- DFT (Design For Test) ---
        _syn(
            "set_scan_configuration",
            "Configure scan chain parameters.",
            "set_scan_configuration ?-chain_count n? ?-clock_mixing mix_type? ?-style style?",
        ),
        _syn("insert_dft", "Insert DFT structures (scan chains).", "insert_dft"),
        # --- Operating conditions ---
        _syn(
            "set_operating_conditions",
            "Set operating conditions for timing.",
            "set_operating_conditions ?-library lib? ?-min min_cond? ?-max max_cond? ?condition_name?",
        ),
        _syn("set_technology", "Set the target technology.", "set_technology ?technology?"),
        # --- Netlist editing (ECO) ---
        _syn(
            "create_port",
            "Create a new port in the design.",
            "create_port port_name ?-direction direction?",
            Arity(1),
        ),
        _syn(
            "create_cell",
            "Create a new cell instance.",
            "create_cell cell_name lib_cell",
            Arity(2, 2),
        ),
        _syn("create_net", "Create a new net.", "create_net net_name", Arity(1, 1)),
        _syn(
            "connect_net",
            "Connect a net to pins/ports.",
            "connect_net net_name port_pin_list",
            Arity(2),
        ),
        _syn(
            "disconnect_net",
            "Disconnect a net from pins/ports.",
            "disconnect_net net_name port_pin_list",
            Arity(2),
        ),
        _syn("remove_cell", "Remove a cell from the design.", "remove_cell cell_list"),
        _syn(
            "size_cell",
            "Resize a cell to a different library cell.",
            "size_cell cell_name lib_cell",
            Arity(2, 2),
        ),
        _syn(
            "swap_cell",
            "Swap a cell with a different library cell.",
            "swap_cell cell_list lib_cell",
        ),
        _syn(
            "group",
            "Group logic into a hierarchical block.",
            "group ?-design_name name? ?-cell_name name? cell_list",
        ),
        _syn(
            "characterize",
            "Characterize a subdesign for context-dependent optimization.",
            "characterize ?-constraints? instance_list",
        ),
        # --- Physical (ICC2) ---
        _syn(
            "read_def",
            "Read a DEF file.",
            "read_def ?-add_def_only_objects all? file_name",
            Arity(1),
        ),
        _syn("write_def", "Write a DEF file.", "write_def ?-version version? file_name"),
        _syn("read_lef", "Read a LEF technology file.", "read_lef file_name", Arity(1, 1)),
        _syn(
            "write_gds", "Write a GDSII stream file.", "write_gds ?-long_names? ?-output file_name?"
        ),
        _syn(
            "create_floorplan",
            "Create an initial floorplan.",
            "create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-left_io2core dist? ?-bottom_io2core dist? ?-right_io2core dist? ?-top_io2core dist?",
        ),
        _syn(
            "initialize_floorplan",
            "Initialize the floorplan from constraints.",
            "initialize_floorplan ?-core_utilization util? ?-core_offset offset?",
        ),
        _syn("place_opt", "Perform placement optimization.", "place_opt ?-effort high|medium|low?"),
        _syn(
            "clock_opt",
            "Perform clock tree synthesis and optimization.",
            "clock_opt ?-effort high|medium|low?",
        ),
        _syn(
            "route_auto",
            "Perform automatic routing.",
            "route_auto ?-max_detail_route_iterations n?",
        ),
        _syn(
            "route_opt", "Perform post-route optimization.", "route_opt ?-effort high|medium|low?"
        ),
        _syn("report_congestion", "Report routing congestion.", "report_congestion ?-nosplit?"),
        _syn("report_design", "Report design summary.", "report_design ?-nosplit? ?-verbose?"),
        # --- PrimeTime ---
        _syn("update_timing", "Update timing in the design.", "update_timing ?-full?"),
        _syn(
            "report_analysis_coverage",
            "Report timing analysis coverage.",
            "report_analysis_coverage ?-nosplit? ?-status_details untested?",
        ),
        _syn(
            "report_bottleneck",
            "Report timing bottleneck analysis.",
            "report_bottleneck ?-nosplit? ?-max_cells n?",
        ),
        _syn(
            "get_timing_paths",
            "Get timing paths as a collection.",
            "get_timing_paths ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-slack_lesser_than value?",
        ),
        _syn(
            "report_delay_calculation",
            "Report delay calculation details.",
            "report_delay_calculation ?-from from_pin? ?-to to_pin?",
        ),
        # --- Formality ---
        _syn(
            "set_reference_design",
            "Set the reference design for formal verification.",
            "set_reference_design design_name",
            Arity(1, 1),
        ),
        _syn(
            "set_implementation_design",
            "Set the implementation design for formal verification.",
            "set_implementation_design design_name",
            Arity(1, 1),
        ),
        _syn("verify", "Run formal equivalence checking.", "verify ?-verbose?"),
        _syn("match", "Match compare points between reference and implementation.", "match"),
        _syn("report_status", "Report verification status.", "report_status ?-verbose?"),
    )
