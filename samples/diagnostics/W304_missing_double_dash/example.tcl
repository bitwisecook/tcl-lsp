# W304: Missing '--' option terminator before substituted input
# tcl-dialect: tcl9.0
# Source: SpiceGenTcl/src/generalClasses.tcl (lsearch, regexp, switch)
#         SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl
#
# The diagnostic is a tristate, driven by what the analyser can prove
# about the first positional argument:
#   OFF      - value structurally cannot be parsed as an option
#   POSSIBLE - dynamic value; could start with '-'         -> info W304
#   ALWAYS   - value is known to start with '-'            -> warning W304
#
# Two structural facts (verified against the Tcl 9.0.4 C sources) shape
# when the check can fire at all:
#
#   * `switch` scans options only while `i < objc-2` (tclCmdMZ.c), so the
#     last two words -- the string and a braced pattern/body list -- are
#     positionally protected. `switch $opt {...}` is safe even when $opt
#     is "-Wall"; only the separate-args form exposes the string word.
#   * `lsearch` has NO `--` terminator at all (its option table in
#     tclCmdIL.c has no "--" entry -- `lsearch -- $l $x` is a runtime
#     error), and its final two words (list, pattern) are likewise
#     positionally protected. W304 therefore never applies to lsearch.
#
# Expected: tclsh 9.0 runs without error.

# ---------------------------------------------------------------
# CASE 1: OFF - the value cannot be misparsed as an option.
# None of these fire W304.
# ---------------------------------------------------------------

# 1a. switch on argparse output with fixed value set - braced-list form,
#     string word positionally protected (i < objc-2).
# Source: generalClasses.tcl:749 - $action is always add/get/set/delete
set action "get"
switch $action {
    add     { set r "adding" }
    get     { set r "getting" }
    set     { set r "setting" }
    delete  { set r "deleting" }
    default { set r "unknown" }
}
puts "switch fixed values: $r"

# 1b. regexp on netlist lines that always start with '.'
# Source: netlistParserClassNgspice.tcl:103
set netlist_line ".include mylib.spice"
if {[regexp {^\.include\s+(.+)} $netlist_line -> filename]} {
    puts "regexp on netlist: $filename"
}

# 1c. lsearch is immune by construction: the needle is one of the final
#     two words, which the option scan never reaches - and lsearch takes
#     no '--' anyway, so there is nothing to insert.
# Source: netlistParserClassNgspice.tcl:320
set tokens {tran 1n 100n uic}
puts "lsearch on tokens: [lsearch -exact $tokens uic]"
proc find_in_list {haystack needle} {
    # $needle may be user input ("-verbose") - still safe: it is the
    # final word, outside the option scan window.
    return [lsearch $haystack $needle]
}
set items {apple -verbose banana -debug cherry}
puts "lsearch positional needle: [find_in_list $items -verbose]"

# 1d. HTTP path always starts with '/'
set path "/api/v1/data"
if {[regexp {^/api/(.+)} $path -> rest]} {
    puts "regexp on path: $rest"
}

# 1e. simulation vector names like v(node), i(src)
# Source: generalClasses.tcl:2197
set vecname "v(anode)"
if {[regexp -nocase {^v\(([^)]+)\)$} $vecname -> node]} {
    puts "regexp on vector name: $node"
}

# 1f. braced-list switch on a value KNOWN to start with '-': still safe,
#     because the string word sits inside the protected final two words.
set name "Wall"
set opt "-$name"
switch $opt {
    -Wall   { set r "all warnings" }
    -O2     { set r "optimise" }
    default { set r "unknown flag" }
}
puts "switch on -\$name (braced form, safe): $r"

# ---------------------------------------------------------------
# CASE 2: POSSIBLE - dynamic value could start with '-'.
# These fire W304 at info severity.
# ---------------------------------------------------------------

# 2a. separate-args switch: the string word is inside the option scan
#     window (objc > 3), so a dynamic value needs '--'.
proc classify {user_opt} {
    switch $user_opt value { return "matched" } default { return "other" }
}
puts "separate-args switch: [classify value]"

# 2b. the same call hardened: '--' terminates the option scan.
proc classify_safe {user_opt} {
    switch -- $user_opt value { return "matched" } default { return "other" }
}
puts "separate-args switch with --: [classify_safe -verbose]"

# 2c. regexp with a dynamic pattern word: could start with '-'.
proc match_any {pattern str} {
    return [regexp $pattern $str]
}
puts "regexp dynamic pattern: [match_any {b.n} "banana"]"

# 2d. hardened: '--' before the pattern makes any content safe.
proc match_any_safe {pattern str} {
    return [regexp -- $pattern $str]
}
puts "regexp dynamic pattern with --: [match_any_safe -- "a--b"]"

# ---------------------------------------------------------------
# CASE 3: ALWAYS - the value is known to start with '-'.
# Fires W304 at warning severity (the misparse is certain).
# ---------------------------------------------------------------

# 3a. a regexp pattern variable that provably holds "-verbose": without
#     '--' the pattern word is parsed as an (unknown) option and the
#     command errors at run time.
set flag "-verbose"
if {[catch {regexp $flag "some -verbose text"} err]} {
    puts "regexp on -\$flag without -- errors: $err"
}

# 3b. hardened with '--': the same value parses as the pattern.
if {[regexp -- $flag "some -verbose text"]} {
    puts "regexp on -\$flag with -- matches"
}

puts "W304 test complete"
