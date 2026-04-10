# cmd_venv.tcl -- `tclpkg venv` subcommand handlers.

namespace eval ::tclpkg::cmd_venv {

    proc dispatch {args_list} {
        if {[llength $args_list] < 1} {
            usage
            exit 2
        }
        set subcmd [lindex $args_list 0]
        set rest [lrange $args_list 1 end]
        switch -- $subcmd {
            create     { cmd_create $rest }
            delete     { cmd_delete $rest }
            info       { cmd_info $rest }
            activate   { cmd_activate $rest }
            deactivate { cmd_deactivate $rest }
            list       { cmd_list $rest }
            update     { cmd_update $rest }
            run        { cmd_run $rest }
            --help - -h - help { usage; exit 0 }
            default {
                puts stderr "error: unknown venv subcommand '$subcmd'"
                usage
                exit 2
            }
        }
    }

    proc usage {} {
        puts stderr "Usage: tclpkg venv <subcommand> \[options\]"
        puts stderr ""
        puts stderr "Subcommands:"
        puts stderr "  create     Create a new virtual environment"
        puts stderr "  delete     Remove a virtual environment"
        puts stderr "  info       Show virtual environment details"
        puts stderr "  activate   Print activation script to stdout"
        puts stderr "  deactivate Print deactivation snippet"
        puts stderr "  list       List discoverable virtual environments"
        puts stderr "  update     Re-link a venv to a different tclsh"
        puts stderr "  run        Run a command inside a venv"
    }

    proc _parse_path {args_list {default ".venv"}} {
        if {[llength $args_list] > 0 && ![string match "--*" [lindex $args_list 0]]} {
            return [lindex $args_list 0]
        }
        return $default
    }

    proc cmd_create {args_list} {
        set path [_parse_path $args_list]
        set tcl_ver ""
        set prompt ""
        set system 0
        set idx [lsearch -exact $args_list "--tcl"]
        if {$idx >= 0} { set tcl_ver [lindex $args_list $idx+1] }
        set idx [lsearch -exact $args_list "--prompt"]
        if {$idx >= 0} { set prompt [lindex $args_list $idx+1] }
        if {"--system-site-packages" in $args_list} { set system 1 }

        set code [catch {
            set created [::tclpkg::venv::create $path -tcl $tcl_ver -system $system -prompt $prompt]
        } err]
        if {$code} {
            puts stderr "error: $err"
            exit 1
        }
        puts [::tclpkg::ui::ok "created $created"]
        puts [::tclpkg::ui::dim "  activate: source $created/bin/activate"]
    }

    proc cmd_delete {args_list} {
        set path [_parse_path $args_list]
        set code [catch { ::tclpkg::venv::delete $path } err]
        if {$code} {
            puts stderr "error: $err"
            exit 1
        }
        puts [::tclpkg::ui::ok "deleted $path"]
    }

    proc cmd_info {args_list} {
        set path [_parse_path $args_list]
        set code [catch {set config [::tclpkg::venv::read_config $path] } _ err]
        if {$code} {
            puts stderr "error: $err"
            exit 1
        }
        foreach key [lsort [dict keys $config]] {
            puts [format "  %-30s  %s" $key [dict get $config $key]]
        }
    }

    proc cmd_activate {args_list} {
        set path [file normalize [_parse_path $args_list]]
        set shell "bash"
        set idx [lsearch -exact $args_list "--shell"]
        if {$idx >= 0} { set shell [lindex $args_list $idx+1] }

        if {$shell eq "fish"} {
            set script_path [file join $path bin activate.fish]
        } else {
            set script_path [file join $path bin activate]
        }
        if {![file isfile $script_path]} {
            puts stderr "error: activation script not found: $script_path"
            exit 1
        }
        set fd [open [file join $script_path] r]
        puts [read $fd]
        close $fd
    }

    proc cmd_deactivate {args_list} {
        puts "deactivate"
    }

