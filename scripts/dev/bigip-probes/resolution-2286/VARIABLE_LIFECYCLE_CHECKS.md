# BIG-IP original variable operands and callback lifetime

Measure which original variable receiver survives a read callback, unset,
recreation and a later output assignment. These observations support the shared
variable-access and trace-lifetime interfaces. C Tcl and Jim captures do not
establish a BIG-IP result or its native object representation.

Use branch `work` and write the peer `VARIABLE_LIFECYCLE_RESULTS.md` beside this
request. Record exact BIG-IP product/version/build/hotfix and source commit at
the beginning. Commit and push the report, complete sources and raw evidence
on the same branch. Use [CONTEXT_NAMING_RESULTS.md](CONTEXT_NAMING_RESULTS.md)
for measured context coverage and [CONTEXT_NAMING_CHECKS.md](CONTEXT_NAMING_CHECKS.md)
for its exact controls. Keep execution contexts distinct; equal naming results
do not establish callback lifetime or native object representation.

## Execution and evidence

- Clients and servers may both run on `dev.bragi0.com`, including loopback
  testing with any suitable protocol. Every test virtual server requires
  **SNAT automap**. Record VIP, pool/backend path, request/reply bytes and the
  peer address seen by the server. The HTTP wrapper below requires an HTTP
  profile; use a separately hashed event-valid wrapper for another protocol.
- Capture `/var/log/ltm` continuously before creation, during traffic and
  through cleanup. Keep complete loader output and actual installed source.
  Record CMP configuration and the TMM roster, then identify reached units
  from each request. Do not infer coverage from request count.
- Use fresh owned names with an ASCII alphanumeric run identifier. Verify
  name absence first and source SHA-256 after transfer. Remove only owned
  objects and retain object-absence proof. Do not restart TMM, change global
  CMP or save global configuration.
- Preserve load rejections, runtime failures and unsupported commands as
  observations. Keep every adapted source separately hashed and explain the
  exact operand/frame change. A successful standalone `tclsh` run is not an
  appliance measurement. Do not inspect or claim native object headers,
  compiler hooks or ABI from equal script output.

## Complete active read/unset discriminator

Replace every `RUN_TOKEN` with the same fresh identifier. All source bytes below
are ASCII with LF line endings; there are no non-ASCII characters, literal NULs
or continued physical lines. Keep braces, command substitutions and list
quoting exactly. `EXTRA` is one list element. The callback receives its original
name, index and operation as separate arguments; log their bytes independently.

```tcl
proc R2286_RUN_TOKEN_hex {value} {
    binary scan $value H* encoded
    return $encoded
}
proc R2286_RUN_TOKEN_watch {name index operation} {
    upvar 1 R2286_RUN_TOKEN_callbacks callbacks
    lappend callbacks [list $operation [R2286_RUN_TOKEN_hex $name] [R2286_RUN_TOKEN_hex $index]]
    if {$operation eq "read" || $operation eq "r"} {
        uplevel 1 {unset R2286_RUN_TOKEN_value; set R2286_RUN_TOKEN_value NEW}
    }
}
when HTTP_REQUEST {
    if {[HTTP::uri] eq "/r2286-RUN_TOKEN-backend"} {
        return
    }
    set R2286_RUN_TOKEN_rows {}
    foreach R2286_RUN_TOKEN_syntax {modern legacy} {
        catch {unset R2286_RUN_TOKEN_value}
        set R2286_RUN_TOKEN_callbacks {}
        set R2286_RUN_TOKEN_value OLD
        if {$R2286_RUN_TOKEN_syntax eq "modern"} {
            set R2286_RUN_TOKEN_install [catch {trace add variable R2286_RUN_TOKEN_value {read unset} R2286_RUN_TOKEN_watch} R2286_RUN_TOKEN_install_result]
        } else {
            set R2286_RUN_TOKEN_install [catch {trace variable R2286_RUN_TOKEN_value ru R2286_RUN_TOKEN_watch} R2286_RUN_TOKEN_install_result]
        }
        set R2286_RUN_TOKEN_code [catch {
            set R2286_RUN_TOKEN_head lappend
            set R2286_RUN_TOKEN_answer [$R2286_RUN_TOKEN_head R2286_RUN_TOKEN_value EXTRA]
            list $R2286_RUN_TOKEN_answer [set R2286_RUN_TOKEN_value]
        } R2286_RUN_TOKEN_result]
        lappend R2286_RUN_TOKEN_rows [list syntax $R2286_RUN_TOKEN_syntax install $R2286_RUN_TOKEN_install install_hex [R2286_RUN_TOKEN_hex $R2286_RUN_TOKEN_install_result] operation $R2286_RUN_TOKEN_code result_hex [R2286_RUN_TOKEN_hex $R2286_RUN_TOKEN_result] callbacks $R2286_RUN_TOKEN_callbacks]
        if {$R2286_RUN_TOKEN_syntax eq "modern"} {
            catch {trace remove variable R2286_RUN_TOKEN_value {read unset} R2286_RUN_TOKEN_watch}
        } else {
            catch {trace vdelete R2286_RUN_TOKEN_value ru R2286_RUN_TOKEN_watch}
        }
        catch {unset R2286_RUN_TOKEN_value}
    }
    log local0.notice [list RESOLUTION2286_VARIABLE_LIFECYCLE RUN_TOKEN tmm [TMM::cmp_unit] rows $R2286_RUN_TOKEN_rows]
    HTTP::respond 200 content [list RUN_TOKEN $R2286_RUN_TOKEN_rows]
}
```

