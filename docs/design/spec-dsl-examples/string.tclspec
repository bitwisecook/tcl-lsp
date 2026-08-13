# Partial port of rust/tcl-registry/src/commands/tcl/string_.rs — the
# `length`, `is`, `map`, and `range` subcommands only (the shipped ensemble
# declares 24).
#
# Exercises: a big ensemble, subcommand-level facts (purity, byte-array
# effect, per-subcommand options / arg types / return types), a pack-level
# named value table shared by several declarations, closed value sets with
# prefix acceptance, per-value Tcl-version floors, and constant folders
# written as Tcl hook bodies.

speclib tcl 1.0 {

# The `string is` character classes.  A named table because the shipped spec
# has the same shape: one `static IS_CLASSES` read both by `arg_values` and by
# the folder that decides whether a class is even available in the target
# release.  Referenced below with `-values-from is-classes`.
values is-classes {
    value alnum   -detail {Any Unicode alphabet or digit character.}
    value alpha   -detail {Any Unicode alphabet character.}
    value ascii   -detail {Any character with a value less than U+0080 (7-bit ASCII).}
    value boolean -detail {Any valid boolean value (true/false/yes/no/on/off/0/1).}
    value control -detail {Any Unicode control character.}
    value dict    -min-tcl tcl9.0 -detail {Any proper dict structure, with optional surrounding whitespace.}
    value digit   -detail {Any Unicode digit character.}
    value double  -detail {Any valid floating-point number.}
    value entier  -min-tcl tcl8.6 -detail {An arbitrary-size integer. On Tcl 8.6, where integer itself was still 32-bit-bounded, entier was the only arbitrary-size class; from Tcl 9.0, where integer itself became arbitrary-size, entier is a plain synonym for it.}
    value false   -detail {Any valid boolean false value.}
    value graph   -detail {Any Unicode printing character, except space.}
    value integer -detail {Any valid integer, with optional surrounding whitespace. Bounded to 32 bits through Tcl 8.6; arbitrary size from Tcl 9.0 onward.}
    # Absent from the 8.4 manpage's class table; `string is list …` raises
    # "bad class" under an 8.4 dialect rather than returning 0/1.
    value list    -min-tcl tcl8.5 -detail {Any proper list structure, with optional surrounding whitespace.}
    value lower   -detail {Any Unicode lower case alphabet character.}
    value print   -detail {Any Unicode printing character, including space.}
    value punct   -detail {Any Unicode punctuation character.}
    value space   -detail {Any Unicode whitespace character.}
    value true    -detail {Any valid boolean true value.}
    value upper   -detail {Any upper case alphabet character.}
    value wideinteger -min-tcl tcl8.5 -detail {Any valid wide integer, with optional surrounding whitespace. From Tcl 9.0 this is bounded to the signed 64-bit range; Tcl 8.5/8.6 accept a wider positive magnitude (up to 2^64-1).}
    value wordchar -detail {Any Unicode word character (alphanumeric + connector punctuation).}
    value xdigit  -detail {Any hexadecimal digit character (0-9, A-F, a-f).}
}

command string {
    # A pure value-transform ensemble with no filesystem/process/network
    # access, so every dialect that hosts a real Tcl core carries it.
    # Individual subcommands and `is` classes still narrow per Tcl version
    # through their own gates.
    dialects {all-tcl f5-irules}
    traits {FRAMELESS_RUNTIME NOT_PROC_FACTORY BYTE_COMPILED CSE_CANDIDATE}
    arity 1..
    inline_codegen_hook -native String

    form Default {string option arg ?arg ...?}

    hover {
        summary {Perform one of several string operations, selected by the first argument.}
        synopsis {string option arg ?arg ...?}
        description {Dispatches to a subcommand (length, match, is, compare, map, range, index, trim, ... — see completion for the full list); an unrecognised option is a runtime error, not a compile-time one, and an unambiguous prefix of a subcommand name is accepted. Nearly every subcommand is a pure value transform with no side effects; the one exception is `is`'s `-failindex varname`, which writes the class-test failure position into varname. Index arguments (string index/range/replace, the first/last of tolower/toupper/totitle, the startIndex/lastIndex of first/last) accept a plain non-negative integer, end (the last character), or end-N; Tcl 8.5+ also accepts end+N and the M+N/M-N arithmetic forms. The subcommand set itself has grown over releases: reverse and the is list/wideinteger classes need Tcl 8.5+; cat and the is entier class need Tcl 8.6+; insert and the is dict class need Tcl 9.0+; bytelength was removed at the Tcl 9.0 boundary after being documented obsolete since 8.4 — prefer string length (character count) or the encoding command (byte count) instead. trim/trimleft/trimright strip whitespace by default when chars is omitted; from Tcl 8.6 that default set also strips NUL (\0), on top of the (also Unicode-aware and separately growing) whitespace string is space matches.}
        source {Tcl string(n)}
        example {set s "  Hello, World!  "
string trim $s                   ;# -> "Hello, World!"
string tolower [string trim $s]  ;# -> "hello, world!"
string map {World Tcl} $s        ;# -> "  Hello, Tcl!  "
string range $s 2 6              ;# -> "Hello"
string is integer -strict 42     ;# -> 1

if {[string first "World" $s] >= 0} {
    puts "found"
}}
        returns {Depends on the subcommand: most string-valued subcommands (cat, index, insert, map, range, replace, reverse, tolower/toupper/totitle, trim/trimleft/trimright) return a (possibly empty) string; is, equal, and match return a boolean 0/1; compare returns -1/0/1; length, wordend, wordstart, and bytelength (through Tcl 8.6) return a non-negative integer; first and last return a non-negative index, or -1 when there is no match.}
    }

    subcommand length {
        semantic_operation {Intrinsic StringLength}
        arity 1
        detail   {Return number of characters.}
        synopsis {string length string}
        pure
        return_type Int

        # Tcl_GetCharLength installs the string intrep, replacing a
        # List/Dict/Int rep.  A pure byte array short-circuits to its byte
        # count and keeps its rep, so ByteArray is transparent even though
        # the result is a count, not bytes (byte_array_effect stays None).
        arg 0 -type String -shimmers -transparent {ByteArray}

        const_fold {words ctx} {
            if {[llength $words] != 1} return
            set s [lindex $words 0]
            # ASCII only: there the byte length equals the character count,
            # matching Tcl's character-counting `string length`.  Non-ASCII
            # bails — the count diverges between Tcl 8.x (UTF-16 units) and
            # Tcl 9 (Unicode scalars) for astral characters.
            if {[string is ascii $s]} { fold [string length $s] }
        }
    }

    subcommand is {
        semantic_operation {Intrinsic StringIs}
        arity 2..
        detail   {Test if string is a member of a character class.}
        synopsis {string is class ?-strict? ?-failindex varname? string}
        return_type Boolean

        # Deliberately NOT `pure`: -failindex optionally writes its varname
        # argument and the static spec cannot see whether a given call uses
        # it.  The purity classifier trusts `pure` unconditionally and never
        # looks at side_effects when it is set, so marking this pure would let
        # DCE/CSE silently drop or reorder a real variable write.
        side_effect Variable -writes

        option -strict -detail {Treat the empty string as not matching the class (by default the empty string matches every class).}
        option -failindex -takes varname -role VarWrite -detail {Variable to receive the index where the class test failed. Left unset when string matches the class; on failure its exact contents are class-specific (e.g. -1 for a numeric overflow, the parse-failure index for list/dict, always 0 for boolean/true/false).}

        # Index 0 after `is` is the character class: an exhaustive set (a
        # non-member is a runtime "bad class" error → W127), and C Tcl accepts
        # a unique prefix (`boo` → `boolean`).
        arg 0 -values-from is-classes -closed
        arg_values_accept_prefix

        # NOT ported to a Tcl body: the shipped folder is a version-aware
        # classifier (per-class availability floors, the 8.x/9.x integer
        # magnitude caps, Tcl 9 radix prefixes and digit separators, the
        # ambiguous-form bail-outs).  Reproducing it in Tcl would be a
        # re-implementation, not a port, so the named native folder is used.
        const_fold_versioned -native string::is
    }

    subcommand map {
        # S110: always builds a character string from the Unicode rep in both
        # 8.6 and 9.0 — the canonical K22406348 corruption step.
        byte_array_effect Coerces
        arity 2..
        detail   {Map substrings via key-value pairs.}
        synopsis {string map ?-nocase? mapping string}
        pure
        return_type String

        option -nocase -detail {Match keys without regard to case.}

        # Mapping: both releases take the dict path only for a *pure* dict and
        # hand everything else to TclListObjGetElements, which installs the
        # List intrep.  Dict is transparent for the pure-dict path only — a
        # dict that has regenerated its string rep does re-parse as a list,
        # which a positional hint cannot see (deliberate under-approximation).
        arg 0 -type List   -shimmers -transparent {Dict}
        # Subject: read via Tcl_GetUnicodeFromObj unconditionally in both
        # releases, so nothing is transparent here.
        arg 1 -type String -shimmers

        const_fold {words ctx} {
            set nocase 0
            if {[lindex $words 0] eq "-nocase"} {
                set nocase 1
                set words [lrange $words 1 end]
            }
            if {[llength $words] != 2} return
            lassign $words mapping s
            if {![string is ascii $mapping] || ![string is ascii $s]} return
            # The registry's fold-safety splitter bails on any backslash
            # rather than decode it; keep that under-approximation so the
            # hook does not fold more than the shipped folder does.
            if {[string first \\ $mapping] >= 0} return
            if {[llength $mapping] % 2 != 0} return
            if {$nocase} {
                fold [string map -nocase $mapping $s]
            } else {
                fold [string map $mapping $s]
            }
        }
    }

    subcommand range {
        semantic_operation {Intrinsic StringRange}
        # S110: keeps a pure byte-array rep in both 8.6 and 9.0
        # (Tcl_GetRange's TclIsPureByteArray path).
        byte_array_effect Transparent
        arity 3
        detail   {Return substring by index range.}
        synopsis {string range string first last}
        pure
        return_type String

        arg 0 -type String -shimmers -transparent {ByteArray}
        arg 1 -type Int    -shimmers
        arg 2 -type Int    -shimmers

        const_fold {words ctx} {
            if {[llength $words] != 3} return
            lassign $words s first last
            if {![string is ascii $s]} return
            # A malformed index raises here, and an error in a hook body is an
            # abstention — the same answer the shipped folder's None gives.
            fold [string range $s $first $last]
        }
    }
}

}
