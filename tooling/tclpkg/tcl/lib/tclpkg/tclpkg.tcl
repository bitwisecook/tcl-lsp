# tclpkg -- package loader. Sources all modules in dependency order.

package provide tclpkg 0.1.0

set _dir [file dirname [info script]]
foreach _mod {
    version
    ui
    manifest
    lockfile
    cas
    resolver
    fetchers
    registry
    venv
    cmd_pkg
    cmd_venv
} {
    source [file join $_dir ${_mod}.tcl]
}
unset _dir _mod
