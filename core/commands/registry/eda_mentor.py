"""Mentor/Siemens EDA Tcl commands — ModelSim, Questa, Calibre.

Vendor-specific commands for the ``mentor-eda-tcl`` dialect.
SDC base commands are provided separately by ``eda_sdc_base``.
"""

from __future__ import annotations

from .models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from .signatures import Arity

_SOURCE = "Siemens EDA ModelSim / Questa / Calibre"
_DIALECT = frozenset({"mentor-eda-tcl"})


def _mtr(
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


def mentor_command_specs() -> tuple[CommandSpec, ...]:
    """Return Mentor/Siemens-specific command specs."""
    return (
        # --- Compilation ---
        _mtr(
            "vlib",
            "Create a design library directory.",
            "vlib ?-type type? library_name",
            Arity(1, 1),
        ),
        _mtr(
            "vmap",
            "Map a logical library name to a directory.",
            "vmap ?-c? ?-del? library_name ?library_path?",
            Arity(1),
        ),
        _mtr(
            "vcom",
            "Compile VHDL source files.",
            "vcom ?-work library? ?-2008? ?-explicit? ?-check_synthesis? ?-lint? ?-suppress n? ?-nowarn n? file_list",
        ),
        _mtr(
            "vlog",
            "Compile Verilog/SystemVerilog source files.",
            "vlog ?-work library? ?-sv? ?+define+name=val? ?+incdir+dir? ?-lint? ?-suppress n? ?-nowarn n? file_list",
        ),
        _mtr(
            "vopt",
            "Optimize a compiled design for simulation.",
            "vopt ?+acc? ?-o optimized_name? ?-debugdb? ?-L library? top_module",
        ),
        _mtr(
            "vdel",
            "Delete a compiled library or design unit.",
            "vdel ?-lib library? ?-all? ?design_unit?",
        ),
        # --- Simulation control ---
        _mtr(
            "vsim",
            "Load and start simulation.",
            "vsim ?-c? ?-do command? ?-t time_resolution? ?-voptargs args? ?-L library? ?-debugdb? ?-wlf file? ?-onfinish action? ?-gui? ?work.top_module?",
        ),
        _mtr("run", "Advance simulation by specified time.", "run ?time_value? ?-all? ?-continue?"),
        _mtr(
            "restart",
            "Restart the current simulation.",
            "restart ?-force? ?-nowave? ?-nolist? ?-nolog? ?-nobreakpoint? ?-nokill?",
        ),
        _mtr(
            "examine",
            "Examine the value of a signal or variable.",
            "examine ?-radix radix? ?-time time? signal_name",
        ),
        _mtr(
            "force",
            "Force a signal to a value.",
            "force ?-freeze | -drive | -deposit? signal_name value ?time? ?-repeat period? ?-cancel period?",
        ),
        _mtr("release", "Release a previously forced signal.", "release signal_name"),
        _mtr(
            "change", "Change the value of a VHDL signal or variable.", "change signal_name value"
        ),
        # --- Waveform window ---
        _mtr(
            "add_wave",
            "Add signals to the wave window (add wave).",
            "add wave ?-position pos? ?-radix radix? ?-format format? ?-label label? ?-divider name? ?-group name? signal_list",
        ),
        _mtr(
            "add_list",
            "Add signals to the list window (add list).",
            "add list ?-radix radix? signal_list",
        ),
        _mtr("add_log", "Log signals for waveform viewing (add log).", "add log ?-r? signal_list"),
        _mtr("wave", "Waveform window command.", "wave ?subcommand? ?args ...?"),
        _mtr(
            "virtual",
            "Create virtual signals or regions.",
            "virtual ?-install | -env env? ?signal | function? ?-name name?",
        ),
        # --- Breakpoints & flow control ---
        _mtr("bp", "Set a breakpoint.", "bp ?file_name? ?line_number? ?-cond condition?"),
        _mtr("bd", "Delete breakpoints.", "bd ?breakpoint_id | -all?"),
        _mtr("bc", "Clear (disable) breakpoints.", "bc ?breakpoint_id | -all?"),
        _mtr("be", "Enable breakpoints.", "be ?breakpoint_id | -all?"),
        _mtr("bl", "List all breakpoints.", "bl"),
        _mtr("onbreak", "Define action when a breakpoint is hit.", "onbreak {command}"),
        _mtr("resume", "Resume simulation from a breakpoint.", "resume"),
        _mtr(
            "when",
            "Define conditional actions during simulation.",
            "when {condition} {action} ?-label label?",
        ),
        _mtr(
            "transcript",
            "Control transcript file output.",
            "transcript ?on | off | file file_name?",
        ),
        # --- Signal operations ---
        _mtr(
            "find",
            "Find signals matching a pattern.",
            "find ?-recursive? ?-type type? ?-ports? ?-signals? ?-internal? pattern",
        ),
        _mtr(
            "describe",
            "Show type and value information for a signal.",
            "describe signal_name",
            Arity(1, 1),
        ),
        _mtr("drivers", "Find drivers of a signal.", "drivers signal_name", Arity(1, 1)),
        _mtr("readers", "Find readers of a signal.", "readers signal_name", Arity(1, 1)),
        # --- Signal spy / force ---
        _mtr(
            "signal_force",
            "Force a signal value using SignalSpy.",
            "signal_force signal_name value ?time? ?-freeze | -drive | -deposit?",
        ),
        _mtr("signal_release", "Release a SignalSpy-forced signal.", "signal_release signal_name"),
        _mtr(
            "init_signal_spy",
            "Initialize signal spy for cross-hierarchy access.",
            "init_signal_spy src_signal dst_signal ?-node? ?-verbose?",
            Arity(2),
        ),
        _mtr(
            "init_signal_driver",
            "Initialize signal driver for cross-hierarchy driving.",
            "init_signal_driver src_signal dst_signal ?-default default_value? ?-delay delay?",
            Arity(2),
        ),
        # --- Coverage ---
        _mtr(
            "coverage",
            "Configure or report code coverage.",
            "coverage ?save | report | reload | clear? ?-file file? ?-directive? ?-comments?",
        ),
        _mtr(
            "vcover",
            "Process coverage data files.",
            "vcover ?merge | report | attr? ?-input file_list? ?-output file? ?-detail? ?-verbose?",
        ),
        _mtr("toggle", "Report toggle coverage statistics.", "toggle ?signal_name? ?-report?"),
        # --- Questa advanced ---
        _mtr(
            "qverilog",
            "Questa one-step Verilog compile and simulate.",
            "qverilog ?-sv? ?+define+name=val? ?-R? ?-c? file_list",
        ),
        _mtr(
            "qvhdl",
            "Questa one-step VHDL compile and simulate.",
            "qvhdl ?-2008? ?-R? ?-c? file_list",
        ),
        _mtr(
            "qrun",
            "Questa unified compile/optimize/simulate command.",
            "qrun ?-f file? ?-clean? ?-sv? ?-optimize? ?-top top? file_list",
        ),
        _mtr("qwave", "Questa waveform viewer command.", "qwave ?subcommand? ?args ...?"),
        # --- Questa Formal ---
        _mtr(
            "formal_compile",
            "Compile design for formal verification.",
            "formal_compile ?-d design? ?-work library?",
        ),
        _mtr(
            "formal_verify",
            "Run formal verification.",
            "formal_verify ?-timeout time? ?-engine engine?",
        ),
        _mtr(
            "formal_analyze",
            "Analyze formal verification results.",
            "formal_analyze ?-property prop_list?",
        ),
        # --- Calibre (physical verification) ---
        _mtr(
            "calibre",
            "Run Calibre physical verification.",
            "calibre ?-drc | -lvs | -pex? ?-hier? ?-turbo? ?-hyper? rule_file",
        ),
        _mtr(
            "calibre_drc",
            "Run Calibre DRC (design rule check).",
            "calibre_drc ?-hier? ?-turbo? rule_file",
        ),
        _mtr(
            "calibre_lvs",
            "Run Calibre LVS (layout vs schematic).",
            "calibre_lvs ?-hier? ?-turbo? rule_file",
        ),
        _mtr(
            "calibre_pex",
            "Run Calibre PEX (parasitic extraction).",
            "calibre_pex ?-hier? ?-turbo? rule_file",
        ),
    )
