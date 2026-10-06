# tmm_shim.tcl -- Make a stock tclsh look like BIG-IP TMM
#
# This file sets up the restricted Tcl environment that iRules run in
# on real BIG-IP devices.  TMM runs a modified Tcl 8.4 interpreter with
# a distinct loader-refused and interpreter-absent command surface.
#
# The framework itself runs on tclsh 8.5+ for convenience, but the
# environment presented to the iRule under test is strict Tcl 8.4 with
# TMM restrictions.
#
# What it does:
#   1. Completely disables commands that TMM removes (exec, socket,
#      open, file, source, glob, etc.); loader-refused commands remain runtime-callable.
#   2. Overrides [info] to report Tcl 8.4 and TMM-like values
#   3. Hides framework internals from [info commands] / [info procs]
#   4. Blocks Tcl 8.5+ features that would not exist on TMM
#
# Usage:
#   source tmm_shim.tcl
#   tmm::init ?-tmos_version 16.1?
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

# Source the registry data (disabled commands, post-8.4 commands, etc.).
# _registry_data.tcl is hand-maintained: no generator produces it, so there is
# no command to regenerate it with.  `cargo xtask gen-irule-test-data` writes
# _event_data.tcl, _mock_stubs.tcl, and _user_surface_data.tcl.
set _tmm_registry_file [file join [file dirname [info script]] _registry_data.tcl]
if {![file exists $_tmm_registry_file]} {
    error "Missing hand-maintained file _registry_data.tcl -- it is checked in; restore it from the repository (no generator produces it)"
}
source $_tmm_registry_file
unset _tmm_registry_file

set _tmm_surface_file [file join [file dirname [info script]] _user_surface_data.tcl]
if {![file exists $_tmm_surface_file]} {
    error "Missing generated file _user_surface_data.tcl -- run: cargo xtask gen-irule-test-data"
}
source $_tmm_surface_file
unset _tmm_surface_file

namespace eval ::tmm {

    # Configuration

    variable tmos_version "16.1.0"
    variable hostname     "bigip1.local"
    variable platform     "BIG-IP"

    # Tcl version TMM reports -- always 8.4 regardless of actual tclsh
    variable reported_tcl_version "8.4.6"
    variable reported_tcl_major  "8.4"

    # Commands completely disabled on TMM -- from generated registry data.
    variable compiler_refused_commands $_gen_runtime_compiler_refused
    variable runtime_namespace_members $_gen_runtime_namespace_members
    variable disabled_commands $_gen_disabled_commands

    # Commands that exist in 8.5+ but not in 8.4 TMM -- from generated registry data.
    variable post84_commands [concat $_gen_post84_commands $_gen_unavailable_modern_commands]

    # Helpers

    # Escape glob-special characters so operator-named commands like *,
    # ?, [, ] are not treated as patterns by [info commands].
    proc _glob_escape {s} {
        string map {* \\* ? \\? [ \\[ ] \\] \\ \\\\} $s
    }

    # Initialisation

    proc init {args} {
        variable tmos_version
        variable _initialized

        # Parse arguments
        foreach {opt val} $args {
            switch -exact -- $opt {
                -tmos_version { set tmos_version $val }
                -hostname     { variable hostname ; set hostname $val }
            }
        }

        # Guard against double-init (e.g. multiple ::orch::test calls)
        if {[info exists _initialized] && $_initialized} {
            return
        }

        _install_disabled_commands
        _install_post84_blocks
        _install_info_override
        _install_namespace_restriction
        _install_package_view
        _install_completion_capabilities

        set _initialized 1
        return
    }

