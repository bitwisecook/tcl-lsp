"""Xilinx/AMD EDA Tcl commands — Vivado, Vitis.

Vendor-specific commands for the ``xilinx-eda-tcl`` dialect.
SDC base commands are provided separately by ``eda_sdc_base``.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "AMD/Xilinx Vivado Design Suite (UG835)"
_DIALECT = frozenset({"xilinx-eda-tcl"})


def _xil(
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


def xilinx_command_specs() -> tuple[CommandSpec, ...]:
    """Return Xilinx/AMD Vivado-specific command specs."""
    return (
        # --- Project management ---
        _xil(
            "create_project",
            "Create a new Vivado project.",
            "create_project ?-force? ?-part part? ?-in_memory? project_name ?project_dir?",
            Arity(1),
        ),
        _xil(
            "open_project",
            "Open an existing Vivado project.",
            "open_project ?-read_only? ?-quiet? project_file",
            Arity(1, 1),
        ),
        _xil("close_project", "Close the current project.", "close_project ?-quiet?"),
        _xil(
            "save_project_as",
            "Save the project with a new name.",
            "save_project_as ?-force? project_name ?project_dir?",
        ),
        _xil(
            "current_project", "Get or set the current project.", "current_project ?project_name?"
        ),
        _xil(
            "get_projects",
            "Get all open projects.",
            "get_projects ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
        ),
        _xil(
            "set_property",
            "Set a property on a Vivado design object.",
            "set_property property_name value objects",
            Arity(3),
        ),
        _xil(
            "get_property",
            "Get a property from a Vivado design object.",
            "get_property property_name object",
            Arity(2, 2),
        ),
        # --- Source file management ---
        _xil(
            "read_verilog",
            "Add Verilog source files to the project.",
            "read_verilog ?-sv? ?-library lib? file_list",
        ),
        _xil(
            "read_vhdl",
            "Add VHDL source files to the project.",
            "read_vhdl ?-library lib? ?-vhdl2008? file_list",
        ),
        _xil(
            "read_xdc",
            "Read XDC timing constraint files.",
            "read_xdc ?-unmanaged? ?-ref ref_name? ?-cells cell? file_list",
        ),
        _xil("read_ip", "Read IP core files (.xci).", "read_ip ?-quiet? file_list"),
        _xil(
            "read_checkpoint",
            "Read a design checkpoint (.dcp).",
            "read_checkpoint ?-cell cell? file_name",
            Arity(1),
        ),
        _xil("read_edif", "Read an EDIF netlist file.", "read_edif file_name", Arity(1, 1)),
        # --- Design flow ---
        _xil(
            "synth_design",
            "Run synthesis.",
            "synth_design ?-top top_module? ?-part part? ?-flatten_hierarchy value? ?-fanout_limit n? ?-retiming? ?-directive directive? ?-mode mode?",
        ),
        _xil(
            "opt_design",
            "Run logic optimization after synthesis.",
            "opt_design ?-directive directive? ?-retarget? ?-propconst? ?-sweep? ?-bram_power_opt? ?-remap?",
        ),
        _xil(
            "place_design",
            "Run placement.",
            "place_design ?-directive directive? ?-no_timing_driven? ?-unplace?",
        ),
        _xil(
            "phys_opt_design",
            "Run physical optimization after placement.",
            "phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt? ?-dsp_register_opt? ?-bram_register_opt? ?-hold_fix?",
        ),
        _xil(
            "route_design",
            "Run routing.",
            "route_design ?-directive directive? ?-nets net_list? ?-unroute? ?-preserve?",
        ),
        _xil(
            "write_bitstream",
            "Generate a bitstream file.",
            "write_bitstream ?-force? ?-bin_file? ?-no_partial_bitfile? ?-cell cell? file_name",
            Arity(1),
        ),
        _xil(
            "write_checkpoint",
            "Save a design checkpoint (.dcp).",
            "write_checkpoint ?-force? ?-cell cell? file_name",
            Arity(1),
        ),
        _xil(
            "open_checkpoint",
            "Open a design checkpoint for editing.",
            "open_checkpoint file_name",
            Arity(1, 1),
        ),
        # --- IP management ---
        _xil(
            "create_ip",
            "Create an IP core instance.",
            "create_ip -name name -vendor vendor -library library -version version ?-module_name mod_name? ?-dir dir?",
        ),
        _xil(
            "generate_target",
            "Generate IP output products.",
            "generate_target target_type ?-force? objects",
        ),
        _xil(
            "get_ips",
            "Get IP core instances.",
            "get_ips ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
        ),
        _xil(
            "upgrade_ip",
            "Upgrade IP cores to a newer version.",
            "upgrade_ip ?-srcset srcset? ?-quiet? ?objects?",
        ),
        _xil(
            "import_ip",
            "Import an IP core into the project.",
            "import_ip ?-srcset srcset? file_list",
        ),
        _xil(
            "config_ip_cache",
            "Configure the IP cache settings.",
            "config_ip_cache ?-import_from_project? ?-clear_output_repo? ?-cache_location dir?",
        ),
        # --- Reports ---
        _xil(
            "report_timing",
            "Report timing paths.",
            "report_timing ?-from from? ?-to to? ?-through through? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-sort_by attr? ?-file file? ?-name name?",
        ),
        _xil(
            "report_timing_summary",
            "Report comprehensive timing summary.",
            "report_timing_summary ?-delay_type type? ?-max_paths n? ?-check_timing_verbose? ?-file file? ?-name name?",
        ),
        _xil(
            "report_utilization",
            "Report device utilization.",
            "report_utilization ?-hierarchical? ?-hierarchical_depth n? ?-file file? ?-name name?",
        ),
        _xil(
            "report_power",
            "Report power consumption.",
            "report_power ?-file file? ?-name name? ?-advisory?",
        ),
        _xil(
            "report_clock_networks",
            "Report clock network topology.",
            "report_clock_networks ?-file file? ?-name name?",
        ),
        _xil(
            "report_clock_utilization",
            "Report clock resource utilization.",
            "report_clock_utilization ?-file file? ?-name name?",
        ),
        _xil(
            "report_drc",
            "Run and report design rule checks.",
            "report_drc ?-ruledecks ruledeck_list? ?-file file? ?-name name?",
        ),
        _xil(
            "report_methodology",
            "Run and report methodology checks.",
            "report_methodology ?-file file? ?-name name?",
        ),
        _xil("report_io", "Report I/O port assignments.", "report_io ?-file file? ?-name name?"),
        _xil(
            "report_route_status",
            "Report routing status.",
            "report_route_status ?-file file? ?-name name?",
        ),
        _xil(
            "report_design_analysis",
            "Report design analysis metrics.",
            "report_design_analysis ?-timing? ?-logic_level_distribution? ?-file file? ?-name name?",
        ),
        # --- Block design (IPI) ---
        _xil(
            "create_bd_design",
            "Create a block design.",
            "create_bd_design ?-dir dir? design_name",
            Arity(1, 1),
        ),
        _xil(
            "open_bd_design",
            "Open a block design.",
            "open_bd_design ?-name name? file_name",
            Arity(1),
        ),
        _xil("save_bd_design", "Save the current block design.", "save_bd_design"),
        _xil(
            "create_bd_cell",
            "Create a block design cell (IP instance).",
            "create_bd_cell -type ip -vlnv vlnv cell_name",
            Arity(1),
        ),
        _xil(
            "create_bd_port",
            "Create a block design external port.",
            "create_bd_port -dir direction ?-type type? port_name",
            Arity(1),
        ),
        _xil(
            "create_bd_intf_port",
            "Create a block design interface port.",
            "create_bd_intf_port -mode mode -vlnv vlnv port_name",
            Arity(1),
        ),
        _xil(
            "connect_bd_net",
            "Connect block design nets.",
            "connect_bd_net ?-net net_name? pin_list",
        ),
        _xil(
            "connect_bd_intf_net",
            "Connect block design interface nets.",
            "connect_bd_intf_net ?-intf_net net_name? intf_pin_list",
        ),
        _xil(
            "apply_bd_automation",
            "Apply block design automation rules.",
            "apply_bd_automation -rule rule_name ?-config config?",
        ),
        _xil("validate_bd_design", "Validate the block design.", "validate_bd_design ?-force?"),
        # --- Hardware manager ---
        _xil("open_hw_manager", "Open the hardware manager.", "open_hw_manager"),
        _xil(
            "connect_hw_server",
            "Connect to a hardware server.",
            "connect_hw_server ?-url url? ?-allow_non_jtag?",
        ),
        _xil(
            "open_hw_target", "Open a hardware target (JTAG chain).", "open_hw_target ?target_name?"
        ),
        _xil("program_hw_devices", "Program FPGA devices.", "program_hw_devices ?device_list?"),
        _xil("close_hw_manager", "Close the hardware manager.", "close_hw_manager"),
        # --- Run management ---
        _xil(
            "launch_runs",
            "Launch synthesis or implementation runs.",
            "launch_runs ?-jobs n? ?-scripts_only? ?-to_step step? run_list",
        ),
        _xil(
            "wait_on_run",
            "Wait for a run to complete.",
            "wait_on_run ?-timeout minutes? run_name",
            Arity(1, 1),
        ),
        _xil(
            "open_run",
            "Open a completed run in memory.",
            "open_run ?-name name? run_name",
            Arity(1, 1),
        ),
        _xil("reset_run", "Reset a run to its initial state.", "reset_run run_name", Arity(1, 1)),
        _xil("get_runs", "Get run objects.", "get_runs ?-regexp? ?-filter expr? ?patterns?"),
        _xil(
            "create_run",
            "Create a new run.",
            "create_run -flow flow ?-strategy strategy? ?-constrset constrset? ?-parent_run parent? run_name",
            Arity(1),
        ),
        # --- Simulation ---
        _xil(
            "launch_simulation",
            "Launch a simulation.",
            "launch_simulation ?-mode mode? ?-scripts_only? ?-simset simset?",
        ),
        _xil("close_sim", "Close the current simulation.", "close_sim ?-force?"),
        # --- IP Packager ---
        _xil(
            "ipx::package_project",
            "Package a project as an IP core.",
            "ipx::package_project ?-root_dir dir? ?-import_files? ?-force?",
        ),
        _xil(
            "ipx::add_bus_interface",
            "Add a bus interface to a packaged IP.",
            "ipx::add_bus_interface -abstraction_type_vlnv vlnv -bus_type_vlnv vlnv interface_name component",
        ),
    )