    proc cmd_list {args_list} {
        set candidates [list [file join [pwd] .venv] [file join [pwd] venv]]
        set home_pool [file join [file normalize ~] .tcl-venvs]
        if {[file isdirectory $home_pool]} {
            foreach d [glob -nocomplain -directory $home_pool *] {
                lappend candidates $d
            }
        }
        set found {}
        foreach candidate $candidates {
            set cfg [file join $candidate tclvenv.cfg]
            if {[file isfile $cfg]} {
                set config [::tclpkg::venv::read_config $candidate]
                set tcl_ver ""
                catch {set tcl_ver [dict get $config tcl_version] } _
                set prompt [file tail $candidate]
                catch {set prompt [dict get $config prompt] } _
                set active ""
                if {[info exists ::env(TCL_VENV)] && [file normalize $::env(TCL_VENV)] eq [file normalize $candidate]} {
                    set active "yes"
                }
                lappend found [list $candidate $tcl_ver $prompt $active]
            }
        }
        if {[llength $found] == 0} {
            puts "  No virtual environments found."
            return
        }
        puts [format "%-30s %-12s %-10s %-6s" PATH TCL PROMPT ACTIVE]
        foreach entry $found {
            lassign $entry path tcl_ver prompt active
            puts [format "%-30s %-12s %-10s %-6s" $path $tcl_ver $prompt $active]
        }
    }

    proc cmd_update {args_list} {
        set path [_parse_path $args_list]
        set new_tcl ""
        set idx [lsearch -exact $args_list "--tcl"]
        if {$idx >= 0} { set new_tcl [lindex $args_list $idx+1] }
        if {$new_tcl eq ""} {
            puts stderr "error: --tcl VERSION is required for update"
            exit 2
        }
        set venv_path [file normalize $path]
        set code [catch { ::tclpkg::venv::read_config $venv_path } err]
        if {$code} {
            puts stderr "error: $err"
            exit 1
        }
        set tclsh [::tclpkg::venv::find_tclsh $new_tcl]

        # Rewrite tclvenv.cfg.
        set cfg_file [file join $venv_path tclvenv.cfg]
        set fd [open [file join $cfg_file] r]
        set text [read $fd]
        close $fd
        set new_lines {}
        foreach line [split $text "\n"] {
            if {[string match "tcl_version*" $line]} {
                lappend new_lines "tcl_version = $new_tcl"
            } elseif {[string match "tcl_executable*" $line]} {
                lappend new_lines "tcl_executable = $tclsh"
            } else {
                lappend new_lines $line
            }
        }
        set fd [open [file join $cfg_file] w]
        puts -nonewline $fd [join $new_lines "\n"]
        close $fd

        # Rewrite tclsh wrapper.
        set wrapper [file join $venv_path bin tclsh]
        set content "#!/bin/sh\n# Auto-generated by tclpkg \u2014 do not edit.\nVENV=\"\$(cd \"\$(dirname \"\$0\")/..\" && pwd)\"\n: \"\${TCLLIBPATH:=}\"\nexport TCLLIBPATH=\"\$VENV/lib\${TCLLIBPATH:+ \$TCLLIBPATH}\"\nexport TCL_VENV=\"\$VENV\"\nexec \"$tclsh\" \"\$@\"\n"
        set fd [open [file join $wrapper] w]
        puts -nonewline $fd $content
        close $fd
        file attributes $wrapper -permissions 0755

        puts [::tclpkg::ui::ok "updated $venv_path to tclsh $new_tcl ($tclsh)"]
    }

    proc cmd_run {args_list} {
        set path [_parse_path $args_list]
        set venv_path [file normalize $path]
        set cfg [file join $venv_path tclvenv.cfg]
        if {![file isfile $cfg]} {
            puts stderr "error: not a tclpkg venv: $venv_path"
            exit 1
        }
        # Find the command after the path arg.
        set cmd_start 1
        if {[llength $args_list] > 0 && ![string match "--*" [lindex $args_list 0]]} {
            set cmd_start 1
        }
        set command [lrange $args_list $cmd_start end]
        # Strip leading -- if present.
        if {[llength $command] > 0 && [lindex $command 0] eq "--"} {
            set command [lrange $command 1 end]
        }
        if {[llength $command] == 0} {
            puts stderr "error: no command specified after --"
            exit 2
        }
        set lib_dir [file join $venv_path lib]
        set ::env(TCL_VENV) $venv_path
        set existing ""
        catch {set existing $::env(TCLLIBPATH) } _
        set ::env(TCLLIBPATH) "$lib_dir [string trim $existing]"
        set sep [expr {$::tcl_platform(platform) eq "windows" ? ";" : ":"}]
        set ::env(PATH) "[file join $venv_path bin]$sep$::env(PATH)"
        exec -- {*}$command >@ stdout 2>@ stderr
    }
}
