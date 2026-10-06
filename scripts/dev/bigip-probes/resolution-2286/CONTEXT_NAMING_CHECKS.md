# BIG-IP naming across execution contexts

Measure the naming semantics needed by the shared Tcl naming interfaces. A result in one BIG-IP execution context does not establish the same behavior in another. The reference reports are [BIGIP_RESULTS.md](BIGIP_RESULTS.md) and [FOLLOWUP_APPLIANCE_RESULTS.md](FOLLOWUP_APPLIANCE_RESULTS.md). They supply measured TMM HTTP_REQUEST naming controls; the cross-context controls in the follow-up report cover expressions.

Work on branch `work`. Write the peer `CONTEXT_NAMING_RESULTS.md` beside this document, retain the complete sources and raw evidence, commit and push on the same branch. State exact BIG-IP version/build/hotfix and the source commit prominently. Documentation describes measured behavior and current coverage only.

## Required evidence

- Record exact product/version/build, topology, CMP configuration, actual TMM roster, source commit and every source SHA-256. Keep TMM, tmsh CLI script, iApp implementation, iApp presentation/APL and iCall observations separate.
- Clients and servers may both run on `dev.bragi0.com`, including loopback testing with any suitable protocol. Every test BIG-IP virtual server requires **SNAT automap**. Prove the client/VIP/backend path and backend peer address when forwarding traffic.
- Capture `/var/log/ltm` continuously from before object creation through cleanup. Retain raw loader output, configurations, request/reply bytes, script output and source/hash verification on the appliance.
- Use fresh ASCII alphanumeric run identifiers and exact disposable object names. Record pre-existing-name checks. Delete only owned objects; keep unsuccessful deletion and actual object absence distinct. Do not restart TMM, alter global CMP, save global configuration or touch residual objects from another run.
- Record successful measurements, load/runtime rejections, unavailable mechanisms and unreached cases separately. Keep every repaired source as a separately hashed case. Do not use standalone `tclsh` as a substitute for a BIG-IP context.

## Exact naming payloads

Generate fresh sources with [generate-followup-controls.py](generate-followup-controls.py). Its required arguments are `--run`, `--out`, `--vip`, `--backend`, `--backend-port`, `--vip-port`; record `--source-commit` explicitly. Choose unused addresses/ports appropriate to the lab. The existing [v6 sources](evidence/r2286_20261006m/fixtures/v6/) show the complete payloads and wrappers, and their manifest records exact bytes and hashes.

Run these complete generated `.tcl` payloads in each supported context:

| Payloads | Naming question |
| --- | --- |
| `nul_counted_last`, `nul_plain_last`, `nul_plain_unset_first` | Can a scalar key containing NUL coexist with its plain prefix? Do read, write, upvar and unset preserve that distinction? |
| `nul_array_root`, `nul_array_index` | Are array root and index length conventions independent? What exact root/index bytes reach trace callbacks and aliases? |
| `unicode_format_forward`, `unicode_format_reverse` | What bytes does `format %c` produce, and which keys do those bytes address? |
| `unicode_bytes_forward`, `unicode_bytes_reverse`, `unicode_cross_producer` | Do byte-produced and format-produced spellings coexist, mutate and unset independently? |
| `commands_format`, `commands_bytes`, `commands_cross_producer` | How do namespace/proc creation, command lookup, invocation and rename treat the producer-specific spellings? |
| `lexical_format_unbraced`, `lexical_format_braced`, `lexical_bytes_unbraced`, `lexical_bytes_braced` | Which exact bytes belong to a lexical variable token? |

Preserve each payload byte-for-byte between contexts. Each context wrapper is independently hashed. Generate non-TMM wrappers with [generate-followup-contexts.py](generate-followup-contexts.py), passing `--run`, `--payload`, `--out` and `--source-commit`. Its full `cli.conf`, `iapp.conf`, `iapp-service.conf` and `icall.conf` are the source definitions; use [run-followup-contexts.sh](run-followup-contexts.sh) and the report's reproduction instructions where applicable. An iApp implementation must actually execute; object creation alone is insufficient. An iCall handler must actually deliver the configured event.

