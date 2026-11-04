# Tcl execution evidence

## Question

> Independent Jim interpreters cannot borrow arbitrary objects merely because their bytes agree. Original child evaluation and alias calls need separately measured string/completion outcomes and pinned extension source evidence for the crossing owner.

> At the pinned Jim interp extension boundary, what one/multiple-source, counted NUL/Unicode string, alias argument/result and non-OK completion outcomes are observed between independently live parent and child interpreters?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One exact ASCII LF source runs in the fresh pinned Jim core/static-extension C API driver without an added outer source envelope. NUL and Unicode are produced at runtime by explicit ASCII backslash-u escapes. The complete original source/request/driver/ELF/compile and process receipts/streams/launcher are retained with required source/header/archive/build/executable hashes. Full separately pinned jim-interp.c and exact source excerpts distinguish source ownership from public equality/results. C8.4–9.1 and BIG-IP are not tested for this Jim extension question; Rust assertions are not executed by publication.

## Script

```tcl-visible
puts [list version [info patchlevel]]
set child [interp]
$child alias echo list
foreach {label value} [list empty {} spaced {A B} nul "A\u0000B" unicode "\u00e9\u20ac" escaped {A\B;C}] {
    set script [list list $value]
    set child_list [$child eval $script]
    set child_value [lindex $child_list 0]
    set alias_list [$child eval [list echo $value]]
    set alias_value [lindex $alias_list 0]
    puts [list roundtrip $label [string length $value] [string equal $value $child_value] [string equal $value $alias_value]]
}
puts [list concat [$child eval {list} {A B}]]
puts [list single [$child eval [list list {A B}]]]
foreach script {{return VALUE} {return -code 7 VALUE} {break} {continue} {error ERR}} {
    set code [catch {$child eval $script} value]
    puts [list completion $script $code $value]
}
namespace eval ::N {
    proc observe {} {
        set x LOCAL
        set child [interp]
        $child alias readX set x
        $child alias where namespace current
        set out [list [$child eval {readX}] [$child eval {where}]]
        $child delete
        return $out
    }
}
puts [list parent_frame [::N::observe]]
$child delete
```

## Results

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|
version 0.84-9-g5bac7c9
roundtrip empty 0 1 1
roundtrip spaced 3 1 1
roundtrip nul 3 1 1
roundtrip unicode 2 1 1
roundtrip escaped 5 1 1
concat {A B}
single {{A B}}
completion {return VALUE} 2 VALUE
completion {return -code 7 VALUE} 2 VALUE
completion break 3 {}
completion continue 4 {}
completion {error ERR} 1 ERR
parent_frame {LOCAL ::N}
```

- Standard error: empty.
