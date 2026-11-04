# BIG-IP active variable lifecycle, modern and legacy traces

## Question

When a read trace unsets and recreates a scalar during lappend, what values and callbacks result with the modern and legacy trace APIs? This constrains trace dispatch and variable-cell lifetime in the compiler and VM.

## Assumptions

This payload ran under catch in an HTTP_REQUEST wrapper after the literal rule was rejected. It constructs the comparison within one request; the results do not establish behavior in other contexts or versions. The response bodies do not themselves record TMM identity or an outer completion code.

## Script

```tcl-visible
proc R2286_vl1_hex {value} {
    binary scan $value H* encoded
    return $encoded
}
proc R2286_vl1_watch {name index operation} {
    upvar 1 R2286_vl1_callbacks callbacks
    lappend callbacks [list $operation [R2286_vl1_hex $name] [R2286_vl1_hex $index]]
    if {$operation eq "read" || $operation eq "r"} {
        uplevel 1 {unset R2286_vl1_value; set R2286_vl1_value NEW}
    }
}
    if {[HTTP::uri] eq "/r2286-vl1-backend"} {
        return
    }
    set R2286_vl1_rows {}
    foreach R2286_vl1_syntax {modern legacy} {
        catch {unset R2286_vl1_value}
        set R2286_vl1_callbacks {}
        set R2286_vl1_value OLD
        if {$R2286_vl1_syntax eq "modern"} {
            set R2286_vl1_install [catch {trace add variable R2286_vl1_value {read unset} R2286_vl1_watch} R2286_vl1_install_result]
        } else {
            set R2286_vl1_install [catch {trace variable R2286_vl1_value ru R2286_vl1_watch} R2286_vl1_install_result]
        }
        set R2286_vl1_code [catch {
            set R2286_vl1_head lappend
            set R2286_vl1_answer [$R2286_vl1_head R2286_vl1_value EXTRA]
            list $R2286_vl1_answer [set R2286_vl1_value]
        } R2286_vl1_result]
        lappend R2286_vl1_rows [list syntax $R2286_vl1_syntax install $R2286_vl1_install install_hex [R2286_vl1_hex $R2286_vl1_install_result] operation $R2286_vl1_code result_hex [R2286_vl1_hex $R2286_vl1_result] callbacks $R2286_vl1_callbacks]
        if {$R2286_vl1_syntax eq "modern"} {
            catch {trace remove variable R2286_vl1_value {read unset} R2286_vl1_watch}
        } else {
            catch {trace vdelete R2286_vl1_value ru R2286_vl1_watch}
        }
        catch {unset R2286_vl1_value}
    }
    log local0.notice [list RESOLUTION2286_VARIABLE_LIFECYCLE vl1 tmm [TMM::cmp_unit] rows $R2286_vl1_rows]
    HTTP::respond 200 content [list vl1 $R2286_vl1_rows]
```

## Results

### BIG-IP HTTP_REQUEST responses

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel not separately recorded by this run.
- Recorded input channel: HTTP_REQUEST, evaluated by the supported wrapper.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
vl1 {{syntax modern install 0 install_hex {} operation 0 result_hex 7b4e45572045585452417d207b4e45572045585452417d callbacks {{read 52323238365f766c315f76616c7565 {}} {unset 52323238365f766c315f76616c7565 {}}}} {syntax legacy install 0 install_hex {} operation 0 result_hex 7b4e45572045585452417d207b4e45572045585452417d callbacks {{r 52323238365f766c315f76616c7565 {}} {u 52323238365f766c315f76616c7565 {}}}}}⟦EOF⟧
```

Four retained HTTP response bodies were byte-identical.
