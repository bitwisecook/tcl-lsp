# BIG-IP fixture: bigip-tmm-traffic-event-23

## Setup

Deploy as described in [tmm-traffic-event](README.md#tmm-traffic-event). Recorded input channels: IAppImplementation; ICallScript; TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST; TmshCliScript.

Each case file fills the template's `⟦HEX:name⟧` slots with the hexadecimal form of its decoded value and its `⟦PARAM:name⟧` slots with the exact value; the composed bytes are the script that ran.

Used by 2 case file(s).

## Template

```tcl-visible
set prefix __tcl_lsp_2286_⟦PARAM:run⟧
set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
set rows [list [list lengths [string length $pre] [string length $decomposed]]]
foreach name [list $pre $decomposed] {
    set raw_rc [catch {binary scan $name H* raw} raw_result]
    if {$raw_rc != 0} {set raw $raw_result}
    set rc [catch {encoding convertto utf-8 $name} encoded]
    if {$rc == 0} {binary scan $encoded H* encoded}
    lappend rows [list raw_conversion $raw_rc $raw utf8_conversion $rc $encoded]
}
set rc [catch {set $decomposed DECOMPOSED} result]
lappend rows [list write_decomposed $rc $result]
set rc [catch {set $pre PRECOMPOSED} result]
lappend rows [list write_pre $rc $result]
foreach name [list $pre $decomposed] {
    set rc [catch {set $name} result]
    lappend rows [list read $rc $result]
}
set rc [catch {unset $decomposed} result]
lappend rows [list unset_decomposed $rc $result]
set rc [catch {set $pre} result]
lappend rows [list remaining_pre $rc $result]
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
```
