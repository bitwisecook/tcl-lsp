set r2286_name [binary format H* 72323238365f63616c6c0068656164]
set r2286_formal x
set r2286_parameters [list $r2286_formal]
set r2286_definition_code [catch {proc $r2286_name $r2286_parameters {return VALUE}} r2286_definition]
if {$r2286_definition_code == 0} {
    set r2286_call_code [catch {$r2286_name} r2286_message]
} else {
    set r2286_call_code NOT_CALLED
    set r2286_message {}
}
list [string length $r2286_name] [string length $r2286_formal] $r2286_definition_code $r2286_definition $r2286_call_code $r2286_message
