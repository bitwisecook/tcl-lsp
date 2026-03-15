"""Intel/Altera Quartus EDA Tcl commands.

Vendor-specific commands for the ``intel-quartus-eda-tcl`` dialect.
SDC base commands are provided separately by ``eda_sdc_base``.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "Intel Quartus Prime Tcl API"
_DIALECT = frozenset({"intel-quartus-eda-tcl"})


def _qrt(
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


def quartus_command_specs() -> tuple[CommandSpec, ...]:
    """Return Intel Quartus-specific command specs."""
    return (
        # --- Project management (::quartus::project) ---
        _qrt(
            "project_new",
            "Create a new Quartus project.",
            "project_new ?-overwrite? ?-revision rev? ?-part part? ?-family family? project_name",
            Arity(1),
        ),
        _qrt(
            "project_open",
            "Open an existing Quartus project.",
            "project_open ?-revision rev? ?-current_revision? ?-force? project_name",
            Arity(1),
        ),
        _qrt(
            "project_close",
            "Close the current Quartus project.",
            "project_close ?-dont_export_assignments?",
        ),
        _qrt(
            "project_exists",
            "Check whether a Quartus project exists.",
            "project_exists project_name",
            Arity(1, 1),
        ),
        # --- Assignments (::quartus::project) ---
        _qrt(
            "set_global_assignment",
            "Set a global project assignment.",
            "set_global_assignment -name name value ?-section_id id?",
            Arity(1),
        ),
        _qrt(
            "get_global_assignment",
            "Get a global project assignment value.",
            "get_global_assignment -name name ?-section_id id?",
        ),
        _qrt(
            "set_instance_assignment",
            "Set an assignment on a specific instance.",
            "set_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id? value",
        ),
        _qrt(
            "get_instance_assignment",
            "Get an instance assignment value.",
            "get_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id?",
        ),
        _qrt(
            "set_location_assignment",
            "Set a location assignment for a pin.",
            "set_location_assignment -to pin_name location",
            Arity(1),
        ),
        _qrt(
            "set_parameter",
            "Set a parameter on a design entity.",
            "set_parameter -name name -to instance value",
        ),
        _qrt("export_assignments", "Export assignments to the .qsf file.", "export_assignments"),
        _qrt(
            "get_all_assignments",
            "Get all assignments matching criteria.",
            "get_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?",
        ),
        _qrt(
            "remove_all_assignments",
            "Remove all assignments matching criteria.",
            "remove_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?",
        ),
        _qrt(
            "set_io_assignment",
            "Set an I/O assignment.",
            "set_io_assignment -name name -to pin_name value",
        ),
        _qrt(
            "get_io_assignment",
            "Get an I/O assignment value.",
            "get_io_assignment -name name -to pin_name",
        ),
        # --- Flow execution (::quartus::flow) ---
        _qrt(
            "execute_flow",
            "Execute a Quartus compilation flow.",
            "execute_flow -compile | -analysis_and_synthesis | -fitter | -assembler | -timing_analyzer | -eda_netlist_writer | -signaltap | -export_database",
        ),
        _qrt(
            "execute_module",
            "Execute a specific Quartus module.",
            "execute_module -tool tool_name ?-args arg_list?",
        ),
        # --- Package management ---
        _qrt(
            "load_package", "Load a Quartus Tcl package.", "load_package package_name", Arity(1, 1)
        ),
        # --- Device information (::quartus::device) ---
        _qrt(
            "get_part_list",
            "Get a list of available FPGA parts.",
            "get_part_list ?-family family? ?-speed_grade speed_grade?",
        ),
        _qrt(
            "get_part_info",
            "Get information about a specific FPGA part.",
            "get_part_info -family | -speed_grade | -package | -pin_count | -available_pin_count part_name",
        ),
        # --- Report access (::quartus::report) ---
        _qrt(
            "load_report", "Load a compilation report into memory.", "load_report ?revision_name?"
        ),
        _qrt("save_report", "Save the compilation report.", "save_report"),
        _qrt(
            "get_report_panel_data",
            "Get data from a specific report panel cell.",
            "get_report_panel_data -name panel_name -row row -col col",
        ),
        _qrt(
            "get_report_panel_id",
            "Get the ID of a report panel.",
            "get_report_panel_id panel_name",
            Arity(1, 1),
        ),
        _qrt(
            "get_report_panel_row_index",
            "Get the row index of matching data.",
            "get_report_panel_row_index -name panel_name row_name",
        ),
        _qrt(
            "get_number_of_rows",
            "Get the number of rows in a report panel.",
            "get_number_of_rows -name panel_name",
        ),
        _qrt(
            "get_number_of_columns",
            "Get the number of columns in a report panel.",
            "get_number_of_columns -name panel_name",
        ),
        # --- Timing analysis (::quartus::sta) ---
        _qrt(
            "create_timing_netlist",
            "Create a timing netlist for analysis.",
            "create_timing_netlist ?-model model? ?-post_map | -post_fit?",
        ),
        _qrt(
            "read_sdc",
            "Read SDC constraint file for TimeQuest.",
            "read_sdc ?-echo? file_name",
            Arity(1),
        ),
        _qrt(
            "update_timing_netlist",
            "Update the timing netlist after changes.",
            "update_timing_netlist",
        ),
        _qrt(
            "delete_timing_netlist", "Delete the current timing netlist.", "delete_timing_netlist"
        ),
        _qrt(
            "report_timing",
            "Report timing paths.",
            "report_timing ?-from from_list? ?-to to_list? ?-through through_list? ?-setup | -hold? ?-npaths n? ?-detail detail? ?-panel_name name? ?-file file?",
        ),
        _qrt(
            "report_min_pulse_width",
            "Report minimum pulse width violations.",
            "report_min_pulse_width ?-nworst n? ?-file file?",
        ),
        _qrt(
            "report_clock_fmax_summary",
            "Report maximum clock frequency summary.",
            "report_clock_fmax_summary ?-file file? ?-panel_name name?",
        ),
        _qrt(
            "report_ucp",
            "Report unconstrained paths.",
            "report_ucp ?-file file? ?-panel_name name?",
        ),
        _qrt(
            "report_datasheet",
            "Report I/O timing datasheet.",
            "report_datasheet ?-file file? ?-panel_name name?",
        ),
        _qrt("check_timing", "Check for timing analysis issues.", "check_timing ?-file file?"),
        # --- SDC extensions (::quartus::sdc) ---
        _qrt(
            "derive_clocks",
            "Automatically derive clocks from the design.",
            "derive_clocks ?-period period?",
        ),
        _qrt(
            "derive_pll_clocks",
            "Automatically derive PLL output clocks.",
            "derive_pll_clocks ?-create_base_clocks? ?-use_net_name?",
        ),
        # --- JTAG (::quartus::jtag) ---
        _qrt(
            "open_device",
            "Open a device on the JTAG chain.",
            "open_device ?-hardware_name name? ?-device_name name?",
        ),
        _qrt("close_device", "Close the active JTAG device.", "close_device"),
        _qrt(
            "device_lock",
            "Lock a JTAG device for exclusive access.",
            "device_lock ?-timeout seconds?",
        ),
        _qrt("device_unlock", "Unlock a previously locked JTAG device.", "device_unlock"),
        # --- Names/nodes (::quartus::names) ---
        _qrt(
            "get_names",
            "Get signal names from the design.",
            "get_names ?-filter filter? ?-node_type type? ?-observable_type type?",
        ),
        _qrt(
            "get_name_info",
            "Get detailed information about a named signal.",
            "get_name_info -info info_type name_id",
        ),
        # --- ECO (::quartus::eco) ---
        _qrt(
            "make_connection",
            "Make a connection in an ECO change.",
            "make_connection -from source -to destination",
        ),
        _qrt(
            "remove_connection",
            "Remove a connection in an ECO change.",
            "remove_connection -from source -to destination",
        ),
        _qrt(
            "rename_node",
            "Rename a node in an ECO change.",
            "rename_node -from old_name -to new_name",
        ),
    )
