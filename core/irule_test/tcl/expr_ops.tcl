# expr_ops.tcl -- TMM custom expression operators for iRule test framework
#
# BIG-IP TMM extends Tcl's [expr] with additional string operators that
# do not exist in standard Tcl:
#
#   contains       - string containment (case-sensitive)
#   matches_regex  - regex matching
#   starts_with    - string prefix test
#   ends_with      - string suffix test
#   equals         - string equality (case-sensitive)
#   matches_glob   - glob-style matching (8.4 TMM extension)
#   and, or, not   - word-form boolean operators (aliases for &&, ||, !)
#
# These operators appear in expressions like:
#   if { [HTTP::uri] contains "/api" } { ... }
#   if { [HTTP::host] ends_with ".example.com" } { ... }
#
# Implementation strategy:
#   We install custom Tcl math functions via [proc ::tcl::mathfunc::*]
#   for operators that can work that way, and use the [unknown] handler
#   for infix operators that Tcl's expr parser cannot handle directly.
#
#   The real trick: TMM's modified expr parser treats these as infix
#   binary operators.  Standard Tcl's expr cannot be extended that way.
#   So we pre-process expressions before they reach [expr], rewriting
#   them into function-call form.
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::tmm::expr_ops {

    # ── Expression rewriter ───────────────────────────────────────────
    #
    # The approach: override [expr] and [if]/[while]/[for] to pre-process
    # their expression arguments, rewriting TMM infix operators into
    # standard Tcl function calls.
    #
    # "x contains y"        -> [::tmm::expr_ops::_contains x y]
    # "x starts_with y"     -> [::tmm::expr_ops::_starts_with x y]
    # "x ends_with y"       -> [::tmm::expr_ops::_ends_with x y]
    # "x equals y"          -> [::tmm::expr_ops::_equals x y]
    # "x matches_regex y"   -> [::tmm::expr_ops::_matches_regex x y]
    # "x matches_glob y"    -> [::tmm::expr_ops::_matches_glob x y]

    # Operator list from generated registry data (_registry_data.tcl).
    variable _tmm_operators $_gen_operators

    # ── Operator implementations ──────────────────────────────────────

    proc _contains {haystack needle} {
        return [expr {[string first $needle $haystack] >= 0}]
    }

    proc _starts_with {str prefix} {
        set plen [string length $prefix]
        return [expr {[string range $str 0 [expr {$plen - 1}]] eq $prefix}]
    }

    proc _ends_with {str suffix} {
        set slen [string length $suffix]
        if {$slen == 0} { return 1 }
        set start [expr {[string length $str] - $slen}]
        if {$start < 0} { return 0 }
        return [expr {[string range $str $start end] eq $suffix}]
    }

    proc _equals {a b} {
        return [expr {$a eq $b}]
    }

    proc _matches_regex {str pattern} {
        return [regexp -- $pattern $str]
    }

    proc _matches_glob {str pattern} {
        return [string match $pattern $str]
    }

    # ── Expression pre-processor ──────────────────────────────────────
    #
    # Rewrites a TMM expression string to replace infix operators with
    # function calls that standard Tcl [expr] can evaluate.
    #
    # This is deliberately simple and handles the common patterns:
    #   [cmd] operator "literal"
    #   $var operator "literal"
    #   $var operator $var
    #   [cmd] operator [cmd]
    #
    # It works by tokenising the expression and looking for operator
    # keywords between non-operator tokens.

    proc rewrite_expr {expr_str} {
        variable _tmm_operators

        # Quick check: if no TMM operators present, return unchanged
        set found 0
        foreach op $_tmm_operators {
            if {[string first $op $expr_str] >= 0} {
                set found 1
                break
            }
        }
        if {!$found} {
            return $expr_str
        }

        # Tokenise: split on whitespace but respect brackets, braces, quotes
        set tokens [_tokenise $expr_str]

        # Scan for operator tokens and rewrite
        set result [list]
        set i 0
        set len [llength $tokens]
        while {$i < $len} {
            set tok [lindex $tokens $i]

            # Check if next token is a TMM operator
            if {$i + 2 < $len} {
                set maybe_op [lindex $tokens [expr {$i + 1}]]
                if {[lsearch -exact $_tmm_operators $maybe_op] >= 0} {
                    set rhs [lindex $tokens [expr {$i + 2}]]
                    # Rewrite: lhs op rhs -> [::tmm::expr_ops::_op lhs rhs]
                    lappend result "\[::tmm::expr_ops::_${maybe_op} ${tok} ${rhs}\]"
                    set i [expr {$i + 3}]
                    continue
                }
            }

            lappend result $tok
            incr i
        }

        return [join $result " "]
    }

    # Simple tokeniser that respects brackets and quotes
    proc _tokenise {str} {
        set tokens [list]
        set len [string length $str]
        set pos 0

        while {$pos < $len} {
            # Skip whitespace
            while {$pos < $len && [string is space [string index $str $pos]]} {
                incr pos
            }
            if {$pos >= $len} break

            set ch [string index $str $pos]
            set start $pos

            if {$ch eq "\["} {
                # Command substitution -- find matching ]
                set depth 1
                incr pos
                while {$pos < $len && $depth > 0} {
                    set c [string index $str $pos]
                    if {$c eq "\["} { incr depth }
                    if {$c eq "\]"} { incr depth -1 }
                    if {$c eq "\\"} { incr pos }
                    incr pos
                }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            } elseif {$ch eq "\""} {
                # Quoted string
                incr pos
                while {$pos < $len && [string index $str $pos] ne "\""} {
                    if {[string index $str $pos] eq "\\"} { incr pos }
                    incr pos
                }
                if {$pos < $len} { incr pos }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            } elseif {$ch eq "\{"} {
                # Braced string
                set depth 1
                incr pos
                while {$pos < $len && $depth > 0} {
                    set c [string index $str $pos]
                    if {$c eq "\{"} { incr depth }
                    if {$c eq "\}"} { incr depth -1 }
                    if {$c eq "\\"} { incr pos }
                    incr pos
                }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            } elseif {$ch eq "\$"} {
                # Variable reference
                incr pos
                while {$pos < $len} {
                    set c [string index $str $pos]
                    if {[string is alnum $c] || $c eq "_" || $c eq ":"} {
                        incr pos
                    } else {
                        break
                    }
                }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            } elseif {$ch eq "&" || $ch eq "|" || $ch eq "!" ||
                      $ch eq "=" || $ch eq "<" || $ch eq ">" ||
                      $ch eq "+" || $ch eq "-" || $ch eq "*" ||
                      $ch eq "/" || $ch eq "%" || $ch eq "~" ||
                      $ch eq "^" || $ch eq "?"  || $ch eq ":"} {
                # Operators -- grab multi-char operators
                incr pos
                while {$pos < $len} {
                    set c [string index $str $pos]
                    if {$c eq "&" || $c eq "|" || $c eq "=" ||
                        $c eq "<" || $c eq ">"} {
                        incr pos
                    } else {
                        break
                    }
                }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            } elseif {$ch eq "("  || $ch eq ")"} {
                incr pos
                lappend tokens $ch
            } else {
                # Word token (identifier, number, operator keyword)
                while {$pos < $len} {
                    set c [string index $str $pos]
                    if {[string is space $c] || $c eq "(" || $c eq ")" ||
                        $c eq "\[" || $c eq "\]" || $c eq "\{" || $c eq "\}"} {
                        break
                    }
                    incr pos
                }
                lappend tokens [string range $str $start [expr {$pos - 1}]]
            }
        }

        return $tokens
    }

    # ── Install expr/if/while/for overrides ───────────────────────────
    #
    # We wrap the builtins to pre-process expressions before evaluation.

    proc install {} {
        # Save originals
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_expr]]} {
            rename ::expr ::tmm::expr_ops::_orig_expr
        }
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_if]]} {
            rename ::if ::tmm::expr_ops::_orig_if
        }
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_while]]} {
            rename ::while ::tmm::expr_ops::_orig_while
        }
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_for]]} {
            rename ::for ::tmm::expr_ops::_orig_for
        }
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_switch]]} {
            rename ::switch ::tmm::expr_ops::_orig_switch
        }

        # expr -- rewrite and evaluate
        proc ::expr {args} {
            if {[llength $args] == 1} {
                set rewritten [::tmm::expr_ops::rewrite_expr [lindex $args 0]]
                return [::tmm::expr_ops::_orig_expr $rewritten]
            }
            # Multi-arg expr: concatenate, rewrite, evaluate
            set joined [join $args " "]
            set rewritten [::tmm::expr_ops::rewrite_expr $joined]
            return [::tmm::expr_ops::_orig_expr $rewritten]
        }

        # if -- rewrite condition expression
        proc ::if {args} {
            # First arg is the condition
            set condition [::tmm::expr_ops::rewrite_expr [lindex $args 0]]
            set rest [lrange $args 1 end]
            return [eval [list ::tmm::expr_ops::_orig_if $condition] $rest]
        }

        # while -- rewrite condition expression
        proc ::while {condition body} {
            set rewritten [::tmm::expr_ops::rewrite_expr $condition]
            return [::tmm::expr_ops::_orig_while $rewritten $body]
        }

        # for -- rewrite test expression
        proc ::for {init test next body} {
            set rewritten [::tmm::expr_ops::rewrite_expr $test]
            return [::tmm::expr_ops::_orig_for $init $rewritten $next $body]
        }

        # switch passes through unchanged -- it doesn't use expressions
    }

    proc uninstall {} {
        foreach cmd {expr if while for switch} {
            if {[llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_$cmd]]} {
                catch { rename ::$cmd {} }
                rename ::tmm::expr_ops::_orig_$cmd ::$cmd
            }
        }
    }
}
