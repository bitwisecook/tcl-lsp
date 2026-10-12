# BIG-IP lexical byte-class execution request

Execution branch: **work**. Current reference appliance: **BIG-IP 21.1.0.1,
build 0.0.26, Point Release 1**. Verify and record the actual version for this
new run. Status: **requested; no new appliance result is claimed here**.
Return the actual execution report as `LEXICAL_BYTE_CLASS_RESULTS.md`. No peer
response exists yet. The optional
[results template](LEXICAL_BYTE_CLASS_RESULTS_TEMPLATE.md) is a preparation form,
not an executed report.

The current [context report](CONTEXT_NAMING_RESULTS.md#lexical-name-boundary)
measured TMM stopping before `e9`, `c3` and `cc`. Non-TMM tmsh/iApp/iCall consumed
`e9`, and consumed only the first byte of binary `c3a9`/`cc81`, stopping before
`a9`/`81`. This does not establish all 256 byte classes or a Unicode character
class. It also does not establish a shared parser across those contexts.

## Exact question and boundaries

Measure unbraced lexical variable-name consumption, with braced substitution
as a separate control, for every binary-produced singleton from `00` through
`ff`. Repeat five explicitly labelled producers: `format %c 233`, `format %c
769`, and binary `c3a9`, `e9`, `cc81`. Do not equate `%c` output with a chosen
UTF-8 sequence. The same family is evaluated using fresh `append_source`
String construction and a separately constructed `binary_source` object.
There are 261 inputs x two source producers x two forms = **1,044 rows**.

Only the variable/eval/read-trace boundary is tested. No row supplies command
lookup, NameTable identity, compiler selection, cache ownership, object lifetime,
ctype/locale authority or successful side-effect advice. The payload does not
inspect unrelated appliance configuration or invoke unrelated lab commands.

For each row, the code seeds a unique ASCII prefix with `SHORT` and the complete
dynamic name with `FULL`. Binary compound cases additionally seed the one-byte
`c3` or `cc` prefix with `PREFIX_1`. It records setup and trace-registration
failures independently of eval. The trace callback is just native `lappend` of
the three supplied read arguments into the owned observation variable: no
context-specific TMM function is used inside the callback. Delimiter bytes may
cause syntax errors, extra list fields, or a fixed `Q` command/name miss; keep
those exact outcomes. They are not automatically letter-class decisions.

The tested byte is appended or inserted into a binary constructor directly.
It is never backslash-escaped, quoted, cleaned, normalized or filtered before
eval. Braced and unbraced source are separately constructed. `eval` receives
that original constructed source object. `source_scan` and `result_scan`
record the actual `binary scan H*` status/hex **after** eval, so inspection does
not prime the selected original source beforehand. `constructor_hex` records
the binary-source input independently. An H* snapshot is an observed conversion,
not a claim about internal UTF-8 storage. The separately caught `encoding
convertto utf-8` diagnostic runs after eval and cannot replace a refused H*
conversion or provide TMM parser authority. If source construction refuses,
`eval_code` is `NOT_REACHED`, not an invented Tcl error or a rejected byte class.

## Source integrity and execution plan

1. Use the current `work` commit and record its exact hash, the product/version,
   and the context-specific Tcl/package version observations. Keep the full
   generated source as ASCII/LF. Extract code blocks byte-for-byte, retaining
   backslashes and physical line breaks; do not paste through a text converter.
   Hash the extracted payload, complete iRule, each CLI/iApp/iCall wrapper and
   the archive before transfer. Verify SHA-256 and byte dumps after transfer
   and before loading. Record any loader rewrite or refusal rather than repairing
   the tested source silently. Hash separately any capture-only repair.
2. Use real clients and a real backend service on **dev.bragi0.com**. An existing
   HTTP server is sufficient; first prove its loopback and reachable-interface
   response, then a pool-backed VIP with TCP+HTTP and **SNAT automap**. Keep the
   complete raw request/response and server's received peer/marker. A synthetic
   terminal iRule response is not the backend baseline. Record the owned VIP,
   pool/member, profiles, CMP setting and translation used for this run. The
   existing report's addresses/ports are not reservations for reuse.
3. Capture `/var/log/ltm` continuously before load through execution and cleanup.
   Load the full iRule below without changing its byte producers. Observe actual
   RULE_INIT identities from one initialization; a configured CPU count or a
   creation success proves no broadcast. Before attaching this heavy rule, use
   a separately recorded lightweight identity baseline to choose one connection
   tuple per actual TMM. For the reference four-unit roster, run at most four
   heavy connections, sequentially, targeting one complete CLIENT_ACCEPTED and
   HTTP_REQUEST capture per reached unit. Do not run a 32-connection sweep or
   reload the rule to fill coverage. A tuple may move to another TMM: preserve
   the actual identity, stop at the finite connection budget, and leave missing
   units uncovered. Each event still executes all 1,044 rows unchanged.
   Record a same-run normal backend pass-through response after attaching the
   rule (a path other than the probe path), including SNAT/backend receipt. Use
   a selected connection for this request and then the probe request when its
   HTTP connection remains open. If it closes, retain the incomplete coverage
   rather than starting an unbounded retry stream.
4. The full HTTP response body is authoritative for HTTP_REQUEST. Save it as raw
   bytes, with headers/status/session, before decoding its export envelope.
   CLIENT_ACCEPTED and RULE_INIT use 384-hex-character chunks. Reassemble only
   a complete `seq=0..total-1` set for one run/context/TMM/session; preserve
   duplicates, missing/conflicting chunks and complete original LTM lines.
   Never accept a truncated one-line syslog result as a full transcript.
   Keep a finite batch inventory for the initialization and each chosen client:
   source hash, run/context/TMM/session, observed first/last sequence, declared
   chunk count, received count, exact reassembled bytes and SHA-256, decoded row
   count, and all 1,044 unique case/source-kind/form keys. Verify completeness
   before starting the next client. The original rule hex-encodes its export
   envelope again for logging; native-sized transcripts can require thousands
   of chunks per event even though each line fits the syslog extent. If capture
   drops or truncates chunks, retain the rows as unknown. A separately hashed
   capture-only batching wrapper may suppress duplicate exports or split output
   into bounded batches, with complete begin/end/count/checksum records. Retain
   its full source and verify that the payload, runtime producers, constructed
   source and eval words are byte-identical to this request. Do not change the
   tested bytes, drop rows or classify a partial event transcript.
5. Execute the non-TMM script below as an actual tmsh CLI script, an actual iApp
   **implementation action**, and a delivered/triggered iCall script. Change
   only the context label in each independently hashed wrapper. Save stdout,
   stderr, outer status and all row bytes. Object creation alone is insufficient.
   APL is a separate optional row only if actual presentation evaluation and
   returned output can be proved; otherwise mark APL not executed/unsupported.
   Do not substitute standalone Tcl, iApp implementation, or a successful loader
   for APL. Remove only this run's owned objects/listeners after evidence capture.

## Common payload (ASCII/LF)

This is the identical payload inside each full event wrapper and the non-TMM
script. Its exact trailing LF is part of the hash. The native portability smoke
ran this payload on C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim;
each emitted 1,044 rows with successful transcript export and no outer stderr.
That is a portability check, not appliance context evidence. Its record is
`/workspace/.proofs/2286-lexical-byte-class-request/portable-smoke-manifest.json`.

This Tcl8.4-compatible trace syntax is available in the native C8 smoke engines.
C9 and Jim reject legacy trace registration; those per-row refusals provide no
read-trace evidence and say nothing about BIG-IP trace availability. C9 also
refuses the `%c 769` binary-source construction while its append-built source
still reaches eval. Keep these source-producer and diagnostic boundaries
independent in the requested appliance results.

Payload SHA-256: `af1d6366e57927bc72cd756bf8804e1a1713de958561f30258876603bc120ae9`.

```tcl
set r2286_lbc_rows {}
for {set r2286_lbc_i 0} {$r2286_lbc_i < 261} {incr r2286_lbc_i} {
    foreach r2286_lbc_source_kind {append_source binary_source} {
        foreach r2286_lbc_form {unbraced braced} {
            if {$r2286_lbc_i < 256} {
                set r2286_lbc_factory [list binary_byte $r2286_lbc_i]
                set r2286_lbc_payload [binary format H* [format %02x $r2286_lbc_i]]
            } else {
                switch -- $r2286_lbc_i {
                    256 {
                        set r2286_lbc_factory [list format_c 233]
                        set r2286_lbc_payload [format %c 233]
                    }
                    257 {
                        set r2286_lbc_factory [list format_c 769]
                        set r2286_lbc_payload [format %c 769]
                    }
                    258 {
                        set r2286_lbc_factory [list binary_hex c3a9]
                        set r2286_lbc_payload [binary format H* c3a9]
                    }
                    259 {
                        set r2286_lbc_factory [list binary_hex e9]
                        set r2286_lbc_payload [binary format H* e9]
                    }
                    260 {
                        set r2286_lbc_factory [list binary_hex cc81]
                        set r2286_lbc_payload [binary format H* cc81]
                    }
                }
            }
            set r2286_lbc_stem "r2286_lbc_${r2286_lbc_i}_${r2286_lbc_source_kind}_${r2286_lbc_form}_v"
            set r2286_lbc_full $r2286_lbc_stem
            append r2286_lbc_full $r2286_lbc_payload Q
            set r2286_lbc_targets [list [list $r2286_lbc_stem SHORT]]
            if {$r2286_lbc_i == 258} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* c3]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            if {$r2286_lbc_i == 260} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* cc]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            lappend r2286_lbc_targets [list $r2286_lbc_full FULL]
            set r2286_lbc_setup {}
            set r2286_lbc_add {}
            set r2286_lbc_attached {}
            set r2286_lbc_callback [list lappend ::r2286_lbc_reads]
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_marker [lindex $r2286_lbc_target 1]
                set r2286_lbc_rc [catch {set $r2286_lbc_name $r2286_lbc_marker} r2286_lbc_value]
                lappend r2286_lbc_setup [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                set r2286_lbc_rc [catch {trace variable $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_add [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                if {$r2286_lbc_rc == 0} {
                    lappend r2286_lbc_attached $r2286_lbc_name
                }
            }
            set r2286_lbc_source {list $}
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \{}
            append r2286_lbc_source $r2286_lbc_full
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \}}
            set r2286_lbc_build_rc 0
            set r2286_lbc_build_result {}
            set r2286_lbc_constructor_hex {}
            if {$r2286_lbc_source_kind == "binary_source"} {
                set r2286_lbc_build_rc [catch {
                    if {$r2286_lbc_i < 256 || $r2286_lbc_i >= 258} {
                        set r2286_lbc_lead {list $}
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_lead \{}
                        binary scan $r2286_lbc_lead H* r2286_lbc_lead_hex
                        binary scan $r2286_lbc_stem H* r2286_lbc_stem_hex
                        if {$r2286_lbc_i < 256} {
                            set r2286_lbc_byte_hex [format %02x $r2286_lbc_i]
                        } else {
                            switch -- $r2286_lbc_i {
                                258 {set r2286_lbc_byte_hex c3a9}
                                259 {set r2286_lbc_byte_hex e9}
                                260 {set r2286_lbc_byte_hex cc81}
                            }
                        }
                        set r2286_lbc_constructor_hex "$r2286_lbc_lead_hex$r2286_lbc_stem_hex${r2286_lbc_byte_hex}51"
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_constructor_hex 7d}
                    } else {
                        binary scan $r2286_lbc_source H* r2286_lbc_constructor_hex
                    }
                    set r2286_lbc_source [binary format H* $r2286_lbc_constructor_hex]
                } r2286_lbc_build_result]
            }
            set ::r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $::r2286_lbc_reads
            set r2286_lbc_source_hex {}
            set r2286_lbc_source_scan_rc [catch {binary scan $r2286_lbc_source H* r2286_lbc_source_hex} r2286_lbc_source_scan_result]
            set r2286_lbc_result_hex {}
            set r2286_lbc_result_scan_rc [catch {binary scan $r2286_lbc_eval_result H* r2286_lbc_result_hex} r2286_lbc_result_scan_result]
            set r2286_lbc_payload_hex {}
            set r2286_lbc_payload_scan_rc [catch {binary scan $r2286_lbc_payload H* r2286_lbc_payload_hex} r2286_lbc_payload_scan_result]
            set r2286_lbc_utf8_hex {}
            set r2286_lbc_encoding_rc [catch {
                set r2286_lbc_utf8 [encoding convertto utf-8 $r2286_lbc_source]
                binary scan $r2286_lbc_utf8 H* r2286_lbc_utf8_hex
            } r2286_lbc_encoding_result]
            set r2286_lbc_remove {}
            foreach r2286_lbc_name $r2286_lbc_attached {
                set r2286_lbc_rc [catch {trace vdelete $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_remove [list $r2286_lbc_rc $r2286_lbc_value]
            }
            set r2286_lbc_cleanup {}
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_rc [catch {unset $r2286_lbc_name} r2286_lbc_value]
                lappend r2286_lbc_cleanup [list $r2286_lbc_rc $r2286_lbc_value]
            }
            lappend r2286_lbc_rows [list \
                case $r2286_lbc_i factory $r2286_lbc_factory \
                source_kind $r2286_lbc_source_kind form $r2286_lbc_form constructor_hex $r2286_lbc_constructor_hex \
                payload_scan [list $r2286_lbc_payload_scan_rc $r2286_lbc_payload_hex $r2286_lbc_payload_scan_result] \
                source_build [list $r2286_lbc_build_rc $r2286_lbc_build_result] \
                source_scan [list $r2286_lbc_source_scan_rc $r2286_lbc_source_hex $r2286_lbc_source_scan_result] \
                eval_code $r2286_lbc_eval_rc \
                result_scan [list $r2286_lbc_result_scan_rc $r2286_lbc_result_hex $r2286_lbc_result_scan_result] \
                reads $r2286_lbc_reads setup $r2286_lbc_setup trace_add $r2286_lbc_add \
                utf8_diagnostic [list $r2286_lbc_encoding_rc $r2286_lbc_utf8_hex $r2286_lbc_encoding_result] \
                trace_remove $r2286_lbc_remove cleanup $r2286_lbc_cleanup]
        }
    }
}
catch {unset ::r2286_lbc_reads}
```

## Full iRule (ASCII/LF)

The following is the complete rule body, including every repeated event payload,
backslash escape, split line and runtime non-ASCII producer. There is no opaque
payload token to replace. `RULE_INIT` and `CLIENT_ACCEPTED` contain no HTTP
commands. HTTP_REQUEST returns a full envelope only at
`/r2286-lexical-byte-class`; other HTTP paths continue to the real backend.
Each event independently catches identity queries and records their status.
The fixed run label is `r2286lex1`; if changing it, hash and retain the entire
changed rule before transfer. Do not modify byte construction or eval source.

Full-rule SHA-256: `49b3c8dcd43221031288595afc53f9cc4642fb714e5a57b123b15d3613d91a40`.

```tcl
when RULE_INIT {
set r2286_lbc_context RULE_INIT
set r2286_lbc_group_rc [catch {TMM::cmp_group} r2286_lbc_group]
set r2286_lbc_unit_rc [catch {TMM::cmp_unit} r2286_lbc_unit]
set r2286_lbc_identity [list $r2286_lbc_group_rc $r2286_lbc_group $r2286_lbc_unit_rc $r2286_lbc_unit]
set r2286_lbc_session init
set r2286_lbc_rows {}
for {set r2286_lbc_i 0} {$r2286_lbc_i < 261} {incr r2286_lbc_i} {
    foreach r2286_lbc_source_kind {append_source binary_source} {
        foreach r2286_lbc_form {unbraced braced} {
            if {$r2286_lbc_i < 256} {
                set r2286_lbc_factory [list binary_byte $r2286_lbc_i]
                set r2286_lbc_payload [binary format H* [format %02x $r2286_lbc_i]]
            } else {
                switch -- $r2286_lbc_i {
                    256 {
                        set r2286_lbc_factory [list format_c 233]
                        set r2286_lbc_payload [format %c 233]
                    }
                    257 {
                        set r2286_lbc_factory [list format_c 769]
                        set r2286_lbc_payload [format %c 769]
                    }
                    258 {
                        set r2286_lbc_factory [list binary_hex c3a9]
                        set r2286_lbc_payload [binary format H* c3a9]
                    }
                    259 {
                        set r2286_lbc_factory [list binary_hex e9]
                        set r2286_lbc_payload [binary format H* e9]
                    }
                    260 {
                        set r2286_lbc_factory [list binary_hex cc81]
                        set r2286_lbc_payload [binary format H* cc81]
                    }
                }
            }
            set r2286_lbc_stem "r2286_lbc_${r2286_lbc_i}_${r2286_lbc_source_kind}_${r2286_lbc_form}_v"
            set r2286_lbc_full $r2286_lbc_stem
            append r2286_lbc_full $r2286_lbc_payload Q
            set r2286_lbc_targets [list [list $r2286_lbc_stem SHORT]]
            if {$r2286_lbc_i == 258} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* c3]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            if {$r2286_lbc_i == 260} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* cc]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            lappend r2286_lbc_targets [list $r2286_lbc_full FULL]
            set r2286_lbc_setup {}
            set r2286_lbc_add {}
            set r2286_lbc_attached {}
            set r2286_lbc_callback [list lappend ::r2286_lbc_reads]
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_marker [lindex $r2286_lbc_target 1]
                set r2286_lbc_rc [catch {set $r2286_lbc_name $r2286_lbc_marker} r2286_lbc_value]
                lappend r2286_lbc_setup [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                set r2286_lbc_rc [catch {trace variable $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_add [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                if {$r2286_lbc_rc == 0} {
                    lappend r2286_lbc_attached $r2286_lbc_name
                }
            }
            set r2286_lbc_source {list $}
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \{}
            append r2286_lbc_source $r2286_lbc_full
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \}}
            set r2286_lbc_build_rc 0
            set r2286_lbc_build_result {}
            set r2286_lbc_constructor_hex {}
            if {$r2286_lbc_source_kind == "binary_source"} {
                set r2286_lbc_build_rc [catch {
                    if {$r2286_lbc_i < 256 || $r2286_lbc_i >= 258} {
                        set r2286_lbc_lead {list $}
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_lead \{}
                        binary scan $r2286_lbc_lead H* r2286_lbc_lead_hex
                        binary scan $r2286_lbc_stem H* r2286_lbc_stem_hex
                        if {$r2286_lbc_i < 256} {
                            set r2286_lbc_byte_hex [format %02x $r2286_lbc_i]
                        } else {
                            switch -- $r2286_lbc_i {
                                258 {set r2286_lbc_byte_hex c3a9}
                                259 {set r2286_lbc_byte_hex e9}
                                260 {set r2286_lbc_byte_hex cc81}
                            }
                        }
                        set r2286_lbc_constructor_hex "$r2286_lbc_lead_hex$r2286_lbc_stem_hex${r2286_lbc_byte_hex}51"
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_constructor_hex 7d}
                    } else {
                        binary scan $r2286_lbc_source H* r2286_lbc_constructor_hex
                    }
                    set r2286_lbc_source [binary format H* $r2286_lbc_constructor_hex]
                } r2286_lbc_build_result]
            }
            set ::r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $::r2286_lbc_reads
            set r2286_lbc_source_hex {}
            set r2286_lbc_source_scan_rc [catch {binary scan $r2286_lbc_source H* r2286_lbc_source_hex} r2286_lbc_source_scan_result]
            set r2286_lbc_result_hex {}
            set r2286_lbc_result_scan_rc [catch {binary scan $r2286_lbc_eval_result H* r2286_lbc_result_hex} r2286_lbc_result_scan_result]
            set r2286_lbc_payload_hex {}
            set r2286_lbc_payload_scan_rc [catch {binary scan $r2286_lbc_payload H* r2286_lbc_payload_hex} r2286_lbc_payload_scan_result]
            set r2286_lbc_utf8_hex {}
            set r2286_lbc_encoding_rc [catch {
                set r2286_lbc_utf8 [encoding convertto utf-8 $r2286_lbc_source]
                binary scan $r2286_lbc_utf8 H* r2286_lbc_utf8_hex
            } r2286_lbc_encoding_result]
            set r2286_lbc_remove {}
            foreach r2286_lbc_name $r2286_lbc_attached {
                set r2286_lbc_rc [catch {trace vdelete $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_remove [list $r2286_lbc_rc $r2286_lbc_value]
            }
            set r2286_lbc_cleanup {}
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_rc [catch {unset $r2286_lbc_name} r2286_lbc_value]
                lappend r2286_lbc_cleanup [list $r2286_lbc_rc $r2286_lbc_value]
            }
            lappend r2286_lbc_rows [list \
                case $r2286_lbc_i factory $r2286_lbc_factory \
                source_kind $r2286_lbc_source_kind form $r2286_lbc_form constructor_hex $r2286_lbc_constructor_hex \
                payload_scan [list $r2286_lbc_payload_scan_rc $r2286_lbc_payload_hex $r2286_lbc_payload_scan_result] \
                source_build [list $r2286_lbc_build_rc $r2286_lbc_build_result] \
                source_scan [list $r2286_lbc_source_scan_rc $r2286_lbc_source_hex $r2286_lbc_source_scan_result] \
                eval_code $r2286_lbc_eval_rc \
                result_scan [list $r2286_lbc_result_scan_rc $r2286_lbc_result_hex $r2286_lbc_result_scan_result] \
                reads $r2286_lbc_reads setup $r2286_lbc_setup trace_add $r2286_lbc_add \
                utf8_diagnostic [list $r2286_lbc_encoding_rc $r2286_lbc_utf8_hex $r2286_lbc_encoding_result] \
                trace_remove $r2286_lbc_remove cleanup $r2286_lbc_cleanup]
        }
    }
}
catch {unset ::r2286_lbc_reads}
set r2286_lbc_transcript [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
set r2286_lbc_transcript_hex {}
set r2286_lbc_export_rc [catch {binary scan $r2286_lbc_transcript H* r2286_lbc_transcript_hex} r2286_lbc_export_result]
set r2286_lbc_export [list LBC1 $r2286_lbc_export_rc $r2286_lbc_export_result $r2286_lbc_transcript_hex]
set r2286_lbc_log_hex {}
set r2286_lbc_log_rc [catch {binary scan $r2286_lbc_export H* r2286_lbc_log_hex} r2286_lbc_log_result]
if {$r2286_lbc_log_rc != 0} {
    log local0. "R2286LBC|run=r2286lex1|ctx=$r2286_lbc_context|export_refusal=$r2286_lbc_log_rc"
} else {
    set r2286_lbc_total [expr {([string length $r2286_lbc_log_hex] + 383) / 384}]
    for {set r2286_lbc_k 0} {$r2286_lbc_k < $r2286_lbc_total} {incr r2286_lbc_k} {
        set r2286_lbc_start [expr {$r2286_lbc_k * 384}]
        set r2286_lbc_chunk [string range $r2286_lbc_log_hex $r2286_lbc_start [expr {$r2286_lbc_start + 383}]]
        log local0. "R2286LBC|run=r2286lex1|ctx=$r2286_lbc_context|group_rc=$r2286_lbc_group_rc|group=$r2286_lbc_group|unit_rc=$r2286_lbc_unit_rc|unit=$r2286_lbc_unit|session=$r2286_lbc_session|seq=$r2286_lbc_k/$r2286_lbc_total|hex=$r2286_lbc_chunk"
    }
}
}

when CLIENT_ACCEPTED {
set r2286_lbc_context CLIENT_ACCEPTED
set r2286_lbc_group_rc [catch {TMM::cmp_group} r2286_lbc_group]
set r2286_lbc_unit_rc [catch {TMM::cmp_unit} r2286_lbc_unit]
set r2286_lbc_identity [list $r2286_lbc_group_rc $r2286_lbc_group $r2286_lbc_unit_rc $r2286_lbc_unit]
set r2286_lbc_session "[IP::client_addr]:[TCP::client_port]"
set r2286_lbc_rows {}
for {set r2286_lbc_i 0} {$r2286_lbc_i < 261} {incr r2286_lbc_i} {
    foreach r2286_lbc_source_kind {append_source binary_source} {
        foreach r2286_lbc_form {unbraced braced} {
            if {$r2286_lbc_i < 256} {
                set r2286_lbc_factory [list binary_byte $r2286_lbc_i]
                set r2286_lbc_payload [binary format H* [format %02x $r2286_lbc_i]]
            } else {
                switch -- $r2286_lbc_i {
                    256 {
                        set r2286_lbc_factory [list format_c 233]
                        set r2286_lbc_payload [format %c 233]
                    }
                    257 {
                        set r2286_lbc_factory [list format_c 769]
                        set r2286_lbc_payload [format %c 769]
                    }
                    258 {
                        set r2286_lbc_factory [list binary_hex c3a9]
                        set r2286_lbc_payload [binary format H* c3a9]
                    }
                    259 {
                        set r2286_lbc_factory [list binary_hex e9]
                        set r2286_lbc_payload [binary format H* e9]
                    }
                    260 {
                        set r2286_lbc_factory [list binary_hex cc81]
                        set r2286_lbc_payload [binary format H* cc81]
                    }
                }
            }
            set r2286_lbc_stem "r2286_lbc_${r2286_lbc_i}_${r2286_lbc_source_kind}_${r2286_lbc_form}_v"
            set r2286_lbc_full $r2286_lbc_stem
            append r2286_lbc_full $r2286_lbc_payload Q
            set r2286_lbc_targets [list [list $r2286_lbc_stem SHORT]]
            if {$r2286_lbc_i == 258} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* c3]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            if {$r2286_lbc_i == 260} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* cc]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            lappend r2286_lbc_targets [list $r2286_lbc_full FULL]
            set r2286_lbc_setup {}
            set r2286_lbc_add {}
            set r2286_lbc_attached {}
            set r2286_lbc_callback [list lappend ::r2286_lbc_reads]
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_marker [lindex $r2286_lbc_target 1]
                set r2286_lbc_rc [catch {set $r2286_lbc_name $r2286_lbc_marker} r2286_lbc_value]
                lappend r2286_lbc_setup [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                set r2286_lbc_rc [catch {trace variable $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_add [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                if {$r2286_lbc_rc == 0} {
                    lappend r2286_lbc_attached $r2286_lbc_name
                }
            }
            set r2286_lbc_source {list $}
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \{}
            append r2286_lbc_source $r2286_lbc_full
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \}}
            set r2286_lbc_build_rc 0
            set r2286_lbc_build_result {}
            set r2286_lbc_constructor_hex {}
            if {$r2286_lbc_source_kind == "binary_source"} {
                set r2286_lbc_build_rc [catch {
                    if {$r2286_lbc_i < 256 || $r2286_lbc_i >= 258} {
                        set r2286_lbc_lead {list $}
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_lead \{}
                        binary scan $r2286_lbc_lead H* r2286_lbc_lead_hex
                        binary scan $r2286_lbc_stem H* r2286_lbc_stem_hex
                        if {$r2286_lbc_i < 256} {
                            set r2286_lbc_byte_hex [format %02x $r2286_lbc_i]
                        } else {
                            switch -- $r2286_lbc_i {
                                258 {set r2286_lbc_byte_hex c3a9}
                                259 {set r2286_lbc_byte_hex e9}
                                260 {set r2286_lbc_byte_hex cc81}
                            }
                        }
                        set r2286_lbc_constructor_hex "$r2286_lbc_lead_hex$r2286_lbc_stem_hex${r2286_lbc_byte_hex}51"
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_constructor_hex 7d}
                    } else {
                        binary scan $r2286_lbc_source H* r2286_lbc_constructor_hex
                    }
                    set r2286_lbc_source [binary format H* $r2286_lbc_constructor_hex]
                } r2286_lbc_build_result]
            }
            set ::r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $::r2286_lbc_reads
            set r2286_lbc_source_hex {}
            set r2286_lbc_source_scan_rc [catch {binary scan $r2286_lbc_source H* r2286_lbc_source_hex} r2286_lbc_source_scan_result]
            set r2286_lbc_result_hex {}
            set r2286_lbc_result_scan_rc [catch {binary scan $r2286_lbc_eval_result H* r2286_lbc_result_hex} r2286_lbc_result_scan_result]
            set r2286_lbc_payload_hex {}
            set r2286_lbc_payload_scan_rc [catch {binary scan $r2286_lbc_payload H* r2286_lbc_payload_hex} r2286_lbc_payload_scan_result]
            set r2286_lbc_utf8_hex {}
            set r2286_lbc_encoding_rc [catch {
                set r2286_lbc_utf8 [encoding convertto utf-8 $r2286_lbc_source]
                binary scan $r2286_lbc_utf8 H* r2286_lbc_utf8_hex
            } r2286_lbc_encoding_result]
            set r2286_lbc_remove {}
            foreach r2286_lbc_name $r2286_lbc_attached {
                set r2286_lbc_rc [catch {trace vdelete $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_remove [list $r2286_lbc_rc $r2286_lbc_value]
            }
            set r2286_lbc_cleanup {}
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_rc [catch {unset $r2286_lbc_name} r2286_lbc_value]
                lappend r2286_lbc_cleanup [list $r2286_lbc_rc $r2286_lbc_value]
            }
            lappend r2286_lbc_rows [list \
                case $r2286_lbc_i factory $r2286_lbc_factory \
                source_kind $r2286_lbc_source_kind form $r2286_lbc_form constructor_hex $r2286_lbc_constructor_hex \
                payload_scan [list $r2286_lbc_payload_scan_rc $r2286_lbc_payload_hex $r2286_lbc_payload_scan_result] \
                source_build [list $r2286_lbc_build_rc $r2286_lbc_build_result] \
                source_scan [list $r2286_lbc_source_scan_rc $r2286_lbc_source_hex $r2286_lbc_source_scan_result] \
                eval_code $r2286_lbc_eval_rc \
                result_scan [list $r2286_lbc_result_scan_rc $r2286_lbc_result_hex $r2286_lbc_result_scan_result] \
                reads $r2286_lbc_reads setup $r2286_lbc_setup trace_add $r2286_lbc_add \
                utf8_diagnostic [list $r2286_lbc_encoding_rc $r2286_lbc_utf8_hex $r2286_lbc_encoding_result] \
                trace_remove $r2286_lbc_remove cleanup $r2286_lbc_cleanup]
        }
    }
}
catch {unset ::r2286_lbc_reads}
set r2286_lbc_transcript [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
set r2286_lbc_transcript_hex {}
set r2286_lbc_export_rc [catch {binary scan $r2286_lbc_transcript H* r2286_lbc_transcript_hex} r2286_lbc_export_result]
set r2286_lbc_export [list LBC1 $r2286_lbc_export_rc $r2286_lbc_export_result $r2286_lbc_transcript_hex]
set r2286_lbc_log_hex {}
set r2286_lbc_log_rc [catch {binary scan $r2286_lbc_export H* r2286_lbc_log_hex} r2286_lbc_log_result]
if {$r2286_lbc_log_rc != 0} {
    log local0. "R2286LBC|run=r2286lex1|ctx=$r2286_lbc_context|export_refusal=$r2286_lbc_log_rc"
} else {
    set r2286_lbc_total [expr {([string length $r2286_lbc_log_hex] + 383) / 384}]
    for {set r2286_lbc_k 0} {$r2286_lbc_k < $r2286_lbc_total} {incr r2286_lbc_k} {
        set r2286_lbc_start [expr {$r2286_lbc_k * 384}]
        set r2286_lbc_chunk [string range $r2286_lbc_log_hex $r2286_lbc_start [expr {$r2286_lbc_start + 383}]]
        log local0. "R2286LBC|run=r2286lex1|ctx=$r2286_lbc_context|group_rc=$r2286_lbc_group_rc|group=$r2286_lbc_group|unit_rc=$r2286_lbc_unit_rc|unit=$r2286_lbc_unit|session=$r2286_lbc_session|seq=$r2286_lbc_k/$r2286_lbc_total|hex=$r2286_lbc_chunk"
    }
}
}

when HTTP_REQUEST {
if {[HTTP::path] != "/r2286-lexical-byte-class"} {return}
set r2286_lbc_context HTTP_REQUEST
set r2286_lbc_group_rc [catch {TMM::cmp_group} r2286_lbc_group]
set r2286_lbc_unit_rc [catch {TMM::cmp_unit} r2286_lbc_unit]
set r2286_lbc_identity [list $r2286_lbc_group_rc $r2286_lbc_group $r2286_lbc_unit_rc $r2286_lbc_unit]
set r2286_lbc_session "[IP::client_addr]:[TCP::client_port]"
set r2286_lbc_rows {}
for {set r2286_lbc_i 0} {$r2286_lbc_i < 261} {incr r2286_lbc_i} {
    foreach r2286_lbc_source_kind {append_source binary_source} {
        foreach r2286_lbc_form {unbraced braced} {
            if {$r2286_lbc_i < 256} {
                set r2286_lbc_factory [list binary_byte $r2286_lbc_i]
                set r2286_lbc_payload [binary format H* [format %02x $r2286_lbc_i]]
            } else {
                switch -- $r2286_lbc_i {
                    256 {
                        set r2286_lbc_factory [list format_c 233]
                        set r2286_lbc_payload [format %c 233]
                    }
                    257 {
                        set r2286_lbc_factory [list format_c 769]
                        set r2286_lbc_payload [format %c 769]
                    }
                    258 {
                        set r2286_lbc_factory [list binary_hex c3a9]
                        set r2286_lbc_payload [binary format H* c3a9]
                    }
                    259 {
                        set r2286_lbc_factory [list binary_hex e9]
                        set r2286_lbc_payload [binary format H* e9]
                    }
                    260 {
                        set r2286_lbc_factory [list binary_hex cc81]
                        set r2286_lbc_payload [binary format H* cc81]
                    }
                }
            }
            set r2286_lbc_stem "r2286_lbc_${r2286_lbc_i}_${r2286_lbc_source_kind}_${r2286_lbc_form}_v"
            set r2286_lbc_full $r2286_lbc_stem
            append r2286_lbc_full $r2286_lbc_payload Q
            set r2286_lbc_targets [list [list $r2286_lbc_stem SHORT]]
            if {$r2286_lbc_i == 258} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* c3]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            if {$r2286_lbc_i == 260} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* cc]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            lappend r2286_lbc_targets [list $r2286_lbc_full FULL]
            set r2286_lbc_setup {}
            set r2286_lbc_add {}
            set r2286_lbc_attached {}
            set r2286_lbc_callback [list lappend ::r2286_lbc_reads]
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_marker [lindex $r2286_lbc_target 1]
                set r2286_lbc_rc [catch {set $r2286_lbc_name $r2286_lbc_marker} r2286_lbc_value]
                lappend r2286_lbc_setup [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                set r2286_lbc_rc [catch {trace variable $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_add [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                if {$r2286_lbc_rc == 0} {
                    lappend r2286_lbc_attached $r2286_lbc_name
                }
            }
            set r2286_lbc_source {list $}
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \{}
            append r2286_lbc_source $r2286_lbc_full
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \}}
            set r2286_lbc_build_rc 0
            set r2286_lbc_build_result {}
            set r2286_lbc_constructor_hex {}
            if {$r2286_lbc_source_kind == "binary_source"} {
                set r2286_lbc_build_rc [catch {
                    if {$r2286_lbc_i < 256 || $r2286_lbc_i >= 258} {
                        set r2286_lbc_lead {list $}
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_lead \{}
                        binary scan $r2286_lbc_lead H* r2286_lbc_lead_hex
                        binary scan $r2286_lbc_stem H* r2286_lbc_stem_hex
                        if {$r2286_lbc_i < 256} {
                            set r2286_lbc_byte_hex [format %02x $r2286_lbc_i]
                        } else {
                            switch -- $r2286_lbc_i {
                                258 {set r2286_lbc_byte_hex c3a9}
                                259 {set r2286_lbc_byte_hex e9}
                                260 {set r2286_lbc_byte_hex cc81}
                            }
                        }
                        set r2286_lbc_constructor_hex "$r2286_lbc_lead_hex$r2286_lbc_stem_hex${r2286_lbc_byte_hex}51"
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_constructor_hex 7d}
                    } else {
                        binary scan $r2286_lbc_source H* r2286_lbc_constructor_hex
                    }
                    set r2286_lbc_source [binary format H* $r2286_lbc_constructor_hex]
                } r2286_lbc_build_result]
            }
            set ::r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $::r2286_lbc_reads
            set r2286_lbc_source_hex {}
            set r2286_lbc_source_scan_rc [catch {binary scan $r2286_lbc_source H* r2286_lbc_source_hex} r2286_lbc_source_scan_result]
            set r2286_lbc_result_hex {}
            set r2286_lbc_result_scan_rc [catch {binary scan $r2286_lbc_eval_result H* r2286_lbc_result_hex} r2286_lbc_result_scan_result]
            set r2286_lbc_payload_hex {}
            set r2286_lbc_payload_scan_rc [catch {binary scan $r2286_lbc_payload H* r2286_lbc_payload_hex} r2286_lbc_payload_scan_result]
            set r2286_lbc_utf8_hex {}
            set r2286_lbc_encoding_rc [catch {
                set r2286_lbc_utf8 [encoding convertto utf-8 $r2286_lbc_source]
                binary scan $r2286_lbc_utf8 H* r2286_lbc_utf8_hex
            } r2286_lbc_encoding_result]
            set r2286_lbc_remove {}
            foreach r2286_lbc_name $r2286_lbc_attached {
                set r2286_lbc_rc [catch {trace vdelete $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_remove [list $r2286_lbc_rc $r2286_lbc_value]
            }
            set r2286_lbc_cleanup {}
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_rc [catch {unset $r2286_lbc_name} r2286_lbc_value]
                lappend r2286_lbc_cleanup [list $r2286_lbc_rc $r2286_lbc_value]
            }
            lappend r2286_lbc_rows [list \
                case $r2286_lbc_i factory $r2286_lbc_factory \
                source_kind $r2286_lbc_source_kind form $r2286_lbc_form constructor_hex $r2286_lbc_constructor_hex \
                payload_scan [list $r2286_lbc_payload_scan_rc $r2286_lbc_payload_hex $r2286_lbc_payload_scan_result] \
                source_build [list $r2286_lbc_build_rc $r2286_lbc_build_result] \
                source_scan [list $r2286_lbc_source_scan_rc $r2286_lbc_source_hex $r2286_lbc_source_scan_result] \
                eval_code $r2286_lbc_eval_rc \
                result_scan [list $r2286_lbc_result_scan_rc $r2286_lbc_result_hex $r2286_lbc_result_scan_result] \
                reads $r2286_lbc_reads setup $r2286_lbc_setup trace_add $r2286_lbc_add \
                utf8_diagnostic [list $r2286_lbc_encoding_rc $r2286_lbc_utf8_hex $r2286_lbc_encoding_result] \
                trace_remove $r2286_lbc_remove cleanup $r2286_lbc_cleanup]
        }
    }
}
catch {unset ::r2286_lbc_reads}
set r2286_lbc_transcript [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
set r2286_lbc_transcript_hex {}
set r2286_lbc_export_rc [catch {binary scan $r2286_lbc_transcript H* r2286_lbc_transcript_hex} r2286_lbc_export_result]
set r2286_lbc_export [list LBC1 $r2286_lbc_export_rc $r2286_lbc_export_result $r2286_lbc_transcript_hex]
log local0. "R2286LBC|run=r2286lex1|ctx=HTTP_REQUEST|group_rc=$r2286_lbc_group_rc|group=$r2286_lbc_group|unit_rc=$r2286_lbc_unit_rc|unit=$r2286_lbc_unit|session=$r2286_lbc_session|rows=[llength $r2286_lbc_rows]|export_rc=$r2286_lbc_export_rc"
HTTP::respond 200 content $r2286_lbc_export "Content-Type" "text/plain" "X-R2286-Run" "r2286lex1" "X-R2286-TMM" "$r2286_lbc_group:$r2286_lbc_unit"
}

```

## Non-TMM full script (ASCII/LF)

This complete example labels the tmsh CLI context. For iApp implementation and
iCall, change only `tmsh_cli` to the appropriate label in separate full files;
the payload remains identical. Use the existing successfully executed wrapper
shape from the context report and preserve the full outer wrapper, rather than
inventing availability from object registration.

```tcl
set r2286_lbc_context tmsh_cli
set r2286_lbc_identity NOT_TMM
set r2286_lbc_rows {}
for {set r2286_lbc_i 0} {$r2286_lbc_i < 261} {incr r2286_lbc_i} {
    foreach r2286_lbc_source_kind {append_source binary_source} {
        foreach r2286_lbc_form {unbraced braced} {
            if {$r2286_lbc_i < 256} {
                set r2286_lbc_factory [list binary_byte $r2286_lbc_i]
                set r2286_lbc_payload [binary format H* [format %02x $r2286_lbc_i]]
            } else {
                switch -- $r2286_lbc_i {
                    256 {
                        set r2286_lbc_factory [list format_c 233]
                        set r2286_lbc_payload [format %c 233]
                    }
                    257 {
                        set r2286_lbc_factory [list format_c 769]
                        set r2286_lbc_payload [format %c 769]
                    }
                    258 {
                        set r2286_lbc_factory [list binary_hex c3a9]
                        set r2286_lbc_payload [binary format H* c3a9]
                    }
                    259 {
                        set r2286_lbc_factory [list binary_hex e9]
                        set r2286_lbc_payload [binary format H* e9]
                    }
                    260 {
                        set r2286_lbc_factory [list binary_hex cc81]
                        set r2286_lbc_payload [binary format H* cc81]
                    }
                }
            }
            set r2286_lbc_stem "r2286_lbc_${r2286_lbc_i}_${r2286_lbc_source_kind}_${r2286_lbc_form}_v"
            set r2286_lbc_full $r2286_lbc_stem
            append r2286_lbc_full $r2286_lbc_payload Q
            set r2286_lbc_targets [list [list $r2286_lbc_stem SHORT]]
            if {$r2286_lbc_i == 258} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* c3]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            if {$r2286_lbc_i == 260} {
                set r2286_lbc_prefix $r2286_lbc_stem
                append r2286_lbc_prefix [binary format H* cc]
                lappend r2286_lbc_targets [list $r2286_lbc_prefix PREFIX_1]
            }
            lappend r2286_lbc_targets [list $r2286_lbc_full FULL]
            set r2286_lbc_setup {}
            set r2286_lbc_add {}
            set r2286_lbc_attached {}
            set r2286_lbc_callback [list lappend ::r2286_lbc_reads]
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_marker [lindex $r2286_lbc_target 1]
                set r2286_lbc_rc [catch {set $r2286_lbc_name $r2286_lbc_marker} r2286_lbc_value]
                lappend r2286_lbc_setup [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                set r2286_lbc_rc [catch {trace variable $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_add [list $r2286_lbc_marker $r2286_lbc_rc $r2286_lbc_value]
                if {$r2286_lbc_rc == 0} {
                    lappend r2286_lbc_attached $r2286_lbc_name
                }
            }
            set r2286_lbc_source {list $}
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \{}
            append r2286_lbc_source $r2286_lbc_full
            if {$r2286_lbc_form == "braced"} {append r2286_lbc_source \}}
            set r2286_lbc_build_rc 0
            set r2286_lbc_build_result {}
            set r2286_lbc_constructor_hex {}
            if {$r2286_lbc_source_kind == "binary_source"} {
                set r2286_lbc_build_rc [catch {
                    if {$r2286_lbc_i < 256 || $r2286_lbc_i >= 258} {
                        set r2286_lbc_lead {list $}
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_lead \{}
                        binary scan $r2286_lbc_lead H* r2286_lbc_lead_hex
                        binary scan $r2286_lbc_stem H* r2286_lbc_stem_hex
                        if {$r2286_lbc_i < 256} {
                            set r2286_lbc_byte_hex [format %02x $r2286_lbc_i]
                        } else {
                            switch -- $r2286_lbc_i {
                                258 {set r2286_lbc_byte_hex c3a9}
                                259 {set r2286_lbc_byte_hex e9}
                                260 {set r2286_lbc_byte_hex cc81}
                            }
                        }
                        set r2286_lbc_constructor_hex "$r2286_lbc_lead_hex$r2286_lbc_stem_hex${r2286_lbc_byte_hex}51"
                        if {$r2286_lbc_form == "braced"} {append r2286_lbc_constructor_hex 7d}
                    } else {
                        binary scan $r2286_lbc_source H* r2286_lbc_constructor_hex
                    }
                    set r2286_lbc_source [binary format H* $r2286_lbc_constructor_hex]
                } r2286_lbc_build_result]
            }
            set ::r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $::r2286_lbc_reads
            set r2286_lbc_source_hex {}
            set r2286_lbc_source_scan_rc [catch {binary scan $r2286_lbc_source H* r2286_lbc_source_hex} r2286_lbc_source_scan_result]
            set r2286_lbc_result_hex {}
            set r2286_lbc_result_scan_rc [catch {binary scan $r2286_lbc_eval_result H* r2286_lbc_result_hex} r2286_lbc_result_scan_result]
            set r2286_lbc_payload_hex {}
            set r2286_lbc_payload_scan_rc [catch {binary scan $r2286_lbc_payload H* r2286_lbc_payload_hex} r2286_lbc_payload_scan_result]
            set r2286_lbc_utf8_hex {}
            set r2286_lbc_encoding_rc [catch {
                set r2286_lbc_utf8 [encoding convertto utf-8 $r2286_lbc_source]
                binary scan $r2286_lbc_utf8 H* r2286_lbc_utf8_hex
            } r2286_lbc_encoding_result]
            set r2286_lbc_remove {}
            foreach r2286_lbc_name $r2286_lbc_attached {
                set r2286_lbc_rc [catch {trace vdelete $r2286_lbc_name r $r2286_lbc_callback} r2286_lbc_value]
                lappend r2286_lbc_remove [list $r2286_lbc_rc $r2286_lbc_value]
            }
            set r2286_lbc_cleanup {}
            foreach r2286_lbc_target $r2286_lbc_targets {
                set r2286_lbc_name [lindex $r2286_lbc_target 0]
                set r2286_lbc_rc [catch {unset $r2286_lbc_name} r2286_lbc_value]
                lappend r2286_lbc_cleanup [list $r2286_lbc_rc $r2286_lbc_value]
            }
            lappend r2286_lbc_rows [list \
                case $r2286_lbc_i factory $r2286_lbc_factory \
                source_kind $r2286_lbc_source_kind form $r2286_lbc_form constructor_hex $r2286_lbc_constructor_hex \
                payload_scan [list $r2286_lbc_payload_scan_rc $r2286_lbc_payload_hex $r2286_lbc_payload_scan_result] \
                source_build [list $r2286_lbc_build_rc $r2286_lbc_build_result] \
                source_scan [list $r2286_lbc_source_scan_rc $r2286_lbc_source_hex $r2286_lbc_source_scan_result] \
                eval_code $r2286_lbc_eval_rc \
                result_scan [list $r2286_lbc_result_scan_rc $r2286_lbc_result_hex $r2286_lbc_result_scan_result] \
                reads $r2286_lbc_reads setup $r2286_lbc_setup trace_add $r2286_lbc_add \
                utf8_diagnostic [list $r2286_lbc_encoding_rc $r2286_lbc_utf8_hex $r2286_lbc_encoding_result] \
                trace_remove $r2286_lbc_remove cleanup $r2286_lbc_cleanup]
        }
    }
}
catch {unset ::r2286_lbc_reads}
set r2286_lbc_transcript [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
set r2286_lbc_transcript_hex {}
set r2286_lbc_export_rc [catch {binary scan $r2286_lbc_transcript H* r2286_lbc_transcript_hex} r2286_lbc_export_result]
set r2286_lbc_export [list LBC1 $r2286_lbc_export_rc $r2286_lbc_export_result $r2286_lbc_transcript_hex]
puts $r2286_lbc_export
```

## Required returned artifacts and interpretation

Return full version/context/commit records, exact wrapper/payload/archive bytes
and SHA-256 before and after transfer, loader statuses, raw stdout/stderr, raw
HTTP headers+bodies, continuous LTM chunks, actual TMM/CMP observations and the
real backend baseline. For each transcript retain all 1,044 rows and the full
outer export status. Keep exact `source_build`, `source_scan`, `eval_code`,
`result_scan`, `reads`, setup, trace-add/remove, cleanup and encoding-diagnostic
boundaries. Report character-class conclusions only for discriminating rows
whose setup, source production and original eval succeeded; trace absence is
not proof of a missing read when trace registration refused. Do not normalize
names, infer a Unicode category, fill absent bytes from a standalone engine, or
merge producer/context/version columns. Unknown or unsupported boundaries stay
explicit. These measurements may inform a future independently selected lexical
scanner receipt; they cannot issue one by themselves.
