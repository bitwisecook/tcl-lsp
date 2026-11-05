# BIG-IP fixture: bigip-tmm-traffic-event-8

## Setup

Deploy as described in [tmm-traffic-event](README.md#tmm-traffic-event). Recorded input channels: IAppImplementation; ICallScript; TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST; TmshCliScript; iRule CLIENT_ACCEPTED event; iRule HTTP_REQUEST event; iRule RULE_INIT event.

Each case file fills the template's `⟦HEX:name⟧` slots with the hexadecimal form of its decoded value and its `⟦PARAM:name⟧` slots with the exact value; the composed bytes are the script that ran.

Used by 3 case file(s).

## Template

```tcl-visible
set prefix __tcl_lsp_2286_⟦PARAM:run⟧
set plain ${prefix}A
set counted $plain
append counted [binary format H* ⟦HEX:payload⟧]
set rows [list [list names [string length $plain] [string length $counted]]]
foreach pair [list [list $plain PLAIN] [list $counted COUNTED]] {
    set name [lindex $pair 0]
    set value [lindex $pair 1]
    set rc [catch {set $name $value} result]
    lappend rows [list write [string length $name] $rc $result]
}
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list read [string length $name] $rc $result]
}
set rc [catch {upvar 0 $counted ${prefix}link; set ${prefix}link LINK_WRITE} result]
lappend rows [list upvar_write $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_link [string length $name] $rc $result]
}
set rc [catch {unset $counted} result]
lappend rows [list unset_counted $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_unset [string length $name] $rc $result]
}
foreach name [list $plain $counted ${prefix}link] {catch {unset $name}}
set rows
```

## Constants

### payload

```tcl-visible
⟦U+0000⟧B⟦EOF⟧
```

