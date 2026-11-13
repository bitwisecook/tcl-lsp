# Procedure args formal and defaults

## Question

Which argument counts does a procedure accept when a formal is named exactly
`args` in the last, a middle or the first position, when it is written
`{args name}`, when a formal is spelled `ARGS`, and when a defaulted formal
comes before or after the required ones?

tcl-lsp publishes each workspace procedure to other documents as a stub whose
argument counts must be the ones the interpreter accepts: a user pack answers
for the procedure only when it accepts every one of those counts (#2532), and
a procedure with an unbounded rest formal has no stub spelling. The result
covers procedure argument binding in these interpreters only, not `apply` or
TclOO methods.

## Assumptions

- Each interpreter ran the file as `tclsh script.tcl` / `jimsh script.tcl` in a
  fresh process, with `TCL_LIBRARY` set to that release's own `library`
  directory for C Tcl.
- The C Tcl interpreters are the unix builds under `tmp/` from
  `make ensure-tcl-deps`; Jim is the revision pinned in
  `rust/tcl-test-support/jim-reference.txt`, built by
  `scripts/dev/ensure-jim-oracle.sh`.
- A `0` after `->` is a successful call and `1` an error; the error message is
  the interpreter's own. An error raised inside the body also prints `1`: on
  C Tcl, `renamed` binds and then fails reading `$rest`, which it never set.
- No BIG-IP execution is recorded for this script.

## Script

```tcl-visible
proc last {a args} { list a=$a args=$args }
proc middle {name args body} { list name=$name args=$args body=$body }
proc first {args b} { list args=$args b=$b }
proc renamed {a {args rest}} { list a=$a rest=$rest }
proc named {a ARGS} { list a=$a ARGS=$ARGS }
proc gap {a {b B} c} { list a=$a b=$b c=$c }
proc tail {a {b B}} { list a=$a b=$b }
foreach call {
    {last 1} {last 1 2 3}
    {middle n} {middle n b} {middle n x b} {middle n x y b}
    {first 1} {first 1 2 3}
    {renamed 1} {renamed 1 2 3}
    {named 1 2 3}
    {gap 1 2} {gap 1 2 3}
    {tail 1} {tail 1 2}
} {
    puts "$call -> [catch $call result] $result"
}
```

## Results

### Tcl 8.4.20 (`info patchlevel` 8.4.20)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name args body"
middle n b -> 1 wrong # args: should be "middle name args body"
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 1 wrong # args: should be "middle name args body"
first 1 -> 1 wrong # args: should be "first args b"
first 1 2 3 -> 1 wrong # args: should be "first args b"
renamed 1 -> 1 can't read "rest": no such variable
renamed 1 2 3 -> 1 can't read "rest": no such variable
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 1 wrong # args: should be "gap a ?b? c"
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.

### Tcl 8.5.19 (`info patchlevel` 8.5.19)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name args body"
middle n b -> 1 wrong # args: should be "middle name args body"
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 1 wrong # args: should be "middle name args body"
first 1 -> 1 wrong # args: should be "first args b"
first 1 2 3 -> 1 wrong # args: should be "first args b"
renamed 1 -> 1 can't read "rest": no such variable
renamed 1 2 3 -> 1 can't read "rest": no such variable
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 1 wrong # args: should be "gap a ?b? c"
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.

### Tcl 8.6.18 (`info patchlevel` 8.6.18)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name args body"
middle n b -> 1 wrong # args: should be "middle name args body"
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 1 wrong # args: should be "middle name args body"
first 1 -> 1 wrong # args: should be "first args b"
first 1 2 3 -> 1 wrong # args: should be "first args b"
renamed 1 -> 1 can't read "rest": no such variable
renamed 1 2 3 -> 1 can't read "rest": no such variable
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 1 wrong # args: should be "gap a ?b? c"
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.

### Tcl 9.0.4 (`info patchlevel` 9.0.4)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name args body"
middle n b -> 1 wrong # args: should be "middle name args body"
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 1 wrong # args: should be "middle name args body"
first 1 -> 1 wrong # args: should be "first args b"
first 1 2 3 -> 1 wrong # args: should be "first args b"
renamed 1 -> 1 can't read "rest": no such variable
renamed 1 2 3 -> 1 can't read "rest": no such variable
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 1 wrong # args: should be "gap a ?b? c"
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.

### Tcl 9.1.0 (`info patchlevel` 9.1.0)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name args body"
middle n b -> 1 wrong # args: should be "middle name args body"
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 1 wrong # args: should be "middle name args body"
first 1 -> 1 wrong # args: should be "first args b"
first 1 2 3 -> 1 wrong # args: should be "first args b"
renamed 1 -> 1 can't read "rest": no such variable
renamed 1 2 3 -> 1 can't read "rest": no such variable
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 1 wrong # args: should be "gap a ?b? c"
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.

### Jim (`info patchlevel` 0.84-10-g82fb4d0)

- Invocation: `jimsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
last 1 -> 0 a=1 args=
last 1 2 3 -> 0 a=1 {args=2 3}
middle n -> 1 wrong # args: should be "middle name ?arg ...? body"
middle n b -> 0 name=n args= body=b
middle n x b -> 0 name=n args=x body=b
middle n x y b -> 0 name=n {args=x y} body=b
first 1 -> 0 args= b=1
first 1 2 3 -> 0 {args=1 2} b=3
renamed 1 -> 0 a=1 rest=
renamed 1 2 3 -> 0 a=1 {rest=2 3}
named 1 2 3 -> 1 wrong # args: should be "named a ARGS"
gap 1 2 -> 0 a=1 b=B c=2
gap 1 2 3 -> 0 a=1 b=2 c=3
tail 1 -> 0 a=1 b=B
tail 1 2 -> 0 a=1 b=2
```

- Standard error: empty.
