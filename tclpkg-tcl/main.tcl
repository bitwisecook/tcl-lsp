#!/usr/bin/env tclsh
# tclpkg — pure-Tcl package manager and virtual environment tool.
#
# Usage:
#   tclsh main.tcl pkg <subcommand> [args...]
#   tclsh main.tcl venv <subcommand> [args...]
#
# Requires Tcl 8.6+ and tcllib (json, http, sha256, fileutil).

package require Tcl 8.6

set script_dir [file dirname [file normalize [info script]]]
lappend auto_path [file join $script_dir lib]

package require tclpkg

namespace eval ::tclpkg::cli {
    proc usage {} {
        puts stderr "Usage: tclsh main.tcl <command> <subcommand> \[options\]"
        puts stderr ""
        puts stderr "Commands:"
        puts stderr "  pkg   Manage Tcl packages and lockfiles"
        puts stderr "  venv  Manage Tcl virtual environments"
        puts stderr ""
        puts stderr "Run 'tclsh main.tcl <command> --help' for subcommand help."
    }

    proc main {args} {
        if {[llength $args] < 1} {
            usage
            exit 2
        }
        set cmd [lindex $args 0]
        set rest [lrange $args 1 end]
        switch -- $cmd {
            pkg  { ::tclpkg::cmd_pkg::dispatch $rest }
            venv { ::tclpkg::cmd_venv::dispatch $rest }
            --help - -h - help {
                usage
                exit 0
            }
            --version {
                puts "tclpkg 0.1.0 (pure Tcl)"
                exit 0
            }
            default {
                puts stderr "error: unknown command '$cmd'"
                usage
                exit 2
            }
        }
    }
}

::tclpkg::cli::main {*}$argv
