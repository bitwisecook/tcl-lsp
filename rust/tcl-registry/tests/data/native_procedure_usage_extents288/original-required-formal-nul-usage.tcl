set r2286_name r2286_required
set r2286_formal [binary format H* 61007461696c]
set r2286_parameters [list $r2286_formal b]
set r2286_definition_code [catch {proc $r2286_name $r2286_parameters {return VALUE}} r2286_definition]
if {$r2286_definition_code == 0} {
    set r2286_call_code [catch {$r2286_name} r2286_message]
} else {
    set r2286_call_code NOT_CALLED
    set r2286_message {}
}
list [string length $r2286_name] [string length $r2286_formal] $r2286_definition_code $r2286_definition $r2286_call_code $r2286_message
