# BIG-IP name identity and callable lifetime checks

These checks distinguish runtime storage identity and callable lifetime on real BIG-IP. Report measurements without assuming C Tcl, Jim, the simulator, or another BIG-IP version supplies the answer. The reference evidence is [BIGIP_RESULTS.md](BIGIP_RESULTS.md). The implementation needs separate answers for counted value bytes, variable/command keys, lexical variable tokens, and activated procedure allocations.

Work on branch `work`. Write results in the peer file `FOLLOWUP_APPLIANCE_RESULTS.md` in this directory, commit both evidence and results on the same branch, and push. State the exact BIG-IP version, build, edition/hotfix, topology, CMP configuration and actual reached TMM group/unit pairs prominently. Record the source commit and original source SHA-256 for every case.

## Lab and evidence requirements

Use uniquely named disposable rules, folders, partitions and VIPs in the authorized isolated lab. Clients and servers may both run on `dev.bragi0.com`, including loopback tests using any suitable protocol. All test virtual servers must use **SNAT automap**. Show actual client-to-VIP-to-backend correlation and backend peer address when forwarding traffic. Retain raw `/var/log/ltm` from before rule creation through cleanup, raw client replies, backend transcripts, object configurations and exact request/phase identifiers.

Discover the actual TMM roster. Run name checks on every reachable pair, reporting coverage gaps explicitly. Keep ordinary-global-induced CMP demotion separate from storage sharing. Reuse the available traffic/backend tools where useful. Do not restart TMM, change global CMP, overwrite configuration, save global configuration, or touch unrelated objects. Delete only exact owned names. Record provider config deletion separately from runtime callable retirement; a successful `tmsh delete` does not establish retirement.

Every outcome is initially unmeasured. Retain complete load diagnostics and runtime catch status, message bytes and available error metadata. A rejection is evidence about that source/context only. Do not substitute appliance shell Tcl for TMM or silently edit a rejected source into a successful one. Any repaired source is a new separately hashed case.

## Exact runtime variable-name controls

Use the following payloads as ASCII bytes with LF line endings. Each payload runs in its own `HTTP_REQUEST` event. Replace `RUN_TOKEN` with an ASCII alphanumeric run identifier before generating the config, and preserve that generated payload as evidence. Source text contains **no literal NUL and no non-ASCII characters**. `binary format H* 0042` creates exactly NUL followed by ASCII `B` at runtime; it is not the two printable characters backslash and zero. All names begin with a unique run prefix and are disposable event-local cells.

NUL identity payload:

```tcl
set prefix __tcl_lsp_2286_RUN_TOKEN_nul_
set plain ${prefix}A
set counted $plain
append counted [binary format H* 0042]
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

Repeat with the plain key written last, and with the plain key unset first. Add separately hashed controls for NUL in an **array root** and in an **array index**, preserving the original name/index object when using `set`, `upvar`, `info exists`, `unset` and classic variable traces. Keep root and index results separate: different length conventions can apply. A trace observer must be a uniquely named disposable dynamic Tcl proc and must record actual root/index/operation callback arguments as byte hex; it must not change stock commands. Prove aliasing with different stored values and a mutation/unset discriminator. A successful counted-name round trip alone does not prove distinct keys.

Unicode identity payload:

```tcl
set prefix __tcl_lsp_2286_RUN_TOKEN_unicode_
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
set rc [catch {set $pre PRECOMPOSED} result]
lappend rows [list write_pre $rc $result]
set rc [catch {set $decomposed DECOMPOSED} result]
lappend rows [list write_decomposed $rc $result]
foreach name [list $pre $decomposed] {
    set rc [catch {set $name} result]
    lappend rows [list read $rc $result]
}
set rc [catch {unset $pre} result]
lappend rows [list unset_pre $rc $result]
set rc [catch {set $decomposed} result]
lappend rows [list remaining_decomposed $rc $result]
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
```

The intended characters are U+00E9 (precomposed e-acute) and U+0065 followed by U+0301 (e plus combining acute). They are created with `format %c`, which is a **different producer** from `binary format H* c3a9` and `binary format H* 65cc81`. Run separate raw-bytearray controls with those exact hex sequences. Never equate the producer representations from their display. Preserve `string length`, raw hex and any actual UTF-8 conversion outcome. Repeat in reverse write/delete order. Record inability to construct a character as a producer failure, not name inequality.

In separately hashed cases, construct actual evaluated script values containing unbraced `$name` and braced `${name}` tokens, then `eval` them. Record exact script hex and lexical outcome. This tests the TMM runtime parser, not config-source acceptance. Also test disposable namespace/procedure names with both producers through dynamically constructed `namespace`, `proc`, `rename` and lookup commands, catching each operation. Preserve command lookup and variable lookup separately. Clean only exact created names; avoid a wildcard over stock namespaces.

## Expression discriminators

The measured `"abcd" matches "bc"` result rules out substring/search behavior but does not distinguish exact equality from whole-string glob matching. In an independent ASCII dynamic payload, record `catch {expr $expression}` for every exact expression below, preserving result/error bytes and context:

```tcl
set rows {}
foreach expression {
    {"abcd" matches "a*"}
    {"a*" matches "a*"}
    {"a*" matches "abcd"}
    {"abcd" matches "a.*"}
    {"abcd" matches "bc"}
    {"abcd" matches "abcd"}
    {1 or 0 matches 0}
    {not "abc" starts_with "a"}
    {not ("abc" starts_with "a")}
} {
    set rc [catch {expr $expression} result]
    lappend rows [list $expression $rc $result]
}
set rows
```

Run in TMM first. The existing five-context driver may transport the identical payload to tmsh, iApp implementation and iCall if available, preserving their distinct identities. Do not infer APL behavior. Do not expand a discriminating result into an untested complete operator precedence table.

## Materializing the wrapper without byte changes

Keep each name/expression payload in a separate ASCII `.tcl` file. This offline Python generator produces the exact config, source hex and hash. It uses hexadecimal transport so disabled literal command heads inside the payload cannot be mistaken for evidence that the literal loader admitted them. Dynamic execution remains an independent observation.

```python
import hashlib
from pathlib import Path

