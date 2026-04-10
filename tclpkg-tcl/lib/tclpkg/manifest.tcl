# manifest.tcl — tclpkg.tcl manifest loader using safe interp.
#
# Evaluates the manifest in a Tcl safe interpreter with only the 13
# whitelisted directives exposed as aliases. This is the pure-Tcl
# equivalent of tclpkg/manifest.py and uses C Tcl's native safe
# interpreter — simpler than the Python version's VM-based sandbox.

namespace eval ::tclpkg::manifest {

    # Load a manifest file and return a dict.
    proc load {path} {
        if {![file isfile $path]} {
            error "manifest not found: $path (run 'tclpkg pkg init' to create one)"
        }
        set fd [open $path r]
        fconfigure $fd -encoding utf-8
        set text [read $fd]
        close $fd
        return [load_text $text $path]
    }

    # Parse manifest source text and return a dict.
    proc load_text {text {path ""}} {
        variable _ast

        # Initialise the AST accumulator.
        set _ast [dict create \
            name "" version "" description "" license "" \
            authors {} homepage "" tcl_constraint ">=8.6" \
            requires {} dev_requires {} replaces {} excludes {} \
            provides {} entry "" path $path]

        # Create a safe child interp.
        set child [interp create -safe]

        # Hide everything, then expose only our directives.
        foreach cmd {package version description license author homepage
                     tcl require dev-require replace exclude provides entry} {
            interp alias $child $cmd {} [namespace current]::_directive_$cmd
        }
        # dev-require has a hyphen — alias it via a helper name.
        interp alias $child dev-require {} [namespace current]::_directive_dev-require

        # Evaluate.
        set code [catch {interp eval $child $text} result opts]
        interp delete $child

        if {$code != 0} {
            set msg $result
            if {$path ne ""} {
                set msg "$path: $msg"
            }
            error $msg
        }

        # Validate required fields.
        if {[dict get $_ast name] eq ""} {
            error "manifest missing required 'package' directive"
        }
        if {[dict get $_ast version] eq ""} {
            error "manifest missing required 'version' directive"
        }

        # Validate version parses.
        ::tclpkg::version::parse [dict get $_ast version]

        return $_ast
    }

    # Directive handlers — each appends to the _ast variable.
    proc _directive_package {name} {
        variable _ast
        if {[dict get $_ast name] ne ""} {
            error "package directive already set"
        }
        dict set _ast name $name
    }

    proc _directive_version {ver} {
        variable _ast
        # Validate it parses.
        ::tclpkg::version::parse $ver
        dict set _ast version $ver
    }

    proc _directive_description {text} {
        variable _ast
        dict set _ast description $text
    }

    proc _directive_license {spdx} {
        variable _ast
        if {![regexp {^[A-Za-z0-9.+ -]+$} $spdx]} {
            error "license: invalid SPDX-style identifier: '$spdx'"
        }
        dict set _ast license $spdx
    }

    proc _directive_author {name} {
        variable _ast
        dict lappend _ast authors $name
    }

    proc _directive_homepage {url} {
        variable _ast
        dict set _ast homepage $url
    }

    proc _directive_tcl {constraint} {
        variable _ast
        if {![regexp {^\s*(>=|>|==|=|<=|<)?\s*[0-9]} $constraint]} {
            error "tcl: invalid version constraint: '$constraint'"
        }
        # Normalise shorthand.
        if {![regexp {^(>=|>|==|=|<=|<)} [string trim $constraint]]} {
            set constraint ">=[string trim $constraint]"
        }
        dict set _ast tcl_constraint $constraint
    }

    proc _directive_require {args} {
        variable _ast
        _parse_require require $args
    }

    proc _directive_dev-require {args} {
        variable _ast
        _parse_require dev-require $args
    }

    proc _parse_require {kind arglist} {
        variable _ast
        set argc [llength $arglist]
        if {$argc < 2 || $argc > 4} {
            error "wrong # args: \"$kind\" expects name, minimum, ?-source URL?"
        }
        set source ""
        if {$argc >= 4 && [lindex $arglist end-1] eq "-source"} {
            set source [lindex $arglist end]
            set arglist [lrange $arglist 0 end-2]
        }
        if {[llength $arglist] != 2} {
            error "wrong # args: \"$kind\" expects name, minimum, ?-source URL?"
        }
        lassign $arglist name minver
        ::tclpkg::version::parse $minver
        set entry [dict create name $name minimum $minver source $source]
        if {$kind eq "dev-require"} {
            dict set entry dev 1
            dict lappend _ast dev_requires $entry
        } else {
            dict set entry dev 0
            dict lappend _ast requires $entry
        }
    }

    proc _directive_replace {name ver source} {
        variable _ast
        ::tclpkg::version::parse $ver
        dict lappend _ast replaces [dict create name $name version $ver source $source]
    }

    proc _directive_exclude {name ver} {
        variable _ast
        ::tclpkg::version::parse $ver
        dict lappend _ast excludes [dict create name $name version $ver]
    }

    proc _directive_provides {cmd} {
        variable _ast
        dict lappend _ast provides $cmd
    }

    proc _directive_entry {file} {
        variable _ast
        dict set _ast entry $file
    }
}