Record the installation result separately from the operation result. The
`/r2286-RUN_TOKEN-backend` path continues to the configured pool; drive it to
prove the client/VIP/server path and SNAT peer address. Use another path for the
discriminator, whose `HTTP::respond` reply is produced by the iRule itself. The
discriminators are whether an unset callback runs during the active read,
whether the final read invokes a destroyed registration again, and whether the
returned list and recreated variable both contain `NEW EXTRA`. Measure the
appliance outcome; this statement specifies what to compare, not an expected
BIG-IP answer. If a TMM identity command or top-level procedure is unavailable,
retain its exact rejection and a separately hashed supported wrapper.

## Required additional controls

Use the complete original programs in
[`native_generic_variable_consumers`](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/README.md)
and its `active-unset` directory as source references. Keep their full original
bytes and separately record each appliance wrapper. Replace their final
`puts $summary` with a recorded result/logging wrapper when stdout is
unavailable; that is an adaptation, not the original source hash.

1. Repeat active read/unset with an array element, an older pending read
   registration, and recursive ordinary read/write. Record callback order and
   exact root/index bytes. Ordinary recursion suppression and unset delivery
   are different purposes.
2. Run the `lset` receiver-recreation control and both `lappend` forms: zero
   additions and a nonempty addition. A missing quiet read and a callback error
   need separate captures. Record error options only through a supported
   mechanism. A `::errorCode` control is explicitly global: put it on a separate
   owned virtual server and record any CMP demotion. An event-local variable
   named `errorCode` does not prove interpreter error-state behavior.
3. Run sequential `scan` output assignment with a first-destination write
   callback that rebinds the second destination. Capture both `upvar #0`
   (global) and an explicitly same-event-frame `upvar 0` variant separately.
   Do not transplant a standalone top-level global assumption into an event
   frame. Verify which replacement cell receives the second output.
4. For supported generic commands, repeat plain, qualified, array-index,
   byte-produced and decomposed names. Exact reference byte producers are
   `binary format H* ff0078` (FF, NUL, ASCII x) and `e\u0301` (e plus U+0301,
   not precomposed U+00E9). Preserve source escapes and measure getter-produced
   bytes independently; do not assume a UTF encoding or normalization.
5. Repeat representative accepted controls in CLIENT_ACCEPTED and RULE_INIT
   with event-valid wrappers. Keep event locals, ordinary globals and
   `static::` storage separate. For `static::` mutation, record RULE_INIT seed
   availability, executing TMM and the values observed on every reached unit.
   A write on one unit does not establish broadcast to other units.

## Result contract

The peer report must contain a case/context/TMM matrix with exact source hashes,
load result, operation completion/result bytes, callback order/root/index bytes,
selected receiver discriminators and observed CMP effects. Link complete raw
logs, installed configurations, traffic captures, evidence manifest and cleanup
proof. State unsupported and unreached cases explicitly. Conclusions apply
only to measured input domains, purposes, execution contexts and appliance
version/build.
