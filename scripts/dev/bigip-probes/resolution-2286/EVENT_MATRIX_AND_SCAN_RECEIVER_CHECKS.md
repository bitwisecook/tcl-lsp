# BIG-IP retained event matrices and scan receivers

Measure complete lexical byte matrices in actual RULE_INIT and CLIENT_ACCEPTED
contexts, and reach the sequential scan-output receiver after a write callback
rebinds its second destination. This request concerns compiler naming semantics.
Portable C Tcl and Jim outcomes do not establish BIG-IP behavior.

Use the same **work** branch. Write the peer report
**EVENT_MATRIX_AND_SCAN_RECEIVER_RESULTS.md** beside this request and commit and
push the exact source/raw evidence/report to that branch. State the actual
BIG-IP product, version, build, point release and hotfix at the top. Record the
source commit and all source/evidence SHA-256 values. Retain existing lexical
and lifecycle reports unchanged; their complete and incomplete observations
keep their original scopes. The reference appliance is BIG-IP 21.1.0.1 build
0.0.26 Point Release 1, which this run must independently identify.

Clients and servers can use any protocol and loopback testing on
**dev.bragi0.com**. Every owned BIG-IP virtual server requires **SNAT automap**.
These complete wrappers use TCP and HTTP profiles. Prove backend delivery using
an owned backend on dev.bragi0.com and retain complete request/response bytes,
received peer address and a run marker before probing the rule. An iRule response
alone is not backend proof. Do not collect credentials, secrets, unrelated
configuration or security-sensitive logs. Retain only compiler-relevant owned
object identities, version/CMP/TMM data and run-marked LTM lines.

## Exact source and byte requirements

Both complete iRule bodies below are ASCII with LF endings. There are no
literal non-ASCII characters or NUL bytes in configuration source. The event
payload produces all binary bytes at runtime and distinguishes `%c 233`,
`%c 769`, binary `c3a9`, binary `e9` and binary `cc81`. Preserve physical line
breaks, split backslash-continued list construction, dollar signs, braces,
quotes and escape bytes exactly. `%c 769` must not be rewritten to UTF-8 cc81.
Do not reencode a binary source or inspect it before the retained eval/getter
order. No Unicode normalization, replacement characters or source repairs are
allowed. Hash the complete bodies and loaded configuration before/after transfer.

The event payload has the original 1,044 rows: 261 cases × two source producers
× two substitution forms. Its only operand changes replace the four qualified
read-log references `::r2286_lbc_reads` with direct event-local
`r2286_lbc_reads`. The read callback directly invokes `lappend`, creating no
extra procedure frame. Original payload construction, target seeding, trace
installation, eval, source/result byte scans, post-eval UTF-8 diagnostic,
trace removal and cleanup order are retained. This is a separately measured
source adaptation, not a claim of identical native object behavior.

## Bounded execution and retained transport

Use fresh owned names and verify absence before creating anything. Do not
restart TMM, change global CMP or save global configuration. Capture the
actual TMM roster and CMP status with the installed event rule. Lightweight
identity traffic may select up to four stable connection tuples before the
heavy rule, but every response must independently report its actual unit.
Keep at most **four heavy connections**, sequentially, with no reconnect to
fill a missing unit. If the rule demotes CMP, report that actual coverage and
leave other units unknown.

RULE_INIT runs once at load and keeps each unit's exact envelope in its own
`static::` cell. CLIENT_ACCEPTED runs once per allowed connection and keeps its
own envelope in connection-local variables. HTTP_REQUEST only retrieves the
retained bytes in pages. On the same keep-alive connection, fetch page 0 for
`context=RULE_INIT` and `context=CLIENT_ACCEPTED` at
`/r2286-event-matrix?context=RULE_INIT&page=0`, then all declared pages for that
context. Each page is at most 65,536 hex characters; there are at most 512 pages
per context per connection. Retrieval does not rerun either event payload.
One READY LTM marker per event replaces full row/chunk logging.

