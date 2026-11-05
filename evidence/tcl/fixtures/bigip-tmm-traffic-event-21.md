# BIG-IP fixture: bigip-tmm-traffic-event-21

## Setup

Deploy as described in [tmm-traffic-event](README.md#tmm-traffic-event). Recorded input channels: IAppImplementation; ICallScript; TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST; TmshCliScript.

Each case file fills the template's `⟦HEX:name⟧` slots with the hexadecimal form of its decoded value and its `⟦PARAM:name⟧` slots with the exact value; the composed bytes are the script that ran.

Used by 2 case file(s).

## Template

```tcl-visible
set prefix __tcl_lsp_2286_⟦PARAM:run⟧
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* ⟦HEX:payload⟧]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* ⟦HEX:payload.2⟧]
set rows {}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    binary scan $name H* raw
    lappend rows [list representation $label [string length $name] $raw]
}
foreach pair [list [list $format_pre FORMAT_PRE] [list $bytes_pre BYTES_PRE] [list $format_decomposed FORMAT_DECOMPOSED] [list $bytes_decomposed BYTES_DECOMPOSED]] {
    set rc [catch {set [lindex $pair 0] [lindex $pair 1]} result]
    lappend rows [list write [lindex $pair 1] $rc $result]
}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list read [lindex $pair 0] $rc $result]
}
set rc [catch {upvar 0 $bytes_pre pre_alias; set pre_alias BYTES_PRE_MUTATION} result]
lappend rows [list mutate_bytes_pre $rc $result]
set rc [catch {upvar 0 $bytes_decomposed decomposed_alias; set decomposed_alias BYTES_DECOMPOSED_MUTATION} result]
lappend rows [list mutate_bytes_decomposed $rc $result]
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list after_mutation [lindex $pair 0] $rc $result]
}
set rc [catch {unset $format_pre} result]
lappend rows [list unset_format_pre $rc $result]
set rc [catch {unset $format_decomposed} result]
lappend rows [list unset_format_decomposed $rc $result]
foreach pair [list [list bytes_pre $bytes_pre] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list after_format_unset [lindex $pair 0] $rc $result]
}
foreach name [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {catch {unset $name}}
set rows
```

## Constants

### payload

```tcl-visible
é⟦EOF⟧
```

### payload.2

```tcl-visible
é⟦EOF⟧
```

