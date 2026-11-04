# Tcl counted-source execution evidence

## Question

> A namespace helper query must establish the actual public procedure-body capability before recording returned declarations or replacement calls. A version-variable existence guard supplies no such helper capability.

> When the original public info-body guard reaches the Jim namespace helper, what formals/body results, global/current helper replacement values and caught unsupported/arity results are observed?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four exact original ASCII LF sources run independently in fresh full provider initialisation under the unchanged counted C API driver, without an added outer source envelope. Public info-body catch is part of the source itself. Whole sources/requests/driver/capture commands/ELFs/stdout/stderr/receipts and required executable/header/archive/source/build pins are retained. Jim public body values and full vendored source assets remain independent channels. The version-variable-gated four-control input and all its actual receipts are losslessly retained under limited-control; its original guard does not reach a helper call. BIG-IP and Rust assertions are not tested. The selected source schema supplies neither an installed scripted helper nor successful forwarding, Native publication/frame/dispatch or Normal completion. Returned public helper bodies and current helper binding remain independent premises.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    list [info args {namespace ensemble}] [info body {namespace ensemble}] [info args ensemble] [info body ensemble]
}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b737562636f6d6d616e6420617267737d207b0a096966207b24737562636f6d6d616e64206e652022637265617465227d207b0a090972657475726e202d636f6465206572726f7220226f6e6c79205c5b6e616d65737061636520656e73656d626c65206372656174655c5d20697320737570706f72746564220a097d0a09736574206e73205b75706c6576656c2031206e616d6573706163652063616e6f6e5d0a0973657420636d6420246e730a096966207b246e732065712022227d207b0a090972657475726e202d636f6465206572726f7220226e616d65737061636520656e73656d626c65206372656174653a206d7573742062652063616c6c65642077697468696e2061206e616d657370616365220a097d0a0a0a09656e73656d626c652024636d64202d6175746f6d617020247b6e737d3a3a207b2a7d24617267730a7d207b636f6d6d616e6420617267737d207b0a09736574206175746f707265666978202224636f6d6d616e6420220a09736574206261646f707473202273686f756c64206265205c22656e73656d626c6520636f6d6d616e64203f2d6175746f6d6170207072656669783f5c22220a096966207b5b6c6c656e6774682024617267735d2025203220213d20307d207b0a090972657475726e202d636f6465206572726f72202277726f6e67202320617267733a20246261646f707473220a097d0a09666f7265616368207b6f70742076616c75657d202461726773207b0a0909737769746368202d2d20246f7074207b0a0909092d6175746f6d6170207b20736574206175746f707265666978202476616c7565207d0a09090964656661756c74207b2072657475726e202d636f6465206572726f72202277726f6e67202320617267733a20246261646f70747322207d0a09097d0a097d0a0970726f632024636f6d6d616e64207b737562636d6420617267737d207b6175746f707265666978207b6d617070696e67207b7d7d7d207b0a09096966207b215b646963742065786973747320246d617070696e672024737562636d645d7d207b0a0a0909096966207b24737562636d6420696e207b2d636f6d6d616e6473202d68656c707d7d207b0a0a09090909736574207072656669786c656e205b737472696e67206c656e67746820246175746f7072656669785d0a0909090973657420737562636d6473205b6c6d61702070205b6c736f7274205b696e666f20636f6d6d616e6473202d616c6c20246175746f7072656669782a5d5d207b0a0909090909737472696e672072616e676520247020247072656669786c656e20656e640a090909097d5d0a090909096966207b24737562636d6420657120222d636f6d6d616e6473227d207b0a090909090972657475726e2024737562636d64730a090909097d0a0909090973657420636f6d6d616e64205b6c696e646578205b696e666f206c6576656c20305d20305d0a0909090972657475726e202255736167653a205c2224636f6d6d616e6420636f6d6d616e64202e2e2e205c222c20776865726520636f6d6d616e64206973206f6e65206f663a205b6a6f696e2024737562636d647320222c20225d220a0909097d0a0a0909096469637420736574206d617070696e672024737562636d6420247b6175746f7072656669787d24737562636d640a09097d0a0a09097461696c63616c6c205b646963742067657420246d617070696e672024737562636d645d207b2a7d24617267730a097d0a7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{subcommand args} {
⟦TAB⟧if {$subcommand ne "create"} {
⟦TAB⟧⟦TAB⟧return -code error "only \[namespace ensemble create\] is supported"
⟦TAB⟧}
⟦TAB⟧set ns [uplevel 1 namespace canon]
⟦TAB⟧set cmd $ns
⟦TAB⟧if {$ns eq ""} {
⟦TAB⟧⟦TAB⟧return -code error "namespace ensemble create: must be called within a namespace"
⟦TAB⟧}


⟦TAB⟧ensemble $cmd -automap ${ns}:: {*}$args
} {command args} {
⟦TAB⟧set autoprefix "$command "
⟦TAB⟧set badopts "should be \"ensemble command ?-automap prefix?\""
⟦TAB⟧if {[llength $args] % 2 != 0} {
⟦TAB⟧⟦TAB⟧return -code error "wrong # args: $badopts"
⟦TAB⟧}
⟦TAB⟧foreach {opt value} $args {
⟦TAB⟧⟦TAB⟧switch -- $opt {
⟦TAB⟧⟦TAB⟧⟦TAB⟧-automap { set autoprefix $value }
⟦TAB⟧⟦TAB⟧⟦TAB⟧default { return -code error "wrong # args: $badopts" }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧}
⟦TAB⟧proc $command {subcmd args} {autoprefix {mapping {}}} {
⟦TAB⟧⟦TAB⟧if {![dict exists $mapping $subcmd]} {

⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$subcmd in {-commands -help}} {

⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧set prefixlen [string length $autoprefix]
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧set subcmds [lmap p [lsort [info commands -all $autoprefix*]] {
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧string range $p $prefixlen end
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧}]
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$subcmd eq "-commands"} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧return $subcmds
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧set command [lindex [info level 0] 0]
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧return "Usage: \"$command command ... \", where command is one of: [join $subcmds ", "]"
⟦TAB⟧⟦TAB⟧⟦TAB⟧}

⟦TAB⟧⟦TAB⟧⟦TAB⟧dict set mapping $subcmd ${autoprefix}$subcmd
⟦TAB⟧⟦TAB⟧}

⟦TAB⟧⟦TAB⟧tailcall [dict get $mapping $subcmd] {*}$args
⟦TAB⟧}
}⟦EOF⟧
```
