set r2286_is_jim [string match 0.* [info patchlevel]]
if {$r2286_is_jim} {set r2286_child [interp]} else {set r2286_child [interp create]}
set r2286_name [binary format H* 72323238365fff]
set r2286_name_before_length [string length $r2286_name]
binary scan $r2286_name H* r2286_name_before_hex
if {$r2286_is_jim} {
    set r2286_definition_code [catch {$r2286_child alias $r2286_name list VALUE} r2286_definition_result]
    set r2286_call_code [catch {$r2286_child eval [list $r2286_name]} r2286_call_result]
    set r2286_command_code [catch {$r2286_child eval {info commands -all *r2286*}} r2286_command_result]
    set r2286_alias_code [catch {$r2286_child eval {info aliases}} r2286_alias_result]
    set r2286_other_alias_code [catch {interp aliases $r2286_child} r2286_other_alias_result]
} else {
    set r2286_definition_code [catch {interp alias $r2286_child $r2286_name {} list VALUE} r2286_definition_result]
    set r2286_call_code [catch {interp eval $r2286_child [list $r2286_name]} r2286_call_result]
    set r2286_command_code [catch {interp eval $r2286_child {info commands *r2286*}} r2286_command_result]
    set r2286_alias_code [catch {interp aliases $r2286_child} r2286_alias_result]
    set r2286_other_alias_code [catch {interp eval $r2286_child {info aliases}} r2286_other_alias_result]
}
set r2286_command_members {}
if {$r2286_command_code == 0} {
foreach r2286_member $r2286_command_result {
    if {$r2286_is_jim} {binary scan $r2286_member H* r2286_hex} else {
        binary scan [encoding convertto utf-8 $r2286_member] H* r2286_hex
    }
    lappend r2286_command_members [list [string length $r2286_member] $r2286_hex]
}
}
set r2286_alias_members {}
if {$r2286_alias_code == 0} {
foreach r2286_member $r2286_alias_result {
    if {$r2286_is_jim} {binary scan $r2286_member H* r2286_hex} else {
        binary scan [encoding convertto utf-8 $r2286_member] H* r2286_hex
    }
    lappend r2286_alias_members [list [string length $r2286_member] $r2286_hex]
}
}
if {$r2286_is_jim} {$r2286_child delete} else {interp delete $r2286_child}
list [list NAME_BEFORE $r2286_name_before_length $r2286_name_before_hex] [list DEFINITION $r2286_definition_code $r2286_definition_result] [list CALL $r2286_call_code $r2286_call_result] [list COMMANDS $r2286_command_code $r2286_command_result $r2286_command_members] [list ALIASES $r2286_alias_code $r2286_alias_result $r2286_alias_members] [list OTHER_ALIAS_SURFACE $r2286_other_alias_code $r2286_other_alias_result]