Accept an event transcript only if every page succeeds, identities agree,
sequence 0 through count−1 is complete without conflicts, concatenated hex
length is exact, decoding produces one envelope and all 1,044 unique
case/producer/form keys exist. Decode the retained Tcl list with the public
lexical decoder; retain the complete page-reassembly source/hash. Missing pages, unavailable static/connection
state, unsupported commands or identity mismatches leave the corresponding
matrix unknown. Preserve each raw HTTP request/header/body and transport code.
The reconstructed hex contains the original five-field LBC1 transcript. Write
an outer ASCII envelope `LBC1 0 {} CONCATENATED_HEX` with one LF terminator and
run `decode-lexical-byte-class.tcl` from the appliance report source on it;
that local decoder is a lossless data transformation, not appliance execution.
Do not extrapolate from a configured CPU count or earlier connection mapping.

Separate static result retention from a native RULE_INIT broadcast mechanism:
a READY initialization on each unit and that unit's later read do not prove a
single writer broadcasts. Preserve the existing per-unit mutation/unset facts.

The scan-receiver rule is a **separate owned virtual/rule**. It may demote CMP
because its global replacement purpose explicitly uses an ordinary global.
Send at most four sequential bounded scan requests; record actual units and
separate local `upvar 0` from global `upvar #0` outcomes. Both callbacks first
unset the existing `second` scalar, then link it, and append `REBOUND` only
after successful linking. The discriminating output assignment is reached
only when REBOUND is present. Record trace-add, scan, original first/second,
replacement-target and trace-removal outcomes independently. Do not repair
an unsupported trace command or runtime failure. The isolated quiet-missing
control has a fresh procedure frame and records pre-operation absence.

## Complete event-matrix iRule

