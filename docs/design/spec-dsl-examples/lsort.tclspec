# Port of rust/tcl-registry/src/commands/tcl/lsort_.rs
#
# Exercises: option table (flags, value options, a command-prefix option with
# an appended arity, an integer-domain option), per-option dialect gates,
# forms, hover.

speclib tcl 1.0 {

command lsort {
    dialects {all-tcl f5-irules}

    # NOT PURE/CSE_CANDIDATE: `-command cmdPrefix` names an arbitrary command
    # that runs with the interpreter's full state as part of the sort, and the
    # purity classifier has no per-option carve-out.  FRAMELESS_RUNTIME stays:
    # cmd_lsort is a dedicated runtime helper and the comparator is dispatched
    # as an ordinary nested call, never an uplevel-style scope walk.
    traits {FRAMELESS_RUNTIME BYTE_COMPILED}

    arity 1..
    return_type           List
    inferred_storage_type List

    # All 12 documented switches, cross-checked against lsort(n) for 8.4-9.1.
    option -ascii -detail {Compare using Unicode code-point (raw string) order — the default. The flag name is a holdover from Tcl's original ASCII-only implementation; it is not restricted to ASCII text.}

    option -dictionary -detail {Dictionary-style comparison: like -ascii but case-insensitive except as a tie-breaker, and embedded numbers compare as integers rather than character-by-character (bigBoy sorts between bigbang and bigboy; x10y sorts between x9y and x11y). Takes precedence over -nocase.}

    option -integer -detail {Convert each element to an integer and compare numerically; an element that doesn't convert is an error.}

    option -real -detail {Convert each element to a floating-point value and compare numerically; an element that doesn't convert is an error.}

    # Added in Tcl 8.5 (absent from the 8.4 manpage's option list).
    option -nocase -dialects tcl8.5+ -detail {Case-insensitive comparison. Only affects -ascii comparisons — has no effect combined with -dictionary, -integer, or -real.}

    option -increasing -detail {Sort in increasing order, smallest items first (the default).}

    option -decreasing -detail {Sort in decreasing order, largest items first.}

    option -indices -dialects tcl8.5+ -detail {Return the sorted positions (indices) into list instead of the elements themselves.}

    option -unique -detail {Retain only the last element of each run of duplicates in the sorted result. Duplicates are determined by the comparison in use — e.g. with -index 0, {1 a} and {1 b} count as duplicates and only {1 b} is kept.}

    # Invoked as `cmdPrefix elem1 elem2` → exactly 2 appended arguments
    # (identical wording in every fetched manpage version).
    option -command -takes cmdPrefix -role CommandPrefix -appends {Exactly 2} \
        -detail {Use cmdPrefix as the comparator: invoked with the two elements being compared appended as additional arguments, and must return an integer less than, equal to, or greater than zero if the first is respectively less than, equal to, or greater than the second. lsort is reentrant, so cmdPrefix may itself call lsort.}

    option -index -takes indexList -detail {Sort by the sub-element at indexList within each list element (itself treated as a sublist, unless -stride is also given) instead of the whole element; indexList accepts end/end-N and, since Tcl 8.5, may itself be a list of indices for nested sublist access (as if passed to lindex). Combined with -stride, the index is relative to each group. Much more efficient than an equivalent -command comparator.}

    # TIP 326 — NOT the same option as lsearch's later, 9.0-only -stride.
    option -stride -takes strideLength -integer {Range 2 max} -dialects tcl8.6+ \
        -detail {Treat list as consecutive groups of strideLength elements (strideLength must be at least 2, and list's length a multiple of it), keeping each group's element order fixed while sorting whole groups by their first element, or by the -index'th element within each group when -index is also given.}

    # The single invocation form, unchanged across 8.4 through 9.1.
    form Default {lsort ?options? list}

    hover {
        summary {Sort the elements of a list.}
        synopsis {lsort ?options? list}
        description {Sorts using a stable merge sort (O(n log n)); equal elements keep their relative input order, except that -unique discards all but the last of each run of duplicates. -ascii (Unicode code-point order) and -increasing are the defaults; -dictionary, -integer, -real, or -command select an alternate comparison. -nocase (Tcl 8.5+) folds case, but only affects -ascii comparisons — it has no effect combined with -dictionary, -integer, or -real. -index sorts by a sub-element instead of the whole list element — since Tcl 8.5 it may itself be a list of indices for nested access — and, combined with -stride (Tcl 8.6+), is relative to each fixed-size group rather than the whole element. -indices (Tcl 8.5+) returns element positions instead of the elements themselves. lsort is reentrant, so a -command comparator may itself call lsort.}
        source {Tcl lsort(n)}
        example {lsort {b a c}
lsort -integer -unique {3 1 4 1 5 9 2 6}
lsort -integer -index 1 {{First 24} {Second 18} {Third 30}}
lsort -stride 2 -index 1 -integer {carrot 10 apple 50 banana 25}}
        returns {A new list containing the same elements as list, permuted into sorted order (or, with -indices, the list of indices that would produce that order).}
    }
}

}
