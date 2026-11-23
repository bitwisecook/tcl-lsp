# Writes the runtime observes without any script read (not dead stores).
set auto_path [linsert $auto_path 0 /nowhere]
set env(TCL_LSP_W) 1
set tcl_interactive 0
set errorInfo ""
set argv {}
puts ok
