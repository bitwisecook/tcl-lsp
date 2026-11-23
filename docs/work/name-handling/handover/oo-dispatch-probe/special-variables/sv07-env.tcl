# env is a traced array linked to the process environment.
puts "array=[array exists env] path=[info exists env(PATH)]"
set env(TCL_LSP_PROBE) hello
puts "child=[exec sh -c {echo $TCL_LSP_PROBE}]"
unset env(TCL_LSP_PROBE)
puts "child-after=[exec sh -c {echo x${TCL_LSP_PROBE}x}]"
puts "missing=[info exists env(TCL_LSP_NOT_SET_EVER)]"
proc p {} { global env; return [info exists env(PATH)] }
puts "proc-global=[p]"
proc q {} { return [info exists env(PATH)] }
puts "proc-local=[q]"