The generated payload files are **ASCII, LF-only and contain no literal NUL**. Runtime `binary format H* 0042` creates NUL followed by ASCII `B`; it is not printable backslash-zero. Runtime `c3a9` and `65cc81` create the stated byte sequences. `format %c 233` and `%c 769` are separate producers whose output must be measured, not assumed to be UTF-8. Preserve physical line breaks, braces and backslashes. Transport scripts/configurations without text conversion and verify hashes after transfer.

## Context matrix

Run the payloads in tmsh CLI script, iApp implementation and triggered iCall. Record each context's own patchlevel and command availability when obtainable. Repeat representative scalar-NUL, array-root/index-NUL, cross-producer and clean lexical controls in bounded periodic iCall and perpetual iCall only where a documented finite lifecycle is available; report unavailable lifecycle support rather than leaving a handler running. For iApp presentation/APL, use a documented presentation execution path and record the exact executed Tcl/APL wrapper. If a payload cannot execute there, retain its rejection and the reached boundary.

Use TMM HTTP_REQUEST as the same-run comparison. Repeat scalar-NUL, array-root/index-NUL and clean lexical controls in CLIENT_ACCEPTED and RULE_INIT with event-valid logging wrappers. Discover coverage from actual event observations. RULE_INIT execution count does not establish TMM broadcast. Record CMP before and after; initialization globals must not be confused with per-event local cells.

The following complete wrapper template is for CLIENT_ACCEPTED. Replace the three ASCII tokens with the run ID, case ID and exact hexadecimal bytes of that case's full `.tcl` payload. Do not add HTTP commands to this event:

```tcl
when CLIENT_ACCEPTED {
    set payload [binary format H* PAYLOAD_HEX]
    set rc [catch {eval $payload} result]
    binary scan $result H* result_hex
    set tmm_group [TMM::cmp_group]
    set tmm_unit [TMM::cmp_unit]
    log local0. "R2286NAMES|RUN_TOKEN|CLIENT_ACCEPTED|CASE_TOKEN|tmm=$tmm_group:$tmm_unit|rc=$rc|result_hex=$result_hex"
}
```

For RULE_INIT, change only the event name and recorded context label, then hash that distinct full wrapper. If TMM identity commands are unavailable in an event, record their own catch status and emit the remaining observation; retain this as a separately hashed wrapper. For HTTP_REQUEST use the full existing generated wrapper, which supplies response correlation and observed TMM headers.

## Clean lexical read discriminator

This additional complete ASCII/LF payload separates variable substitution from a second variable-name lookup. Replace `RUN_TOKEN` with the fresh run ID. The literal backslash before `$` prevents substitution while constructing the script. Escaped braces are script data. Runtime suffix bytes may be non-ASCII or NUL although the payload's source bytes are entirely ASCII.

