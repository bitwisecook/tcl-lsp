"""Shared SDC (Synopsys Design Constraints) / OpenSTA base commands.

Industry-standard timing constraint and query commands used across all
EDA vendor tools.  Each vendor dialect inherits these.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "IEEE 1801 / SDC 2.1 standard"

# Dialect sets are injected at import time by each vendor module;
# this module only defines the specs.

_SDC_DIALECTS: frozenset[str] = frozenset(
    {
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
    }
)


def _sdc(
    name: str,
    summary: str,
    synopsis: str = "",
    arity: Arity | None = None,
) -> CommandSpec:
    """Create an SDC CommandSpec shared across all EDA dialects."""
    syn = synopsis or f"{name} ?options? ?args ...?"
    return CommandSpec(
        name=name,
        dialects=_SDC_DIALECTS,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(syn,),
            source=_SOURCE,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=syn),),
        validation=ValidationSpec(arity=arity or Arity()),
    )


def sdc_base_command_specs() -> tuple[CommandSpec, ...]:
    """Return all shared SDC command specs."""
    return (
        # --- Clock constraints ---
        _sdc(
            "create_clock",
            "Create a clock object.",
            "create_clock ?-period period? ?-name name? ?-waveform edge_list? ?-add? ?source_objects?",
            Arity(1),
        ),
        _sdc(
            "create_generated_clock",
            "Create a generated clock object.",
            "create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-multiply_by factor? ?-duty_cycle percent? ?-invert? ?-add? source_objects",
            Arity(1),
        ),
        _sdc(
            "set_clock_groups",
            "Set mutual exclusivity between clock groups.",
            "set_clock_groups ?-physically_exclusive | -logically_exclusive | -asynchronous? -group clock_list ...",
        ),
        _sdc(
            "set_clock_latency",
            "Set clock network latency.",
            "set_clock_latency ?-source? ?-early | -late? ?-rise | -fall? delay object_list",
        ),
        _sdc(
            "set_clock_uncertainty",
            "Set clock uncertainty (jitter).",
            "set_clock_uncertainty ?-setup | -hold? ?-from from_clock? ?-to to_clock? ?-rise | -fall? uncertainty ?object_list?",
        ),
        _sdc(
            "set_clock_transition",
            "Set clock transition time.",
            "set_clock_transition ?-rise | -fall? ?-min | -max? transition clock_list",
        ),
        _sdc(
            "set_propagated_clock",
            "Specify that clock latency should be propagated.",
            "set_propagated_clock object_list",
        ),
        # --- I/O delay constraints ---
        _sdc(
            "set_input_delay",
            "Set input delay on ports.",
            "set_input_delay ?-clock clock_name? ?-clock_fall? ?-level_sensitive? ?-rise | -fall? ?-min | -max? ?-add_delay? ?-network_latency_included? ?-source_latency_included? delay_value port_pin_list",
        ),
        _sdc(
            "set_output_delay",
            "Set output delay on ports.",
            "set_output_delay ?-clock clock_name? ?-clock_fall? ?-level_sensitive? ?-rise | -fall? ?-min | -max? ?-add_delay? ?-network_latency_included? ?-source_latency_included? delay_value port_pin_list",
        ),
        _sdc(
            "set_input_transition",
            "Set input transition time on ports.",
            "set_input_transition ?-rise | -fall? ?-min | -max? transition port_list",
        ),
        _sdc(
            "set_load",
            "Set capacitive load on nets or ports.",
            "set_load ?-pin_load | -wire_load? ?-min | -max? ?-subtract_pin_load? value objects",
        ),
        _sdc(
            "set_driving_cell",
            "Set the driving cell for input ports.",
            "set_driving_cell ?-lib_cell cell_name? ?-library lib? ?-pin pin_name? ?-from_pin from_pin_name? ?-no_design_rule? ?-dont_scale? ?-input_transition_rise rise? ?-input_transition_fall fall? port_list",
        ),
        # --- Timing exceptions ---
        _sdc(
            "set_false_path",
            "Define a false timing path.",
            "set_false_path ?-setup | -hold? ?-from from_list? ?-through through_list? ?-to to_list? ?-rise_from rise_from_list? ?-fall_from fall_from_list? ?-rise_to rise_to_list? ?-fall_to fall_to_list? ?-comment comment?",
        ),
        _sdc(
            "set_multicycle_path",
            "Define a multicycle timing path.",
            "set_multicycle_path ?-setup | -hold? ?-start | -end? ?-from from_list? ?-through through_list? ?-to to_list? path_multiplier",
        ),
        _sdc(
            "set_max_delay",
            "Set maximum delay constraint on a path.",
            "set_max_delay ?-from from_list? ?-through through_list? ?-to to_list? ?-rise | -fall? delay_value",
        ),
        _sdc(
            "set_min_delay",
            "Set minimum delay constraint on a path.",
            "set_min_delay ?-from from_list? ?-through through_list? ?-to to_list? ?-rise | -fall? delay_value",
        ),
        # --- Design rule constraints ---
        _sdc("set_max_area", "Set maximum area constraint.", "set_max_area area_value", Arity(1)),
        _sdc(
            "set_max_fanout",
            "Set maximum fanout constraint.",
            "set_max_fanout fanout_value object_list",
        ),
        _sdc(
            "set_max_transition",
            "Set maximum transition time constraint.",
            "set_max_transition transition_value ?-clock_path? ?-data_path? object_list",
        ),
        _sdc(
            "set_max_capacitance",
            "Set maximum capacitance constraint.",
            "set_max_capacitance cap_value object_list",
        ),
        # --- Object access commands ---
        _sdc(
            "get_cells",
            "Get cell objects matching a pattern.",
            "get_cells ?-hierarchical? ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?",
        ),
        _sdc(
            "get_pins",
            "Get pin objects matching a pattern.",
            "get_pins ?-hierarchical? ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?-leaf? ?patterns?",
        ),
        _sdc(
            "get_nets",
            "Get net objects matching a pattern.",
            "get_nets ?-hierarchical? ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?",
        ),
        _sdc(
            "get_ports",
            "Get port objects matching a pattern.",
            "get_ports ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
        ),
        _sdc(
            "get_clocks",
            "Get clock objects matching a pattern.",
            "get_clocks ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
        ),
        _sdc(
            "get_libs",
            "Get library objects matching a pattern.",
            "get_libs ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
        ),
        _sdc(
            "get_lib_cells",
            "Get library cell objects matching a pattern.",
            "get_lib_cells ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?",
        ),
        _sdc(
            "get_lib_pins",
            "Get library pin objects matching a pattern.",
            "get_lib_pins ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?",
        ),
        # --- Collection shortcuts ---
        _sdc("all_inputs", "Return all input ports.", "all_inputs"),
        _sdc("all_outputs", "Return all output ports.", "all_outputs"),
        _sdc(
            "all_registers",
            "Return all register cells.",
            "all_registers ?-clock clock_name? ?-rise_clock clock_name? ?-fall_clock clock_name? ?-cells? ?-data_pins? ?-clock_pins? ?-slave_clock_pins? ?-async_pins? ?-output_pins? ?-level_sensitive?",
        ),
        _sdc("all_clocks", "Return all defined clocks.", "all_clocks"),
        _sdc(
            "all_fanout",
            "Return all fanout of a pin/port.",
            "all_fanout ?-from objects? ?-flat? ?-endpoints_only? ?-only_cells?",
        ),
        _sdc(
            "all_fanin",
            "Return all fanin of a pin/port.",
            "all_fanin ?-to objects? ?-flat? ?-startpoints_only? ?-only_cells?",
        ),
        # --- Collection utilities ---
        _sdc(
            "foreach_in_collection",
            "Iterate over objects in a collection.",
            "foreach_in_collection var collection body",
            Arity(3, 3),
        ),
        _sdc(
            "get_object_name",
            "Return the name of an object.",
            "get_object_name object",
            Arity(1, 1),
        ),
        _sdc(
            "sizeof_collection",
            "Return the size of a collection.",
            "sizeof_collection collection",
            Arity(1, 1),
        ),
        _sdc(
            "filter_collection",
            "Filter a collection by an expression.",
            "filter_collection collection filter_expr",
            Arity(2, 2),
        ),
        # --- Analysis & reporting ---
        _sdc(
            "report_timing",
            "Report timing paths.",
            "report_timing ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-sort_by attr?",
        ),
        _sdc("report_area", "Report design area.", "report_area ?-hierarchy?"),
        _sdc("report_power", "Report power consumption.", "report_power ?-hierarchy? ?-verbose?"),
        _sdc(
            "report_constraint",
            "Report design constraints.",
            "report_constraint ?-all_violators? ?-max_delay? ?-min_delay?",
        ),
        _sdc("report_clock", "Report clock definitions.", "report_clock ?clock_list?"),
        _sdc(
            "report_clock_timing",
            "Report clock timing characteristics.",
            "report_clock_timing ?-type type?",
        ),
        _sdc(
            "check_timing",
            "Check for unconstrained paths and issues.",
            "check_timing ?-verbose? ?-override_defaults list?",
        ),
        # --- Misc SDC ---
        _sdc("current_design", "Get or set the current design.", "current_design ?design_name?"),
        _sdc(
            "link_design",
            "Link the design to its library cells.",
            "link_design ?-keep_sub_designs?",
        ),
        _sdc(
            "set_units",
            "Set the unit specifications.",
            "set_units ?-time unit? ?-capacitance unit? ?-resistance unit? ?-voltage unit? ?-current unit? ?-power unit?",
        ),
        _sdc(
            "define_proc_attributes",
            "Define attributes for a procedure.",
            "define_proc_attributes proc_name ?-info string?",
            Arity(1),
        ),
        _sdc(
            "set_case_analysis",
            "Set constant case analysis on a port/pin.",
            "set_case_analysis value port_pin_list",
        ),
        _sdc(
            "set_disable_timing",
            "Disable timing arcs.",
            "set_disable_timing ?-from from_pin? ?-to to_pin? object_list",
        ),
        _sdc(
            "set_ideal_network",
            "Mark a network as ideal.",
            "set_ideal_network ?-no_propagate? object_list",
        ),
        _sdc(
            "group_path",
            "Group paths for multi-scenario analysis.",
            "group_path ?-name group_name? ?-from from_list? ?-through through_list? ?-to to_list? ?-weight weight? ?-default?",
        ),
        _sdc(
            "set_wire_load_model",
            "Set the wire load model.",
            "set_wire_load_model -name model_name ?-library lib? ?object_list?",
        ),
        _sdc(
            "set_wire_load_mode", "Set the wire load mode.", "set_wire_load_mode mode", Arity(1, 1)
        ),
        _sdc("set_dont_touch", "Prevent optimization of cells/nets.", "set_dont_touch object_list"),
        _sdc("set_dont_use", "Prevent use of library cells.", "set_dont_use lib_cell_list"),
        _sdc(
            "set_size_only",
            "Mark cells as size-only (no restructuring).",
            "set_size_only object_list ?-all_instances?",
        ),
        _sdc(
            "set_ideal_latency",
            "Set ideal latency on a clock network.",
            "set_ideal_latency ?-rise | -fall? ?-min | -max? delay object_list",
        ),
    )