```tcl
when RULE_INIT {
set r2286_lbc_context RULE_INIT
set r2286_lbc_group_rc [catch {TMM::cmp_group} r2286_lbc_group]
set r2286_lbc_unit_rc [catch {TMM::cmp_unit} r2286_lbc_unit]
set r2286_lbc_identity [list $r2286_lbc_group_rc $r2286_lbc_group $r2286_lbc_unit_rc $r2286_lbc_unit]
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
            set r2286_lbc_callback [list lappend r2286_lbc_reads]
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
            set r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $r2286_lbc_reads
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
catch {unset r2286_lbc_reads}
set r2286_lbc_envelope [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
binary scan $r2286_lbc_envelope H* static::__tcl_lsp_2286_evt2_init_hex
set static::__tcl_lsp_2286_evt2_init_identity $r2286_lbc_identity
set static::__tcl_lsp_2286_evt2_init_rows [llength $r2286_lbc_rows]
log local0. [list R2286EVT2 READY RULE_INIT $r2286_lbc_identity $static::__tcl_lsp_2286_evt2_init_rows [string length $static::__tcl_lsp_2286_evt2_init_hex]]
}
when CLIENT_ACCEPTED {
set r2286_lbc_context CLIENT_ACCEPTED
set r2286_lbc_group_rc [catch {TMM::cmp_group} r2286_lbc_group]
set r2286_lbc_unit_rc [catch {TMM::cmp_unit} r2286_lbc_unit]
set r2286_lbc_identity [list $r2286_lbc_group_rc $r2286_lbc_group $r2286_lbc_unit_rc $r2286_lbc_unit]
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
            set r2286_lbc_callback [list lappend r2286_lbc_reads]
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
            set r2286_lbc_reads {}
            set r2286_lbc_eval_rc NOT_REACHED
            set r2286_lbc_eval_result {}
            if {$r2286_lbc_build_rc == 0} {
                set r2286_lbc_eval_rc [catch {eval $r2286_lbc_source} r2286_lbc_eval_result]
            }
            set r2286_lbc_reads $r2286_lbc_reads
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
catch {unset r2286_lbc_reads}
set r2286_lbc_envelope [list LBC1 $r2286_lbc_context $r2286_lbc_identity [llength $r2286_lbc_rows] $r2286_lbc_rows]
binary scan $r2286_lbc_envelope H* r2286_evt2_accept_hex
set r2286_evt2_accept_identity $r2286_lbc_identity
set r2286_evt2_accept_rows [llength $r2286_lbc_rows]
log local0. [list R2286EVT2 READY CLIENT_ACCEPTED $r2286_lbc_identity $r2286_evt2_accept_rows [string length $r2286_evt2_accept_hex]]
}
when HTTP_REQUEST {
if {[HTTP::path] == "/r2286-event-matrix"} {
    set r2286_evt2_context [URI::query [HTTP::uri] context]
    set r2286_evt2_page [URI::query [HTTP::uri] page]
    set r2286_evt2_rc [catch {
        if {![string is integer -strict $r2286_evt2_page] || $r2286_evt2_page < 0 || $r2286_evt2_page >= 512} {
            error BAD_PAGE
        }
        set r2286_evt2_group_rc [catch {TMM::cmp_group} r2286_evt2_group]
        set r2286_evt2_unit_rc [catch {TMM::cmp_unit} r2286_evt2_unit]
        set r2286_evt2_reader_identity [list $r2286_evt2_group_rc $r2286_evt2_group $r2286_evt2_unit_rc $r2286_evt2_unit]
        if {$r2286_evt2_context == "RULE_INIT"} {
            set r2286_evt2_original_identity $static::__tcl_lsp_2286_evt2_init_identity
            set r2286_evt2_rows $static::__tcl_lsp_2286_evt2_init_rows
            set r2286_evt2_hex $static::__tcl_lsp_2286_evt2_init_hex
        } elseif {$r2286_evt2_context == "CLIENT_ACCEPTED"} {
            set r2286_evt2_original_identity $r2286_evt2_accept_identity
            set r2286_evt2_rows $r2286_evt2_accept_rows
            set r2286_evt2_hex $r2286_evt2_accept_hex
        } else {
            error BAD_CONTEXT
        }
        if {$r2286_evt2_original_identity != $r2286_evt2_reader_identity} {
            error IDENTITY_MISMATCH
        }
        set r2286_evt2_length [string length $r2286_evt2_hex]
        set r2286_evt2_count [expr {($r2286_evt2_length + 65535) / 65536}]
        if {$r2286_evt2_count > 512 || $r2286_evt2_page >= $r2286_evt2_count} {error PAGE_LIMIT}
        set r2286_evt2_start [expr {$r2286_evt2_page * 65536}]
        set r2286_evt2_chunk [string range $r2286_evt2_hex $r2286_evt2_start [expr {$r2286_evt2_start + 65535}]]
        list run R2286EVT2 context $r2286_evt2_context original_identity $r2286_evt2_original_identity reader_identity $r2286_evt2_reader_identity page $r2286_evt2_page count $r2286_evt2_count rows $r2286_evt2_rows total_hex_length $r2286_evt2_length chunk_hex $r2286_evt2_chunk
    } r2286_evt2_answer]
    HTTP::respond 200 content [list transport_catch $r2286_evt2_rc result $r2286_evt2_answer] "Content-Type" "text/plain; charset=us-ascii"
}
}
```

## Complete scan-receiver iRule

