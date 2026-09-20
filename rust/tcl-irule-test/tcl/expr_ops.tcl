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
#   and / or / not - word forms of && / || / !
#
# These operators appear in expressions like:
#   if { [HTTP::uri] contains "/api" } { ... }
#   if { [HTTP::host] ends_with ".example.com" } { ... }
#   if { [HTTP::method] eq "GET" and not ([HTTP::uri] contains "/x") } { ... }
#
# Implementation: TMM's modified expr parser treats these as infix binary
# operators; standard Tcl's expr cannot be extended that way.  So [expr]
# is wrapped to pre-process its expression argument before evaluation,
# rewriting each recognised infix operator into a function call that
# plain Tcl can evaluate.
#
# The word-form booleans are the exception: they are rewritten to the
# plain-Tcl symbols they are equivalent to, NOT to helper procs.  A proc
# call evaluates both of its arguments before the proc body runs, which
# would turn a short-circuiting operator into an eager one; substituting
# && / || / ! keeps [expr]'s own short-circuiting and binding powers,
# which is exactly what TMM has (see _gen_boolean_operators in
# _registry_data.tcl for the authority).
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::tmm::expr_ops {

    # Expression rewriter
    #
    # The approach: override [expr] to pre-process its expression argument,
    # rewriting TMM infix operators into standard Tcl function calls.  The
    # control-flow commands are not overridden -- `install` says why -- so
    # their expression arguments are rewritten in the iRule source instead,
    # by `rewrite_irule_source` below.
    #
    # "x contains y"        -> [::tmm::expr_ops::_contains x y]
    # "x starts_with y"     -> [::tmm::expr_ops::_starts_with x y]
    # "x ends_with y"       -> [::tmm::expr_ops::_ends_with x y]
    # "x equals y"          -> [::tmm::expr_ops::_equals x y]
    # "x matches_regex y"   -> [::tmm::expr_ops::_matches_regex x y]
    # "x matches_glob y"    -> [::tmm::expr_ops::_matches_glob x y]
    # "x and y"             -> x && y
    # "x or y"              -> x || y
    # "not x"               -> ! x
    #
    # `not` directly in front of the left operand of a custom comparison --
    # "not x starts_with y" -- is refused rather than grouped; see
    # `_refuse_unmeasured_prefix_grouping`.

    # Operator lists from the registry data (_registry_data.tcl).
    variable _tmm_operators $_gen_operators

    # Word-form boolean operators, as {word symbol ...} pairs.
    variable _tmm_boolean_operators $_gen_boolean_operators

    # Word-boundary matcher for "does this text use any TMM word operator
    # at all".  A plain substring test would fire on the `or` inside `for`
    # (and so route every iRule through the rewriter); \m..\M pins the
    # token boundaries.
    variable _tmm_operator_re "\\m([join $_gen_all_operators |])\\M"

    # The plain-Tcl operator a word-form boolean is equivalent to, or ""
    # when the token is not one.
    proc _boolean_symbol {word} {
        variable _tmm_boolean_operators
        foreach {name symbol} $_tmm_boolean_operators {
            if {$word eq $name} { return $symbol }
        }
        return ""
    }

    # The plain-Tcl symbols that bind as a prefix rather than an infix.  A
    # word-form boolean is classified by the symbol it is equivalent to, so
    # this needs no second list of operator names.
    variable _prefix_symbols {! ~}

    proc _is_prefix_symbol {symbol} {
        variable _prefix_symbols
        if {[lsearch -exact $_prefix_symbols $symbol] >= 0} { return 1 }
        return 0
    }

    # Operator implementations

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

    # Expression pre-processor
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

    # Refuse a word-form prefix operator applied directly to the left operand
    # of a custom comparison -- `not $x starts_with "/a"`.
    #
    # The two readings are `not ($x starts_with "/a")` and
    # `(not $x) starts_with "/a"`, and nothing measures which one TMM takes.
    # `not` is deliberately absent from F5_TCL_PRECEDENCE_ROWS in
    # rust/tcl-dialect/src/model/expr_grammar.rs (`lookup("not") == None`),
    # and the UNARY_BP that rust/tcl-syntax/src/expr/parser.rs gives it is the
    # parser's shared default for every prefix operator, not an F5
    # measurement: docs/design/f5/bigip-irule-parser-measurements.md pins only
    # `if {not 0}` (the §4a expr_and_or_not probe), which exercises no binding
    # power at all.
    #
    # So this harness refuses, the way the bare `matches` operator is left
    # unimplemented rather than given a guessed meaning.  Failing loudly is
    # correct; silently answering one of the two groupings is not.
    #
    # `not X and Y` and a bare `not X` are unaffected: `and` and `or` sit
    # below every unary operator on the ladder (6/7 and 4/5), which is the
    # grouping plain Tcl's own `expr` gives `!X && Y`.
    proc _refuse_unmeasured_prefix_grouping {expr_str tokens i word} {
        variable _tmm_operators

        set op_index $i
        incr op_index 2
        if {$op_index >= [llength $tokens]} { return }
        set op [lindex $tokens $op_index]
        if {[lsearch -exact $_tmm_operators $op] < 0} { return }

        set operand_index $i
        incr operand_index
        set operand [lindex $tokens $operand_index]
        error "unmeasured TMM operator grouping in expression {$expr_str}: the\
word-form unary `$word` is applied directly to `$operand`, the left operand of\
the custom comparison `$op`, and F5's grouping for that pair is not measured --\
`$word ($operand $op ...)` and `($word $operand) $op ...` are both consistent\
with every transcript in docs/design/f5/bigip-irule-parser-measurements.md, so\
this harness refuses to guess one. Parenthesise the intended grouping."
    }

    proc rewrite_expr {expr_str} {
        variable _tmm_operators
        variable _tmm_operator_re

        # Quick check: if no TMM operator token is present, return unchanged
        if {![regexp -- $_tmm_operator_re $expr_str]} {
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

            # Word-form boolean: substitute the equivalent plain-Tcl
            # operator, so [expr] supplies the short-circuiting and the
            # binding power rather than this rewriter.
            set sym [_boolean_symbol $tok]
            if {$sym ne ""} {
                if {[_is_prefix_symbol $sym]} {
                    _refuse_unmeasured_prefix_grouping $expr_str $tokens $i $tok
                }
                lappend result $sym
                incr i
                continue
            }

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

    # Install the ::expr override
    #
    # We wrap that one builtin to pre-process its expression before
    # evaluation.  The control-flow commands are handled in the source
    # instead -- see the body for why they must not be wrapped.

    proc install {} {
        # Only override ::expr.  We deliberately do NOT replace
        # if/while/for -- wrapping control-flow commands with procs
        # breaks break/continue/return propagation.  Instead the
        # iRule source is preprocessed at load time by
        # rewrite_irule_source (called from itest::load_irule).
        if {![llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_expr]]} {
            ::tmm::_orig_rename ::expr ::tmm::expr_ops::_orig_expr
        }

        # Re-entrancy guard so the rewriter and operator
        # implementations can use expr without infinite recursion.
        variable _in_rewrite 0

        proc ::expr {args} {
            variable ::tmm::expr_ops::_in_rewrite
            if {$_in_rewrite} {
                return [uplevel 1 ::tmm::expr_ops::_orig_expr $args]
            }
            set _in_rewrite 1
            set code [catch {
                if {[llength $args] == 1} {
                    set rewritten [::tmm::expr_ops::rewrite_expr [lindex $args 0]]
                    set _result [uplevel 1 [list ::tmm::expr_ops::_orig_expr $rewritten]]
                } else {
                    set joined [join $args " "]
                    set rewritten [::tmm::expr_ops::rewrite_expr $joined]
                    set _result [uplevel 1 [list ::tmm::expr_ops::_orig_expr $rewritten]]
                }
            } result opts]
            set _in_rewrite 0
            if {$code} {
                return -code $code $result
            }
            return $_result
        }
    }

    proc uninstall {} {
        foreach cmd {expr} {
            if {[llength [::tmm::_orig_info commands ::tmm::expr_ops::_orig_$cmd]]} {
                catch { ::tmm::_orig_rename ::$cmd {} }
                ::tmm::_orig_rename ::tmm::expr_ops::_orig_$cmd ::$cmd
            }
        }
    }

    # Source-level rewriting
    #
    # Preprocess iRule source text to rewrite TMM custom operators in the
    # expression arguments of if/elseif/while/for/expr.  This avoids
    # replacing those commands with procs, which breaks break/continue/return
    # propagation (see `install`).
    #
    # The scan is command-aware: the source is split into commands and words
    # the way Tcl itself splits it, and only a word that a *recognised command
    # in command position* uses as an expression is rewritten.  A regexp over
    # the raw text cannot tell a command word from a quoted string, a braced
    # data literal or a comment, so it rewrote Tcl data -- `set message
    # "if {1 and 0}"` became `set message "if {1 && 0}"`.  That shape has no
    # workaround for the author, and the word-form booleans made it common
    # (`and` / `or` / `not` are ordinary English words, so ordinary iRule text
    # now reaches the rewriter).
    #
    # `for` is not special-cased anywhere else: there is no ::for override --
    # `install` explains why there must not be one -- so its test expression
    # is rewritten here, at the second argument.
    #
    # Limits.  A braced word is entered only where one of the commands in the
    # table below takes a script or an expression there, so script text held
    # as data and evaluated later (`eval`, `uplevel`, a list of snippets) is
    # left with its word operators, and the runtime ::expr override is then
    # the only thing that sees them.  A command substitution inside a bare or
    # quoted word *is* entered -- `set v [if {$a and $b} {…}]` -- because that
    # is a command position.

    # The argument roles of the commands this rewriter enters, as
    # {name expr-positions script-positions} triples.  A position is a word
    # index (0 is the command name); `end` is the last word.  `if`, `elseif`,
    # `else` and `switch` have their own walkers below because their argument
    # shape is a chain rather than fixed positions.
    #
    # Only a braced word is ever entered: an unbraced expression argument is
    # substituted before the command sees it, and an unbraced script argument
    # is not a literal this rewriter could splice into.
    variable _rewrite_arg_roles {
        while   {1} {2}
        for     {2} {1 3 4}
        expr    {1} {}
        when    {}  {end}
        foreach {}  {end}
        catch   {}  {1}
        after   {}  {end}
    }

    proc rewrite_irule_source {source} {
        variable _tmm_operator_re

        # Quick check: if no TMM operator token appears anywhere, return
        # unchanged
        if {![regexp -- $_tmm_operator_re $source]} {
            return $source
        }

        # Collect the rewrites as {start end replacement} edits over the
        # original text, left to right and non-overlapping, then splice them
        # in.  Editing rather than rebuilding keeps every byte the rewriter
        # does not own -- whitespace, comments, data -- exactly as written.
        set result ""
        set pos 0
        foreach edit [_script_edits $source 0 [string length $source]] {
            set start [lindex $edit 0]
            set stop [lindex $edit 1]
            set before $start
            incr before -1
            append result [string range $source $pos $before]
            append result [lindex $edit 2]
            set pos $stop
        }
        append result [string range $source $pos end]

        return $result
    }

    # Every edit for the script occupying [start,end) of $str.
    proc _script_edits {str start end} {
        set edits [list]
        foreach cmd [_script_commands $str $start $end] {
            foreach edit [_command_edits $str $cmd] {
                lappend edits $edit
            }
        }
        return $edits
    }

    # Every edit for one command (a list of {start end kind} words).
    proc _command_edits {str cmd} {
        set roles [_argument_roles $str $cmd]
        set nwords [llength $cmd]
        set edits [list]
        # Word order is text order, and the splice needs its edits sorted.
        for {set idx 0} {$idx < $nwords} {incr idx} {
            set word [lindex $cmd $idx]
            set role [lindex $roles $idx]
            if {$role eq "expr"} {
                foreach edit [_word_expr_edits $str $word] { lappend edits $edit }
            } elseif {$role eq "script"} {
                foreach edit [_word_script_edits $str $word] { lappend edits $edit }
            } elseif {$role eq "switchbody"} {
                foreach edit [_switch_body_edits $str $word] { lappend edits $edit }
            } else {
                # Everything else is data to this rewriter -- but a command
                # substitution inside it is a genuine command position.
                foreach edit [_word_substitution_edits $str $word] { lappend edits $edit }
            }
        }
        return $edits
    }

    # The role this rewriter gives each word of $cmd: `expr` for an
    # expression argument, `script` for a script body, `switchbody` for a
    # `switch` pattern/body list, and `word` for everything it does not own.
    proc _argument_roles {str cmd} {
        variable _rewrite_arg_roles

        set nwords [llength $cmd]
        set roles [list]
        for {set idx 0} {$idx < $nwords} {incr idx} { lappend roles word }

        set first [lindex $cmd 0]
        if {[lindex $first 2] ne "bare"} {
            # A braced or quoted first word is not a command name, and its
            # content is not necessarily a script.
            return $roles
        }
        set name [_word_text $str $first]
        if {[string match "::*" $name]} {
            set name [string range $name 2 end]
        }

        if {$name eq "if" || $name eq "elseif"} {
            # A bare `elseif` / `else` command is the iRules lookahead that
            # picks the clause up across a newline (N5 in
            # docs/design/f5/bigip-irule-parser-measurements.md); the same
            # walker handles it and the single-command form.
            return [_if_chain_roles $str $cmd $roles expr]
        }
        if {$name eq "else"} {
            return [_if_chain_roles $str $cmd $roles body]
        }
        if {$name eq "switch"} {
            return [_switch_roles $str $cmd $roles]
        }

        foreach {cname expr_positions script_positions} $_rewrite_arg_roles {
            if {$cname ne $name} { continue }
            foreach idx [_resolve_positions $expr_positions $nwords] {
                set roles [lreplace $roles $idx $idx expr]
            }
            foreach idx [_resolve_positions $script_positions $nwords] {
                set roles [lreplace $roles $idx $idx script]
            }
            return $roles
        }
        return $roles
    }

    # Word indices a role list names, with `end` resolved and out-of-range
    # positions dropped.
    proc _resolve_positions {positions nwords} {
        set out [list]
        set last $nwords
        incr last -1
        foreach position $positions {
            if {$position eq "end"} {
                set position $last
            }
            if {$position >= 1 && $position <= $last} {
                lappend out $position
            }
        }
        return $out
    }

    # The `if` / `elseif` / `else` clause chain: condition, body, then any
    # number of `elseif` condition body clauses and an optional `else` body.
    proc _if_chain_roles {str cmd roles expect} {
        set nwords [llength $cmd]
        set idx 1
        while {$idx < $nwords} {
            set word [lindex $cmd $idx]
            set text [_word_text $str $word]
            if {$expect eq "expr"} {
                set roles [lreplace $roles $idx $idx expr]
                set expect body
                incr idx
                continue
            }
            if {$expect eq "body"} {
                if {[lindex $word 2] eq "bare" && $text eq "then"} {
                    incr idx
                    continue
                }
                set roles [lreplace $roles $idx $idx script]
                set expect clause
                incr idx
                continue
            }
            if {[lindex $word 2] ne "bare"} { break }
            if {$text eq "elseif"} {
                set expect expr
                incr idx
                continue
            }
            if {$text eq "else"} {
                set expect body
                incr idx
                continue
            }
            break
        }
        return $roles
    }

    # `switch ?options? string {pattern body ...}` and the flattened
    # `switch ?options? string pattern body ...` form.  Only the bodies are
    # scripts; the patterns are data.
    proc _switch_roles {str cmd roles} {
        set nwords [llength $cmd]
        set idx 1
        while {$idx < $nwords} {
            set text [_word_text $str [lindex $cmd $idx]]
            if {$text eq "--"} {
                incr idx
                break
            }
            if {![string match "-*" $text]} { break }
            incr idx
        }
        # $idx is the string argument; the patterns and bodies follow it.
        incr idx
        if {$idx >= $nwords} { return $roles }

        set last $nwords
        incr last -1
        if {$idx == $last} {
            return [lreplace $roles $idx $idx switchbody]
        }
        set body $idx
        incr body
        while {$body < $nwords} {
            set roles [lreplace $roles $body $body script]
            incr body 2
        }
        return $roles
    }

    # The braced `{pattern body pattern body ...}` argument of a `switch`:
    # every second word is a script.
    proc _switch_body_edits {str word} {
        if {[lindex $word 2] ne "brace"} { return [list] }
        set start [lindex $word 0]
        incr start
        set end [lindex $word 1]
        incr end -1

        set edits [list]
        set words [_region_words $str $start $end]
        set body 1
        while {$body < [llength $words]} {
            foreach edit [_word_script_edits $str [lindex $words $body]] {
                lappend edits $edit
            }
            incr body 2
        }
        return $edits
    }

    # A word this rewriter does not own, but whose command substitutions are
    # command positions all the same: `set v [if {$a and $b} {…}]`.  Only a
    # bare or quoted word substitutes -- a braced one is literal.
    proc _word_substitution_edits {str word} {
        set kind [lindex $word 2]
        if {$kind ne "bare" && $kind ne "quote"} { return [list] }

        set pos [lindex $word 0]
        set end [lindex $word 1]
        set edits [list]
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {$ch eq "\\"} {
                incr pos 2
                continue
            }
            if {$ch eq "\["} {
                incr pos
                set close [_matching_bracket $str $pos $end]
                foreach edit [_script_edits $str $pos $close] {
                    lappend edits $edit
                }
                set pos $close
                incr pos
                continue
            }
            incr pos
        }
        return $edits
    }

    # The index of the `]` closing a command substitution that starts at
    # $pos, or $end when it is unterminated.
    proc _matching_bracket {str pos end} {
        set depth 1
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {$ch eq "\\"} {
                incr pos 2
                continue
            }
            if {$ch eq "\{"} {
                set nested [_word_extent $str $pos $end]
                set pos [lindex $nested 1]
                continue
            }
            if {$ch eq "\["} {
                incr depth
                incr pos
                continue
            }
            if {$ch eq "\]"} {
                incr depth -1
                if {$depth == 0} { return $pos }
                incr pos
                continue
            }
            incr pos
        }
        return $end
    }

    # A braced word used as a script: recurse into its contents.
    proc _word_script_edits {str word} {
        if {[lindex $word 2] ne "brace"} { return [list] }
        set start [lindex $word 0]
        incr start
        set end [lindex $word 1]
        incr end -1
        return [_script_edits $str $start $end]
    }

    # A braced word used as an expression: rewrite its contents in one edit.
    proc _word_expr_edits {str word} {
        if {[lindex $word 2] ne "brace"} { return [list] }
        set start [lindex $word 0]
        incr start
        set end [lindex $word 1]
        incr end -1
        set stop $end
        incr stop -1
        set inner [string range $str $start $stop]
        set rewritten [rewrite_expr $inner]
        if {$rewritten eq $inner} { return [list] }
        return [list [list $start $end $rewritten]]
    }

    # Split [start,end) of $str into commands, each a list of
    # {start end kind} words.  Comments, separators and whitespace are
    # skipped: they are never rewritten, so they need no representation.
    proc _script_commands {str start end} {
        set cmds [list]
        set pos $start
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {$ch eq "\\"} {
                set nxt $pos
                incr nxt
                if {[string index $str $nxt] eq "\n"} {
                    set pos $nxt
                    incr pos
                    continue
                }
            } elseif {[string is space -strict $ch] || $ch eq ";"} {
                incr pos
                continue
            } elseif {$ch eq "#"} {
                set pos [_skip_comment $str $pos $end]
                continue
            }

            set words [list]
            while {$pos < $end} {
                set ch [string index $str $pos]
                if {$ch eq "\\"} {
                    set nxt $pos
                    incr nxt
                    if {[string index $str $nxt] eq "\n"} {
                        set pos $nxt
                        incr pos
                        continue
                    }
                }
                if {$ch eq " " || $ch eq "\t" || $ch eq "\r"} {
                    incr pos
                    continue
                }
                if {$ch eq ";"} {
                    incr pos
                    break
                }
                if {$ch eq "\n"} {
                    # iRules' brace-line continuation: a newline does not end
                    # the command when the next line's first non-blank
                    # character is an open brace, at any nesting depth (N1 and
                    # N3 in docs/design/f5/bigip-irule-parser-measurements.md).
                    set peek $pos
                    incr peek
                    set peek [_skip_blanks $str $peek $end]
                    if {$peek < $end && [string index $str $peek] eq "\{"} {
                        set pos $peek
                        continue
                    }
                    incr pos
                    break
                }
                set extent [_word_extent $str $pos $end]
                lappend words $extent
                set pos [lindex $extent 1]
            }
            if {[llength $words]} { lappend cmds $words }
        }
        return $cmds
    }

    # Whitespace-separated words of [start,end), with no command structure:
    # a `switch` pattern/body list.
    proc _region_words {str start end} {
        set words [list]
        set pos $start
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {[string is space -strict $ch]} {
                incr pos
                continue
            }
            set extent [_word_extent $str $pos $end]
            lappend words $extent
            set pos [lindex $extent 1]
        }
        return $words
    }

    # One word starting at $pos, as {start end kind} with $end one past the
    # word and $kind one of brace / quote / bare.  An unterminated brace or
    # quote is reported bare, so nothing inside it is ever entered.
    proc _word_extent {str pos end} {
        set start $pos
        set ch [string index $str $pos]

        if {$ch eq "\{"} {
            set depth 1
            incr pos
            while {$pos < $end && $depth > 0} {
                set c [string index $str $pos]
                if {$c eq "\\"} {
                    incr pos 2
                    continue
                }
                if {$c eq "\{"} { incr depth }
                if {$c eq "\}"} { incr depth -1 }
                incr pos
            }
            if {$depth == 0} { return [list $start $pos brace] }
            return [list $start $pos bare]
        }

        if {$ch eq "\""} {
            set closed 0
            incr pos
            while {$pos < $end} {
                set c [string index $str $pos]
                if {$c eq "\\"} {
                    incr pos 2
                    continue
                }
                incr pos
                if {$c eq "\""} {
                    set closed 1
                    break
                }
            }
            if {$closed} { return [list $start $pos quote] }
            return [list $start $pos bare]
        }

        # A bare word runs to the next separator that is not nested inside a
        # command substitution or a brace pair.
        set depth 0
        while {$pos < $end} {
            set c [string index $str $pos]
            if {$c eq "\\"} {
                incr pos 2
                continue
            }
            if {$c eq "\["} {
                incr depth
                incr pos
                continue
            }
            if {$c eq "\]"} {
                if {$depth > 0} { incr depth -1 }
                incr pos
                continue
            }
            if {$c eq "\{"} {
                set nested [_word_extent $str $pos $end]
                set pos [lindex $nested 1]
                continue
            }
            if {$depth == 0} {
                if {$c eq " " || $c eq "\t" || $c eq "\r" ||
                    $c eq "\n" || $c eq ";"} {
                    break
                }
            }
            incr pos
        }
        if {$pos <= $start} {
            set pos $start
            incr pos
        }
        return [list $start $pos bare]
    }

    # Past a comment: to the end of its line, honouring backslash
    # continuation.
    proc _skip_comment {str pos end} {
        incr pos
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {$ch eq "\\"} {
                incr pos 2
                continue
            }
            if {$ch eq "\n"} {
                incr pos
                break
            }
            incr pos
        }
        return $pos
    }

    # Past spaces and tabs only: a newline still terminates a command, which
    # is what makes a blank line end one (N4).
    proc _skip_blanks {str pos end} {
        while {$pos < $end} {
            set ch [string index $str $pos]
            if {$ch ne " " && $ch ne "\t" && $ch ne "\r"} { break }
            incr pos
        }
        return $pos
    }

    # The source text of a {start end kind} word.
    proc _word_text {str word} {
        set stop [lindex $word 1]
        incr stop -1
        return [string range $str [lindex $word 0] $stop]
    }
}
