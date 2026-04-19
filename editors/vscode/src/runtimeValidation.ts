export type RuntimeValidationAdapter = "tcl-syntax" | "irules-stub";

export type RuntimeValidationAdapterMode = "auto" | RuntimeValidationAdapter;

const IRULES_DIALECTS = new Set(["f5-irules"]);

const BASE_CHECKER_SCRIPT =
  "set path [lindex $argv 0]\n" +
  'if {$path eq ""} { puts stderr "Missing script path"; exit 2 }\n' +
  "set f [open $path r]\n" +
  "set script [read $f]\n" +
  "close $f\n" +
  'if {![info complete $script]} { puts stderr "Incomplete or unbalanced Tcl script"; exit 1 }\n';

const TCL_SYNTAX_CHECKER_SCRIPT = BASE_CHECKER_SCRIPT + 'puts "OK"\n' + "exit 0\n";

const IRULES_STUB_CHECKER_SCRIPT =
  BASE_CHECKER_SCRIPT +
  'proc unknown {args} { return "" }\n' +
  "proc when {event args} {\n" +
  "    if {[llength $args] == 1} {\n" +
  "        set body [lindex $args 0]\n" +
  '    } elseif {[llength $args] == 3 && [lindex $args 0] eq "priority"} {\n' +
  "        set priority [lindex $args 1]\n" +
  "        if {![string is integer -strict $priority]} {\n" +
  "            error \"Invalid priority '$priority' for event '$event'\"\n" +
  "        }\n" +
  "        set body [lindex $args 2]\n" +
  "    } else {\n" +
  "        error \"Invalid when syntax for event '$event'\"\n" +
  "    }\n" +
  "    if {![info complete $body]} {\n" +
  "        error \"Incomplete body for event '$event'\"\n" +
  "    }\n" +
  '    set procName "__tcl_lsp_validate_when__[clock clicks]"\n' +
  "    proc $procName {} $body\n" +
  "    rename $procName {}\n" +
  '    return ""\n' +
  "}\n" +
  "if {[catch {uplevel #0 $script} err]} {\n" +
  "    puts stderr $err\n" +
  "    exit 1\n" +
  "}\n" +
  'puts "OK"\n' +
  "exit 0\n";

export function resolveRuntimeValidationAdapter(
  mode: RuntimeValidationAdapterMode,
  dialect: string,
): RuntimeValidationAdapter {
  if (mode !== "auto") {
    return mode;
  }
  if (IRULES_DIALECTS.has(dialect)) {
    return "irules-stub";
  }
  return "tcl-syntax";
}

export function buildRuntimeValidationChecker(adapter: RuntimeValidationAdapter): string {
  return adapter === "irules-stub" ? IRULES_STUB_CHECKER_SCRIPT : TCL_SYNTAX_CHECKER_SCRIPT;
}

export function runtimeValidationAdapterLabel(adapter: RuntimeValidationAdapter): string {
  switch (adapter) {
    case "irules-stub":
      return "iRules stub adapter";
    case "tcl-syntax":
    default:
      return "Tcl syntax adapter";
  }
}