    # The C host supplies native capabilities; the Rust embedder installs
    # equivalent captured native tokens before sourcing this framework.
    # Public completion grammar remains the iRules Tcl 8.4 grammar.
    proc _install_completion_capabilities {} {
        if {[llength [::tmm::_orig_info commands ::tmm::_host_catch]]} { return }
        if {[catch {catch {} result options}]} {
            error "iRules simulation requires a host with native completion options (Tcl 8.5+)"
        }
        ::tmm::_orig_rename ::catch ::tmm::_host_catch
        ::tmm::_orig_rename ::return ::tmm::_host_return
        proc ::catch {args} {
            if {[llength $args] < 1 || [llength $args] > 2} {
                error {wrong # args: should be "catch command ?varName?"}
            }
            return [uplevel 1 [linsert $args 0 ::tmm::_host_catch]]
        }
        proc ::return {args} {
            foreach {key value} [lrange $args 0 end-1] {
                if {[string match -* $key] && $key ni {-code -errorcode -errorinfo}} {
                    error "bad option \"$key\": must be -code, -errorcode, or -errorinfo"
                }
            }
            ::tmm::_host_catch {uplevel 1 [linsert $args 0 ::tmm::_host_return]} result options
            set level [::tmm::_orig_tcl::dict::get $options -level]
            incr level
            set options [::tmm::_orig_tcl::dict::replace $options -level $level]
            ::tmm::_host_return -options $options $result
        }
    }

    # Disabled commands

    proc _install_disabled_commands {} {
        variable disabled_commands
        variable compiler_refused_commands

        # Preserve a private alias for rename -- the loop itself needs it,
        # and the framework uses it later for post-8.4 blocks.
        if {[llength [::info commands ::rename]]} {
            ::rename ::rename ::tmm::_orig_rename
        }

        # Pre-create parent namespaces for any namespaced commands
        # (e.g. regex::quote → ::regex) before the loop disables namespace.
        foreach cmd $disabled_commands {
            if {[string first "::" $cmd] >= 0} {
                set ns [namespace qualifiers ::$cmd]
                if {$ns ne "" && $ns ne "::"} {
                    catch {namespace eval $ns {}}
                }
            }
        }

        foreach cmd $disabled_commands {
            # Skip rename -- handled above; we must keep our private copy.
            if {$cmd eq "rename"} continue

            # Save the original if it exists (for framework internal use)
            if {[llength [::info commands ::[_glob_escape $cmd]]]} {
                ::tmm::_orig_rename ::$cmd ::tmm::_orig_$cmd
            }
            if {[lsearch -exact $compiler_refused_commands $cmd] >= 0} {
                set body [format {return [uplevel 1 [linsert $args 0 ::tmm::_orig_%s]]} $cmd]
                proc ::$cmd {args} $body
                continue
            }
            # Interpreter-absent commands remain unavailable at runtime.
            set body [format {
                error "invalid command name \"%s\"" "invalid command name \"%s\""
            } $cmd $cmd]
            proc ::$cmd {args} $body
        }

        proc ::rename {args} {return [uplevel 1 [linsert $args 0 ::tmm::_orig_rename]]}
    }

    # Block Tcl 8.5+ commands
    #
    # If running on 8.5/8.6/9.0, remove commands that TMM 8.4 wouldn't have.
    # Blockers are namespace-aware: framework namespaces (::tmm::, ::itest::,
    # ::orch::, ::state::, ::proto::) may call through to the
    # original command. An explicit event-execution boundary blocks user
    # event code, including RULE_INIT in the interpreter root frame. Host
    # test code outside that boundary retains its modern command surface.
    #
    # The blocker body uses only Tcl 8.4 commands (string match, linsert,
    # uplevel) to avoid chicken-and-egg issues.

    # One alias spelling for root commands and qualified native members.
    proc _private_command_alias {canonical} {
        return "::tmm::_orig_[string range $canonical 2 end]"
    }

    proc _install_post84_blocks {} {
        variable post84_commands

        array set seen {}
        foreach cmd $post84_commands {
            set canonical [::tmm::_orig_namespace which -command ::$cmd]
            if {$canonical eq "" || [info exists seen($canonical)]} { continue }
            set seen($canonical) 1
            set private [_private_command_alias $canonical]
            if {[llength [::info commands [_glob_escape $private]]]} { continue }
            ::tmm::_orig_namespace eval [::tmm::_orig_namespace qualifiers $private] {}
            ::tmm::_orig_rename $canonical $private
                # Smart blocker: checks caller namespace at call time.
                # By call time, _install_namespace_restriction has run so
                # ::namespace (the wrapper) handles "current" correctly.
                ::set body [format {
                    ::set _caller_ns [::uplevel 1 {::namespace current}]
                    if {$_caller_ns eq "::tmm" || [::string match "::tmm::*" $_caller_ns] ||
                        $_caller_ns eq "::itest" || [::string match "::itest::*" $_caller_ns] ||
                        $_caller_ns eq "::orch" || [::string match "::orch::*" $_caller_ns] ||
                        $_caller_ns eq "::state" || [::string match "::state::*" $_caller_ns] ||
                        $_caller_ns eq "::proto" || [::string match "::proto::*" $_caller_ns]} {
                        ::return [::uplevel 1 [::linsert $args 0 %s]]
                    }
                    # The execution boundary is explicit: RULE_INIT uses
                    # the root namespace frame and has no handler proc frame.
                    if {![::info exists ::itest::_executing_rule] ||
                        !$::itest::_executing_rule} {
                        ::return [::uplevel 1 [::linsert $args 0 %s]]
                    }
                    ::error "invalid command name \"%s\"" "invalid command name \"%s\""
                } $private $private $canonical $canonical]
                proc $canonical {args} $body
        }
    }

    # info override

    proc _install_info_override {} {
        # Only rename once
        if {![llength [::info commands ::tmm::_orig_info]]} {
            ::tmm::_orig_rename ::info ::tmm::_orig_info
        }

        proc ::info {subcommand args} {
            variable ::tmm::reported_tcl_version
            variable ::tmm::reported_tcl_major
            variable ::tmm::hostname

            switch -exact -- $subcommand {
                nameofexecutable {
                    return "/usr/bin/tmm"
                }
                patchlevel {
                    return $reported_tcl_version
                }
                tclversion {
                    return $reported_tcl_major
                }
                hostname {
                    return $hostname
                }
                library {
                    return "/usr/share/tmm/tcl8.4"
                }
                loaded {
                    return {}
                }
                sharedlibextension {
                    return ".so"
                }
                commands {
                    # Get real commands, filter out framework internals
                    if {[llength $args] > 0} {
                        set result [::tmm::_orig_info commands [lindex $args 0]]
                    } else {
                        set result [::tmm::_orig_info commands]
                    }
                    set filtered [list]
                    foreach cmd $result {
                        if {[string match "::tmm::*" $cmd]} { continue }
                        if {[string match "tmm::*" $cmd]} { continue }
                        if {[string match "::state::*" $cmd]} { continue }
                        if {[string match "state::*" $cmd]} { continue }
                        if {[string match "::itest::*" $cmd]} { continue }
                        if {[string match "itest::*" $cmd]} { continue }
                        if {[string match "::_irh_*" $cmd]} { continue }
                        if {[string match "_irh_*" $cmd]} { continue }
                        lappend filtered $cmd
                    }
                    return $filtered
                }
                procs {
                    if {[llength $args] > 0} {
                        set result [::tmm::_orig_info procs [lindex $args 0]]
                    } else {
                        set result [::tmm::_orig_info procs]
                    }
                    set filtered [list]
                    foreach p $result {
                        if {[string match "::tmm::*" $p]} { continue }
                        if {[string match "tmm::*" $p]} { continue }
                        if {[string match "::state::*" $p]} { continue }
                        if {[string match "state::*" $p]} { continue }
                        if {[string match "::itest::*" $p]} { continue }
                        if {[string match "itest::*" $p]} { continue }
                        if {[string match "::_irh_*" $p]} { continue }
                        if {[string match "_irh_*" $p]} { continue }
                        lappend filtered $p
                    }
                    return $filtered
                }
                default {
                    # Pass through to real info in the CALLER's scope
                    # (uplevel is essential for scope-sensitive subcommands
                    # like exists, vars, locals, level, args, body, etc.)
                    if {[llength $args] > 0} {
                        return [uplevel 1 [linsert $args 0 ::tmm::_orig_info $subcommand]]
                    } else {
                        return [uplevel 1 [list ::tmm::_orig_info $subcommand]]
                    }
                }
            }
        }
    }

    # namespace restriction

    proc _select_namespace_member {original members} {
        if {[llength [::tmm::_orig_info commands ::tmm::_logical_namespace_member]]} {
            return [::tmm::_logical_namespace_member $original $members]
        }
        if {[lsearch -exact $members $original] >= 0} { return $original }
        set prefix [_private_command_alias ::tcl::prefix]
        if {[llength [::tmm::_orig_info commands $prefix]]} {
            # The retained host command owns pure prefix selection. It receives
            # only the registry-generated Tcl84 member roster.
            return [uplevel 1 [list $prefix match $members $original]]
        }
        return -code error -errorcode {IRULES SIMULATION CAPABILITY KEYWORD} \
            "authored namespace prefix provider is unavailable"
    }

    proc _install_namespace_restriction {} {
        if {![llength [::tmm::_orig_info commands ::tmm::_orig_namespace]]} {
            ::tmm::_orig_rename ::namespace ::tmm::_orig_namespace
        }

        proc ::namespace {subcommand args} {
            variable ::tmm::runtime_namespace_members
            set subcommand [::tmm::_select_namespace_member $subcommand $runtime_namespace_members]
            switch -exact -- $subcommand {
                delete {
                    set ns [lindex $args 0]
                    if {[string match "::tmm*" $ns] || [string match "tmm*" $ns]} {
                        error "cannot delete namespace \"$ns\""
                    }
                    if {[string match "::state*" $ns] || [string match "state*" $ns]} {
                        error "cannot delete namespace \"$ns\""
                    }
                    return [uplevel 1 [linsert $args 0 ::tmm::_orig_namespace delete]]
                }
                eval {
                    set ns [lindex $args 0]
                    if {[string match "::tmm::_*" $ns] || [string match "tmm::_*" $ns]} {
                        error "cannot eval in namespace \"$ns\""
                    }
                    return [uplevel 1 [linsert $args 0 ::tmm::_orig_namespace eval]]
                }
                which {
                    set result [uplevel 1 [linsert $args 0 ::tmm::_orig_namespace which]]
                    # Hide framework-internal commands
                    if {[string match "::tmm::_orig_*" $result]} {
                        return ""
                    }
                    if {[string match "::_irh_*" $result]} {
                        return ""
                    }
                    return $result
                }
                current {
                    # Must use uplevel so _orig_namespace sees the
                    # caller's scope, not this wrapper proc's scope.
                    return [uplevel 1 [list ::tmm::_orig_namespace current]]
                }
                default {
                    return [uplevel 1 [linsert $args 0 ::tmm::_orig_namespace $subcommand]]
                }
            }
        }
    }

    proc _install_package_view {} {
        proc ::package {args} {
            if {[llength [::tmm::_orig_info commands ::tmm::_logical_package]]} {
                return [uplevel 1 [linsert $args 0 ::tmm::_logical_package [lindex [info level 0] 0]]]
            }
            if {[::tmm::_orig_package provide Tcl] eq "8.4"} {
                return [uplevel 1 [linsert $args 0 ::tmm::_orig_package]]
            }
            return -code error -errorcode {IRULES SIMULATION CAPABILITY PACKAGE} \
                "authored Tcl84 package provider is unavailable"
        }
    }

    # Guard _orig_* commands from direct iRule access
    #
    # iRule code could bypass the sandbox by calling ::tmm::_orig_exec etc.
    # After init, wrap each _orig_* so only framework namespaces (::tmm::,
    # ::itest::, ::orch::, ::state::, ::proto::) may invoke them.

    # Note: ::tmm::_orig_* commands are hidden from iRule code via:
    #   - info commands filter (hides ::tmm::* pattern)
    #   - namespace which guard (returns "" for _orig_* lookups)
    #   - namespace eval restriction (blocks eval in ::tmm::_*)
    # Direct invocation by name is still possible if the caller knows
    # the exact command name, but discovery is blocked.  This matches
    # BIG-IP TMM's security model where the interpreter is not a
    # security boundary — iRules are trusted code.

    # Framework-internal access to disabled commands
    #
    # The framework itself needs to read/write files, use dicts, etc.
    # These are only accessible from ::tmm:: and ::itest:: namespaces.

    proc _internal_puts {args} {
        eval [list ::tmm::_orig_puts] $args
    }

    proc _internal_gets {args} {
        eval [list ::tmm::_orig_gets] $args
    }

    proc _internal_read {args} {
        eval [list ::tmm::_orig_read] $args
    }

    proc _internal_open {args} {
        eval [list ::tmm::_orig_open] $args
    }

    proc _internal_close {args} {
        eval [list ::tmm::_orig_close] $args
    }

    proc _internal_flush {args} {
        eval [list ::tmm::_orig_flush] $args
    }

    proc _internal_eof {args} {
        eval [list ::tmm::_orig_eof] $args
    }

    proc _internal_source {args} {
        eval [list ::tmm::_orig_source] $args
    }

    # Restore original environment

    proc restore {} {
        variable disabled_commands
        variable post84_commands
        variable _initialized
        set _initialized 0

        # Restore disabled and post-8.4 commands.
        # Skip "rename" -- it must be restored LAST because _orig_rename
        # is the tool we use to restore everything else.
        foreach cmd [concat $disabled_commands $post84_commands] {
            set canonical [::tmm::_orig_namespace which -command ::$cmd]
            if {$canonical eq "" || $canonical eq "::rename"} { continue }
            set private [_private_command_alias $canonical]
            if {[llength [::tmm::_orig_info commands [_glob_escape $private]]]} {
                catch { ::tmm::_orig_rename $canonical {} }
                ::tmm::_orig_rename $private $canonical
            }
        }

        # Restore wrapped builtins (info, namespace)
        foreach wrapped {info namespace} {
            if {[llength [::tmm::_orig_info commands ::tmm::_orig_$wrapped]]} {
                catch { ::tmm::_orig_rename ::$wrapped {} }
                ::tmm::_orig_rename ::tmm::_orig_$wrapped ::$wrapped
            }
        }

        foreach wrapped {catch return} {
            if {[llength [::tmm::_orig_info commands ::tmm::_host_$wrapped]]} {
                if {[llength [::tmm::_orig_info procs ::$wrapped]]} {
                    catch { ::tmm::_orig_rename ::$wrapped {} }
                    ::tmm::_orig_rename ::tmm::_host_$wrapped ::$wrapped
                } else {
                    ::tmm::_orig_rename ::tmm::_host_$wrapped {}
                }
            }
        }

        # Restore rename itself last
        if {[llength [::tmm::_orig_info commands ::tmm::_orig_rename]]} {
            # Remove the blocker proc and move original back
            catch { ::tmm::_orig_rename ::rename {} }
            ::tmm::_orig_rename ::tmm::_orig_rename ::rename
        }

        # Uninstall expr_ops overrides
        catch { ::tmm::expr_ops::uninstall }
    }
}
