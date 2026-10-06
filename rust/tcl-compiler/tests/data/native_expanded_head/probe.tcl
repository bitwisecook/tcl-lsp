proc expanded {} {{*}{set x} A; set x}
puts [list literal [catch {expanded} message] $message]
if {[llength [info commands ::tcl::unsupported::disassemble]]} {
 puts [list bytecode [::tcl::unsupported::disassemble proc expanded]]
}
proc generic_mutation {} {{*}{set x} [rename set saved; saved value A]}
puts [list expanded_mutation [catch {generic_mutation} message] $message]
if {[llength [info commands saved]]} {rename saved set}
proc inline_mutation {} {set x [rename set saved; saved value A]}
puts [list inline_mutation [catch {inline_mutation} message] $message]
if {[llength [info commands saved]]} {rename saved set}
proc dynamic_mutation {command} {{*}$command [rename set saved; saved value A]}
puts [list dynamic_mutation [catch {dynamic_mutation {set x}} message] $message]
if {[llength [info commands saved]]} {rename saved set}
if {[llength [info commands ::tcl::unsupported::disassemble]]} {
 puts [list dynamic_bytecode [::tcl::unsupported::disassemble proc dynamic_mutation]]
}
proc dynamic_empty {command} {{*}$command set x A; set x}
puts [list dynamic_empty [catch {dynamic_empty {}} message] $message]