```tcl
when RULE_INIT {
    proc __tcl_lsp_2286_evt2_watch_local {n k op} {
        uplevel 1 {unset second; upvar 0 replacement second; lappend reached REBOUND}
    }
    proc __tcl_lsp_2286_evt2_watch_global {n k op} {
        uplevel 1 {unset second; upvar #0 ::__tcl_lsp_2286_evt2_replacement second; lappend reached REBOUND}
    }
    proc __tcl_lsp_2286_evt2_scan {mode} {
        set replacement BEFORE
        set first INIT
        set second ORIGINAL
        set reached {}
        if {$mode == "GLOBAL"} {
            set ::__tcl_lsp_2286_evt2_replacement BEFORE
            set callback __tcl_lsp_2286_evt2_watch_global
        } else {
            set callback __tcl_lsp_2286_evt2_watch_local
        }
        set add_rc [catch {trace variable first w $callback} add_answer]
        set n first
        set m second
        set h scan
        set code [catch {set count [$h {7 9} {%d %d} $n $m]} answer]
        set remove_rc [catch {trace vdelete first w $callback} remove_answer]
        set first_rc [catch {set first} first_answer]
        set second_rc [catch {set second} second_answer]
        if {$mode == "GLOBAL"} {
            set target_rc [catch {set ::__tcl_lsp_2286_evt2_replacement} target_answer]
        } else {
            set target_rc [catch {set replacement} target_answer]
        }
        list mode $mode trace_add [list $add_rc $add_answer] scan [list $code $answer] reached $reached first [list $first_rc $first_answer] second [list $second_rc $second_answer] target [list $target_rc $target_answer] trace_remove [list $remove_rc $remove_answer]
    }
    proc __tcl_lsp_2286_evt2_quiet_missing {} {
        set before [info exists isolated_missing]
        set h lappend
        set code [catch {$h isolated_missing} answer]
        list before $before operation [list $code $answer] after [info exists isolated_missing]
    }
}
when HTTP_REQUEST {
    if {[HTTP::path] == "/r2286-scan-receiver"} {
        set r2286_evt2_group_rc [catch {TMM::cmp_group} r2286_evt2_group]
        set r2286_evt2_unit_rc [catch {TMM::cmp_unit} r2286_evt2_unit]
        set r2286_evt2_identity [list $r2286_evt2_group_rc $r2286_evt2_group $r2286_evt2_unit_rc $r2286_evt2_unit]
        set r2286_evt2_local_rc [catch {__tcl_lsp_2286_evt2_scan LOCAL} r2286_evt2_local]
        set r2286_evt2_global_rc [catch {__tcl_lsp_2286_evt2_scan GLOBAL} r2286_evt2_global]
        set r2286_evt2_quiet_rc [catch {__tcl_lsp_2286_evt2_quiet_missing} r2286_evt2_quiet]
        HTTP::respond 200 content [list run R2286EVT2 context HTTP_REQUEST identity $r2286_evt2_identity local [list $r2286_evt2_local_rc $r2286_evt2_local] global [list $r2286_evt2_global_rc $r2286_evt2_global] quiet [list $r2286_evt2_quiet_rc $r2286_evt2_quiet]] "Content-Type" "text/plain; charset=us-ascii"
    }
}
```

## Required report and cleanup

Report each build/context/actualunit separately. Include all source hashes,
loaded configuration, raw statuses, READY markers, page inventory, complete
reconstructed envelopes and decoded matrices. Keep syntax/error rows and the
binary-source braced257 missing-cell counterexample; successful construction
and scanned bytes do not certify dynamic variable-key identity. State whether
each intended callback/link/output receiver was reached. Include rejected load,
unsupported command, timeout and incomplete outcomes without substituting a
standalone interpreter. These measurements grant no C header, ABI, CPP,
compiler-hook, allocated namespace or unmeasured context permission.

Remove only owned objects/clients/servers created for the run and verify their
absence. Retain run-specific raw evidence plus a complete checksummed inventory
under `evidence/`. Commit and push the peer report, source and evidence to
**work**. Return the report commit/link, evidence link, archive SHA-256 if used,
exact appliance version and remaining unknowns.

Expected body hashes:

- `original_payload_sha256`: `af1d6366e57927bc72cd756bf8804e1a1713de958561f30258876603bc120ae9`
- `adapted_payload_sha256`: `6ea52b6cc900e12c7199530972ed655b9254d8fbf10ab3ad8fd6b09c869be5a2`
- `event_matrix_sha256`: `8f7ebe6041e5b4931ec2a550bf610f02e57fc9f5213c2eb3869976ac44856e3c`
- `scan_receiver_sha256`: `f075ddbaa2ab8398a13ed86c033d2986cd715897f59eaed352094e347049cf89`
