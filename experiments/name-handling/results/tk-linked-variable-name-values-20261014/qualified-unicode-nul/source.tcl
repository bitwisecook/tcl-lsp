package require Tk
puts [list PROVIDER Tcl [info patchlevel] Tk [package provide Tk] LOADED [info loaded]]
set mapping [open /proc/[pid]/maps r]
foreach line [split [read $mapping] \n] {
    if {[regexp {/[^ ]+\.so[^ ]*} $line library]} {
        puts [list LOADED_LIBRARY $library]
    }
}
close $mapping
namespace eval original_scope {}
set ::original_scope::literal PRIOR
set zero [format %c 0]
foreach {name marker} [list "original_scope::literal${zero}tail" QUALIFIED_NUL "original_scope::items${zero}tail(key)" ELEMENT_ROOT_NUL] {
 entry .original -textvariable $name
 .original insert 0 $marker
 set code [catch {set $name} value]
 puts [list QUALIFIED_UNICODE_NUL $name [string length $name] [.original cget -textvariable] [.original get] $code $value [set ::original_scope::literal]]
 destroy .original
}
destroy .
