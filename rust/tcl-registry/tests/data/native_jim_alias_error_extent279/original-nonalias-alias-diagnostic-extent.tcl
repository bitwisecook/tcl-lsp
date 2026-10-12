if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    set rows {}
    set name [binary format H* 72323238365f657874656e74005441494c]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list nul-middle [string length $name] $code $message]
    set name [binary format H* 0072323238365f657874656e74]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list nul-leading [string length $name] $code $message]
    set name [binary format H* 72323238365fc3a9]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list utf8 [string length $name] $code $message]
    set name [binary format H* 72323238365fff]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list invalid-byte [string length $name] $code $message]
    set rows
}
