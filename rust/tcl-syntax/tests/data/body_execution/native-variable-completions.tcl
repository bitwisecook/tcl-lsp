# Native variable primitives normalise trace failures to error completions.
foreach code {ok error return break continue 7} {
    proc variableCompletion args [list return -code $code trace-result]
    set observed 1
    if {[package vcompare [info tclversion] 9] < 0} {
        trace variable observed w variableCompletion
    } else {
        trace add variable observed write variableCompletion
    }
    set writeCode [catch {set observed 2}]
    set incrementCode [catch {incr observed}]
    if {[package vcompare [info tclversion] 9] < 0} {
        trace vdelete observed w variableCompletion
    } else {
        trace remove variable observed write variableCompletion
    }
    puts [list trace-$code $writeCode $incrementCode]
}
puts [list missing-read [catch {set never-created-variable}]]
set nonnumeric invalid
puts [list invalid-increment [catch {incr nonnumeric}]]
proc conflictingGlobal {} {set local existing; global local}
puts [list conflicting-global [catch conflictingGlobal]]
proc selfUpvar {} {upvar 0 local local}
puts [list self-upvar [catch selfUpvar]]

puts "proc-opaque-body [catch {proc native_definition {} {return -code 7}}]"
puts "proc-invalid-namespace [catch {proc ::missing_definition_namespace::bad {} {}}]"
puts "rename-normal [catch {rename native_definition moved_definition}]"
puts "rename-missing [catch {rename missing_definition moved_definition}]"
proc existing_definition {} {}
puts "rename-existing [catch {rename moved_definition existing_definition}]"
puts "rename-delete [catch {rename moved_definition {}}]"
