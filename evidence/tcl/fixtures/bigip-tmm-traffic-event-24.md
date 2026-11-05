# BIG-IP fixture: bigip-tmm-traffic-event-24

## Setup

Deploy as described in [tmm-traffic-event](README.md#tmm-traffic-event). Recorded input channels: IAppImplementation; ICallScript; TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST; TmshCliScript; iRule CLIENT_ACCEPTED event; iRule HTTP_REQUEST event; iRule RULE_INIT event.

Each case file fills the template's `⟦HEX:name⟧` slots with the hexadecimal form of its decoded value and its `⟦PARAM:name⟧` slots with the exact value; the composed bytes are the script that ran.

Used by 2 case file(s).

## Template

```tcl-visible
set prefix __tcl_lsp_2286_⟦PARAM:run⟧
set rows {}
foreach pair [list [list format_pre [format %c 233]] [list format_comb [format %c 769]] [list bytes_pre [binary format H* ⟦HEX:payload⟧]] [list bytes_comb [binary format H* ⟦HEX:payload.2⟧]] [list nul [binary format H* ⟦HEX:payload.3⟧]]] {
    set label [lindex $pair 0]
    set suffix [lindex $pair 1]
    set name $prefix
    append name $suffix
    set seed_rc [catch {set $prefix SHORT_PREFIX} seed_result]
    set write_rc [catch {set $name FULL_VALUE} write_result]
    foreach form {unbraced braced} {
        set script "set observed \$"
        if {$form eq "braced"} {
            append script "\{" $name "\}"
        } else {
            append script $name
        }
        binary scan $script H* script_hex
        set rc [catch {eval $script} result]
        binary scan $result H* result_hex
        lappend rows [list $label $form seed $seed_rc write $write_rc script_hex $script_hex eval $rc result_hex $result_hex]
    }
    catch {unset $name}
    catch {unset $prefix}
    catch {unset observed}
}
set rows
```

## Constants

### payload

```tcl-visible
é⟦EOF⟧
```

### payload.2

```tcl-visible
́⟦EOF⟧
```

### payload.3

```tcl-visible
⟦U+0000⟧B⟦EOF⟧
```

