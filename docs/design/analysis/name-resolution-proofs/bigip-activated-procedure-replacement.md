# naming.bigip.activated-procedure-replacement

Kind: `native-observation`

## Problem statement

Deleting a configuration object can be confused with retiring its activated TMM procedure. A rejected delete or failed replacement can appear to prove lifetime even though the old object stayed present.

## Question

What did fresh calls observe after accepted provider deletion, successful same-name replacement and rejected replacement?

## Conclusion

The fresh-name control proves OLD remains callable after accepted config deletion, NEW replaces body/formals for fresh calls, rejected replacement leaves NEW active, and recreating OLD restores OLD behaviour. Rejected dependency deletes remain counterexamples; they cannot prove retirement. No bounded entered-frame replacement was reached.

## Scope

Authoritative fresh provider/caller object names, exact OLD/NEW/invalid sources, explicit object absence and32responses per each of four TMMs. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

The fresh-name control proves OLD remains callable after accepted config deletion, NEW replaces body/formals for fresh calls, rejected replacement leaves NEW active, and recreating OLD restores OLD behaviour. Rejected dependency deletes remain counterexamples; they cannot prove retirement. No bounded entered-frame replacement was reached.

The measured report rows and their limits are:


The exact literal OLD and NEW provider sources were rejected at load because `namespace` is disabled in literal iRule source:

```text
01070151:3: Rule [...] error: ...:4: error: [command is disabled: "namespace"][namespace current]
```

Separately hashed repaired providers constructed `namespace current` dynamically. Those sources loaded; inside both OLD and NEW procedures it returned `::`. The caller used full absolute F5 routing to `/R2286_r2286m/lifetime/..._provider::incarnation`, passed an event-local `cell` by `upvar 1`, and returned both the procedure value and caller cell.

The authoritative lifetime control used fresh provider and caller object names that had never held a literal dependency. The provider was not attached to the VIP; the caller and backend rule were attached. The call target came from the byte-preserved request header.

| Measured configuration | Provider config object | One actual argument | Two actual arguments | Coverage |
| --- | --- | --- | --- | --- |
| OLD active | Present, OLD body | catch 0; `OLD :: OLD_WRITE`; caller cell `OLD_WRITE` | catch 1; `wrong # args: should be "call incarnation <name>`; caller cell `CALLER_SEED` | 32 responses from each of `0:0`–`0:3` |
| After provider delete | Absent; `tmsh list` returned not found | Same OLD success and mutation | Same OLD arity error and unchanged cell | 32 per TMM in both traffic batches |
| Same name recreated NEW | Present, NEW body/formals | catch 0; `NEW NEW_DEFAULT :: NEW_WRITE`; cell `NEW_WRITE` | catch 0; `NEW EXPLICIT_SUFFIX :: NEW_WRITE`; cell `NEW_WRITE` | 32 per TMM in both traffic batches |
| Invalid replacement attempted | Existing NEW object retained | Same NEW default result | Same NEW explicit result | 32 per TMM in both traffic batches |
| Same name recreated OLD | Present, OLD body/formals | OLD success restored | OLD arity error restored | 32 per TMM in both traffic batches |

Deleting the fresh provider configuration therefore did not retire its already activated procedure from any reached TMM. Recreating the same owner path replaced the active body and formal list for fresh invocations. A failed replacement did not disturb the prior NEW callable. Recreating OLD replaced it again. These observations are about fresh calls after each configuration operation; they do not establish the lifetime of an already-entered suspended frame.

The invalid replacement was rejected exactly as follows:

```text
01070151:3: Rule [/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider_fresh] error: /R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider_fresh:4: error: [parse error: missing close-bracket][[list FAILED]
```

The original literal caller created a configuration dependency. Subsequent attempts using a dynamically constructed constant and a request-header target under that same caller object name still rejected provider deletion with:

```text
01070265:3: The rule (/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider) cannot be deleted because it is in use by a rule (/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_caller).
```

The capture groups named `provider_deleted`, `dynamic_provider_deleted` and `header_provider_deleted` followed those rejected delete attempts; the provider object was still present, so those responses are not callable-retirement evidence. They are retained as counterexamples rather than relabelled or discarded. The separately named `fresh_provider_deleted` groups follow an accepted delete plus explicit object-absence proof and are the authoritative deletion result.

No documented, bounded, event-valid mechanism was available to suspend execution inside an entered provider procedure without blocking TMM. The entered-frame replacement control is unsupported/unreached.


## Exact evidence

[bigip-activated-procedure-replacement.json](bigip-activated-procedure-replacement.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