```tcl
set prefix __tcl_lsp_2286_RUN_TOKEN_lexclean_
set rows {}
foreach pair [list [list format_pre [format %c 233]] [list format_comb [format %c 769]] [list bytes_pre [binary format H* c3a9]] [list bytes_comb [binary format H* cc81]] [list nul [binary format H* 0042]]] {
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

Retain both returned values and exact generated script bytes. A value beginning `SHORT_PREFIX` followed by suffix bytes distinguishes a shortened lexical token from the complete dynamically written name. Failed dynamic writes and full-name reads remain separately recorded.

## Qualified variable and callback boundaries

Add separately hashed controls using a dynamically created ASCII namespace and two scalar/array roots `::namespace::A` and `::namespace::A` followed by runtime `0042`. Repeat write-order and unset-order discriminators, upvar and trace callbacks. These controls ask whether qualified variable roots share the counted-key behavior measured for unqualified event-local names; they do not imply NUL support in namespace components.

Record full original root/index/operation bytes for traces and aliases. Keep the selected cell's identity separate from the alias spelling reported to a callback. Use exact mutation and unset discriminators. Record namespace current/which and command-name inventories only through independently supported commands; command display alone is insufficient to prove slot identity.

## Counted-key input boundaries

Run additional fresh controls with prefixes unrelated to the earlier fixture
names. The compiler needs a recipe for an operation's input domain, so report
whether counted identity survives different ASCII prefixes, lengths, NUL
positions and multiple NUL bytes. Keep empty names, empty indices, colons,
parentheses and qualified roots as distinct cases. Repeat the write/unset-order,
upvar and trace discriminators for accepted inputs. Do not infer a grammar from
a display string or combine rejected inputs with successful ones.

The following complete HTTP event payload measures ordinary scalar/combined
input acceptance and identity. Its source is ASCII with LF line endings. Every
NUL comes from `format %c 0` at runtime. The two values `PLAIN` and `COUNTED`
must remain distinguishable in the raw
records. Use a fresh owned virtual server with SNAT automap; clients and servers
may both run on dev.bragi0.com using any suitable protocol.

```tcl
when HTTP_REQUEST {
    set nul [format %c 0]
    set rows {}
    foreach spec {
        {short x}
        {long __resolution2286_context_invariance_long_ascii_prefix_0123456789}
        {different relocated_2286_key}
    } {
        set label [lindex $spec 0]
        set plain [lindex $spec 1]
        set name $plain
        append name $nul B $nul C
        catch {unset $plain}
        catch {unset $name}
        set plain_rc [catch {set $plain PLAIN} plain_result]
        set name_rc [catch {set $name COUNTED} name_result]
        foreach input [list $plain $name] {
            binary scan $input H* input_hex
            set rc [catch {set $input} result]
            binary scan $result H* result_hex
            lappend rows [list $label name_hex $input_hex write_plain $plain_rc write_counted $name_rc read $rc result_hex $result_hex]
        }
        catch {unset $name}
        set rc [catch {set $plain} result]
        binary scan $result H* result_hex
        lappend rows [list $label after_counted_unset $rc result_hex $result_hex]
        catch {unset $plain}
    }
    set inputs [list {} {()} {(k)} {A()} {A(k)} {A(k(l))} {A(k)tail} {:} {A:B} {A::B}]
    foreach input $inputs {
        binary scan $input H* input_hex
        set rc [catch {set $input GRAMMAR_VALUE} result]
        binary scan $result H* result_hex
        lappend rows [list grammar name_hex $input_hex write $rc result_hex $result_hex]
        set read_rc [catch {set $input} read_result]
        binary scan $read_result H* read_hex
        lappend rows [list grammar name_hex $input_hex read $read_rc result_hex $read_hex]
        catch {unset $input}
    }
    log local0.notice [list RESOLUTION2286_COUNTED_INPUT_BOUNDARIES tmm [TMM::cmp_unit] rows $rows]
    HTTP::respond 200 content OK
}
```

Also create separately hashed array-root and index controls for those relocated
prefixes and multiple-NUL positions, including reversed writes and unsets. The
payload above does not establish separate-root/index API parsing, qualified
namespace lookup, callback names or command/formal naming. Record exact original
bytes, produced bytes and the actual reached context for every added control.

## Result contract

Write `CONTEXT_NAMING_RESULTS.md` on `work` beside this document. Put exact BIG-IP version/build and reached context/TMM coverage first. Include a matrix of case, context, source hashes, load result, execution catch/result bytes, trace/root/index bytes and conclusion. Link raw evidence, manifests and cleanup/object-absence proof. State unsupported and untested boundaries explicitly, including namespace-component NUL, command-name NUL, suspended entered procedures, cross-blade/HA behavior and reload persistence unless separately measured with an authorized supported mechanism.

The report must support a context- and purpose-specific naming interface. Do not infer a C Tcl/Jim string representation, native object ABI, compiler hook or another context's naming behavior from equal outputs.