payload = Path("PAYLOAD.tcl").read_bytes()
assert payload.isascii() and b"\x00" not in payload and b"\r" not in payload
assert b"RUN_TOKEN" not in payload
rule = "__tcl_lsp_2286_UNIQUE_RUN_CASE"
assert rule.replace("_", "").isalnum()
body = (
    "ltm rule /Common/" + rule + " {\n"
    "    when HTTP_REQUEST {\n"
    "        set payload [binary format H* " + payload.hex() + "]\n"
    "        set rc [catch {eval $payload} result]\n"
    "        binary scan $result H* result_hex\n"
    "        log local0. \"R2286 " + rule + " tmm=[TMM::cmp_group]:[TMM::cmp_unit] rc=$rc result_hex=$result_hex\"\n"
    "        HTTP::respond 200 content [list $rc $result_hex]\n"
    "    }\n"
    "}\n"
).encode("ascii")
path = Path(rule + ".conf")
path.write_bytes(body)
Path(str(path) + ".hex").write_text(body.hex() + "\n", encoding="ascii")
print(path, hashlib.sha256(body).hexdigest(), len(body))
print("payload", hashlib.sha256(payload).hexdigest(), len(payload))
```

The wrapper has literal physical LF line endings and no continuation backslash at end-of-line. Its Python `\n` escapes write LF; its `\"` escapes write ordinary quote bytes. Confirm every transferred config/payload hash on the appliance before load. Retain raw error information if the outer catch fails; HTTP status 200 is merely transport, not success. A failed outer catch makes inner coverage incomplete.

## Activated procedure replacement and original caller cells

Create a provider and a distinct caller in an owned folder/partition, with full absolute `call` routing. Do not repeat partition-root routing already measured. Record config object and active TMM callable identities independently.

Provider old body, top-level in its rule:

```tcl
proc incarnation {name} {
    upvar 1 $name cell
    set cell OLD_WRITE
    return [list OLD [namespace current] $cell]
}
```

Provider replacement body at the **same exact rule object name**:

```tcl
proc incarnation {name {suffix NEW_DEFAULT}} {
    upvar 1 $name cell
    set cell NEW_WRITE
    return [list NEW $suffix [namespace current] $cell]
}
```

`namespace current` may require separately transporting these bodies dynamically if the literal loader refuses it; preserve the original rejected body and generate a separate ASCII dynamic command invocation within the same top-level proc. Do not conceal that distinction. The caller's event sets local `cell` to `CALLER_SEED`, catches `call /partition/folder/PROVIDER::incarnation cell`, and returns/logs catch status, full returned value and its local cell after the call. Invoke both one and two actual arguments as separate requests to distinguish formal binding changes.

Measure fresh traffic with OLD activated; after deleting only the provider config; after recreating that same provider name with NEW body/formals; after a failed separately hashed replacement; and after recreating OLD again. Prove every actual responding TMM. Record whether the provider is attached, whether its config object exists, and which old/new body and caller-cell mutation ran. Do not infer callable replacement from a successful config merge or a changed `RULE_INIT` log.

If the isolated lab supports a genuine bounded suspension **inside an entered provider procedure**, compare that already-entered invocation with a fresh invocation after replacement/deletion. Use a documented event-valid mechanism and a disposable backend/client on `dev.bragi0.com` with SNAT automap. Specify the exact primitive and show why the procedure frame is still entered. Holding an HTTP connection after a proc has returned does not test frame retention. Do not block TMM, restart services or improvise an unbounded wait. Mark this control unsupported/unreached if a valid mechanism is unavailable.

## Results contract

Write `FOLLOWUP_APPLIANCE_RESULTS.md` beside this source document on `work`. Include exact version/build and roster first, source commit, fixture and evidence hashes, per-case loader/runtime verdict, original byte/provenance distinctions, raw paths, cleanup proof and coverage limits. Separate measured conclusions from unresolved alternatives. Include sufficient controls to support or withdraw each identity/lifetime claim. Keep documentation about current contracts and evidence only; do not describe development stages or implementation history.
