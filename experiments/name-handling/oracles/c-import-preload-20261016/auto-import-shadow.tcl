puts [list provider [info patchlevel]]
proc auto_import {p} {error GLOBAL}
namespace eval dest {
    proc auto_import {p} {error LOCAL}
    set code [catch {namespace import ::missing::*} result]
    puts [list $code $result]
}
