# Tcl execution evidence

## Question

> Same-interpreter alias publication can expose different public names for plain, relative-qualified and explicitly rooted input. A global-only installer can lose that distinction, and an incompatible Jim interp interface cannot answer the C API question.

> Does interp alias publication in the same interpreter use the current namespace holder for an unqualified or relative-qualified alias name, or the root holder, and how do explicit root names compare?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One fixed ASCII LF file per fresh provider process creates namespace N and its child q, then measures same-interpreter alias calls for q::a, plain and ::N::q::rooted. Exact info commands patterns and caught call results are retained. No NUL, Unicode, control escape, arbitrary namespace path, failed namespace creation, missing destination namespace, multiple interpreter or alias lifetime scenario is exercised. This is the actual C interp alias syntax on every provider; Jim incompatible syntax is caught, with no alternative API substituted.

## Script

```tcl-visible
puts [list version [info patchlevel] interp_available [info commands interp]]
if {[llength [info commands interp]] == 0} {
    puts [list current_holder_scope not_applicable]
    exit 0
}
namespace eval N {namespace eval q {}}
set code [catch {namespace eval N {interp alias {} q::a {} list VALUE}} result]
puts [list relative_qualified $code $result [info commands ::N::q::a] [info commands ::q::a]]
set local_code [catch {::N::q::a} local_result]
set root_code [catch {::q::a} root_result]
puts [list relative_calls $local_code $local_result $root_code $root_result]
set code [catch {namespace eval N {interp alias {} plain {} list PLAIN}} result]
puts [list unqualified $code $result [info commands ::N::plain] [info commands ::plain]]
set code [catch {namespace eval N {interp alias {} ::N::q::rooted {} list ROOTED}} result]
set call_code [catch {::N::q::rooted} call_result]
puts [list explicitly_rooted $code $result [info commands ::N::q::rooted] $call_code $call_result]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 interp_available interp
relative_qualified 0 q::a ::N::q::a {}
relative_calls 0 VALUE 1 {invalid command name "::q::a"}
unqualified 0 plain {} ::plain
explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 interp_available interp
relative_qualified 0 q::a ::N::q::a {}
relative_calls 0 VALUE 1 {invalid command name "::q::a"}
unqualified 0 plain {} ::plain
explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 interp_available interp
relative_qualified 0 q::a ::N::q::a {}
relative_calls 0 VALUE 1 {invalid command name "::q::a"}
unqualified 0 plain {} ::plain
explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 interp_available interp
relative_qualified 0 q::a ::N::q::a {}
relative_calls 0 VALUE 1 {invalid command name "::q::a"}
unqualified 0 plain {} ::plain
explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 interp_available interp
relative_qualified 0 q::a ::N::q::a {}
relative_calls 0 VALUE 1 {invalid command name "::q::a"}
unqualified 0 plain {} ::plain
explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 interp_available interp
relative_qualified 1 {wrong # args: should be "interp"} {} {}
relative_calls 1 {invalid command name "::N::q::a"} 1 {invalid command name "::q::a"}
unqualified 1 {wrong # args: should be "interp"} {} {}
explicitly_rooted 1 {wrong # args: should be "interp"} {} 1 {invalid command name "::N::q::rooted"}
```

- Standard error: empty.
