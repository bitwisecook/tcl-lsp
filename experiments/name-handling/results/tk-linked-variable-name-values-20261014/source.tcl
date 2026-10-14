package require Tk
puts [list PROVIDER Tcl [info patchlevel] Tk [package provide Tk] LOADED [info loaded]]
set mapping [open /proc/[pid]/maps r]
foreach line [split [read $mapping] \n] {
    if {[regexp {/[^ ]+\.so[^ ]*} $line library]} {
        puts [list LOADED_LIBRARY $library]
    }
}
close $mapping
set {} ORIGINAL_EMPTY
foreach {name marker} {{} EMPTY {$literal} SIGIL {items(key)} ELEMENT} {
    entry .original -textvariable $name
    .original insert 0 $marker
    set code [catch {set $name} value]
    puts [list LINKED_OPTION $name [.original cget -textvariable] [.original get] $code $value]
    destroy .original
}
namespace eval original_scope {
    entry .original -textvariable global_link
    .original insert 0 GLOBAL_VALUE
    puts [list GLOBAL_LINK [info exists ::global_link] [info exists ::original_scope::global_link] [set ::global_link]]
    destroy .original
}
destroy .
