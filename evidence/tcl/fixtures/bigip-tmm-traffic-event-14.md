# BIG-IP fixture: bigip-tmm-traffic-event-14

## Setup

Deploy as described in [tmm-traffic-event](README.md#tmm-traffic-event). Recorded input channels: IAppImplementation; ICallScript; TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST; TmshCliScript.

Each case file fills the template's `⟦HEX:name⟧` slots with the hexadecimal form of its decoded value and its `⟦PARAM:name⟧` slots with the exact value; the composed bytes are the script that ran.

Used by 2 case file(s).

## Template

```tcl-visible
set prefix __tcl_lsp_2286_⟦PARAM:run⟧
set pre $prefix
append pre [binary format H* ⟦HEX:payload⟧]
set decomposed $prefix
append decomposed [binary format H* ⟦HEX:payload.2⟧]
set rows {}
foreach pair [list [list pre $pre PRE_VALUE] [list decomposed $decomposed DECOMPOSED_VALUE]] {
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    set value [lindex $pair 2]
    set write_rc [catch {set $name $value} write_result]
    set script "set \$"; append script $name
    binary scan $script H* script_hex
    set rc [catch {eval $script} result]
    binary scan $result H* result_hex
    lappend rows [list $label write $write_rc $write_result script_hex $script_hex eval $rc $result_hex]
}
foreach name [list $pre $decomposed] {catch {unset $name}}
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

