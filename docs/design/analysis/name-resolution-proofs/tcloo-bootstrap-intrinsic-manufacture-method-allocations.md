# naming.tcloo.bootstrap-intrinsic-manufacture-method-allocations

Kind: `native-observation`

## Problem statement

A class role alone does not certify stock manufacture dispatch when own methods or mixins replace it.

## Question

Which actual own method type/visibility records implement create/new/configure, and does an own create override take precedence?

## Conclusion

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

## Scope

Configured original C private/public object APIs on fresh bootstrap and explicitly printed rename/lifecycle/dispatch controls. ASCII Tcl_EvalEx source controls and actual selected header fields; no compiler admission, cache, variable, method-body, observer, Registry-role or Rust parity capability. C8.4/C8.5/Jim expose no oo::class in the measured initialized interpreter. BIG-IP not tested.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls.. Dialect: C Tcl.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls.. Dialect: C Tcl.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls.. Dialect: C Tcl.

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls.. Dialect: C Tcl.

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls.. Dialect: C Tcl.

C8.6/C9 classRoot class methods contain intrinsic create/new; its object facet has a flags-only unexported new record. C9 support owns public intrinsic configure. Installing an own create method on classRoot changes the actual own table and calling classRoot create returns OVERRIDE. Roles do not certify unchanged dispatch after such mutations.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Exact configured compile argv, selected AC_FLAGS, public/private header, library, source, Makefile and launched executable digests in attached provider receipts; each successful captured process is0. C9.0 V2 compile1/no native process is preserved separately.. Channel: Exact ASCII Jim_Eval info patchlevel/info commands; no TclOO private-header API.. Dialect: Jim Tcl.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### bigip

Status: `not-tested`. Version: not recorded. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `v1-probe-c` (input): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/probe.c). Exact original probe input
- `v1-capture-py` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/capture.py). Exact original configured build/run program; portable rerun requires separately matching provider source/library/header/build inputs.
- `v2-probe-c` (input): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/probe.c). Exact original probe input
- `v2-capture-py` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/capture.py). Exact original configured build/run program; portable rerun requires separately matching provider source/library/header/build inputs.
- `v3-probe-c` (input): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/probe.c). Exact original probe input
- `v3-capture-py` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/capture.py). Exact original configured build/run program; portable rerun requires separately matching provider source/library/header/build inputs.
- `v1-tcl8.4-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/receipt.json). Original v1 8.4.20 receipt.json; original process/compiler channels remain distinct.
- `v1-tcl8.4-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/stdout.tsv). Original v1 8.4.20 stdout.tsv; original process/compiler channels remain distinct.
- `v1-tcl8.4-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/stderr). Original v1 8.4.20 stderr; original process/compiler channels remain distinct.
- `v1-tcl8.4-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.4.20/compile.stderr). Original v1 8.4.20 compile.stderr; original process/compiler channels remain distinct.
- `v2-tcl8.4-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/receipt.json). Original v2 8.4.20 receipt.json; original process/compiler channels remain distinct.
- `v2-tcl8.4-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/stdout.tsv). Original v2 8.4.20 stdout.tsv; original process/compiler channels remain distinct.
- `v2-tcl8.4-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/stderr). Original v2 8.4.20 stderr; original process/compiler channels remain distinct.
- `v2-tcl8.4-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.4.20/compile.stderr). Original v2 8.4.20 compile.stderr; original process/compiler channels remain distinct.
- `v3-tcl8.4-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/receipt.json). Original v3 8.4.20 receipt.json; original process/compiler channels remain distinct.
- `v3-tcl8.4-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/stdout.tsv). Original v3 8.4.20 stdout.tsv; original process/compiler channels remain distinct.
- `v3-tcl8.4-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/stderr). Original v3 8.4.20 stderr; original process/compiler channels remain distinct.
- `v3-tcl8.4-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.4.20/compile.stderr). Original v3 8.4.20 compile.stderr; original process/compiler channels remain distinct.
- `v1-tcl8.5-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/receipt.json). Original v1 8.5.19 receipt.json; original process/compiler channels remain distinct.
- `v1-tcl8.5-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/stdout.tsv). Original v1 8.5.19 stdout.tsv; original process/compiler channels remain distinct.
- `v1-tcl8.5-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/stderr). Original v1 8.5.19 stderr; original process/compiler channels remain distinct.
- `v1-tcl8.5-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.5.19/compile.stderr). Original v1 8.5.19 compile.stderr; original process/compiler channels remain distinct.
- `v2-tcl8.5-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/receipt.json). Original v2 8.5.19 receipt.json; original process/compiler channels remain distinct.
- `v2-tcl8.5-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/stdout.tsv). Original v2 8.5.19 stdout.tsv; original process/compiler channels remain distinct.
- `v2-tcl8.5-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/stderr). Original v2 8.5.19 stderr; original process/compiler channels remain distinct.
- `v2-tcl8.5-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.5.19/compile.stderr). Original v2 8.5.19 compile.stderr; original process/compiler channels remain distinct.
- `v3-tcl8.5-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/receipt.json). Original v3 8.5.19 receipt.json; original process/compiler channels remain distinct.
- `v3-tcl8.5-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/stdout.tsv). Original v3 8.5.19 stdout.tsv; original process/compiler channels remain distinct.
- `v3-tcl8.5-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/stderr). Original v3 8.5.19 stderr; original process/compiler channels remain distinct.
- `v3-tcl8.5-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.5.19/compile.stderr). Original v3 8.5.19 compile.stderr; original process/compiler channels remain distinct.
- `v1-tcl8.6-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/receipt.json). Original v1 8.6.18 receipt.json; original process/compiler channels remain distinct.
- `v1-tcl8.6-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/stdout.tsv). Original v1 8.6.18 stdout.tsv; original process/compiler channels remain distinct.
- `v1-tcl8.6-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/stderr). Original v1 8.6.18 stderr; original process/compiler channels remain distinct.
- `v1-tcl8.6-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/8.6.18/compile.stderr). Original v1 8.6.18 compile.stderr; original process/compiler channels remain distinct.
- `v2-tcl8.6-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/receipt.json). Original v2 8.6.18 receipt.json; original process/compiler channels remain distinct.
- `v2-tcl8.6-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/stdout.tsv). Original v2 8.6.18 stdout.tsv; original process/compiler channels remain distinct.
- `v2-tcl8.6-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/stderr). Original v2 8.6.18 stderr; original process/compiler channels remain distinct.
- `v2-tcl8.6-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/8.6.18/compile.stderr). Original v2 8.6.18 compile.stderr; original process/compiler channels remain distinct.
- `v3-tcl8.6-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/receipt.json). Original v3 8.6.18 receipt.json; original process/compiler channels remain distinct.
- `v3-tcl8.6-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/stdout.tsv). Original v3 8.6.18 stdout.tsv; original process/compiler channels remain distinct.
- `v3-tcl8.6-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/stderr). Original v3 8.6.18 stderr; original process/compiler channels remain distinct.
- `v3-tcl8.6-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/8.6.18/compile.stderr). Original v3 8.6.18 compile.stderr; original process/compiler channels remain distinct.
- `v1-tcl9.0-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/receipt.json). Original v1 9.0.4 receipt.json; original process/compiler channels remain distinct.
- `v1-tcl9.0-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/stdout.tsv). Original v1 9.0.4 stdout.tsv; original process/compiler channels remain distinct.
- `v1-tcl9.0-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/stderr). Original v1 9.0.4 stderr; original process/compiler channels remain distinct.
- `v1-tcl9.0-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.0.4/compile.stderr). Original v1 9.0.4 compile.stderr; original process/compiler channels remain distinct.
- `v2-tcl9.0-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/receipt.json). Original v2 9.0.4 receipt.json; original process/compiler channels remain distinct.
- `v2-tcl9.0-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr). Original v2 9.0.4 compile.stderr; original process/compiler channels remain distinct.
- `v3-tcl9.0-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/receipt.json). Original v3 9.0.4 receipt.json; original process/compiler channels remain distinct.
- `v3-tcl9.0-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/stdout.tsv). Original v3 9.0.4 stdout.tsv; original process/compiler channels remain distinct.
- `v3-tcl9.0-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/stderr). Original v3 9.0.4 stderr; original process/compiler channels remain distinct.
- `v3-tcl9.0-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.0.4/compile.stderr). Original v3 9.0.4 compile.stderr; original process/compiler channels remain distinct.
- `v1-tcl9.1-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/receipt.json). Original v1 9.1.0 receipt.json; original process/compiler channels remain distinct.
- `v1-tcl9.1-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/stdout.tsv). Original v1 9.1.0 stdout.tsv; original process/compiler channels remain distinct.
- `v1-tcl9.1-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/stderr). Original v1 9.1.0 stderr; original process/compiler channels remain distinct.
- `v1-tcl9.1-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/9.1.0/compile.stderr). Original v1 9.1.0 compile.stderr; original process/compiler channels remain distinct.
- `v2-tcl9.1-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/receipt.json). Original v2 9.1.0 receipt.json; original process/compiler channels remain distinct.
- `v2-tcl9.1-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/stdout.tsv). Original v2 9.1.0 stdout.tsv; original process/compiler channels remain distinct.
- `v2-tcl9.1-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/stderr). Original v2 9.1.0 stderr; original process/compiler channels remain distinct.
- `v2-tcl9.1-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.1.0/compile.stderr). Original v2 9.1.0 compile.stderr; original process/compiler channels remain distinct.
- `v3-tcl9.1-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/receipt.json). Original v3 9.1.0 receipt.json; original process/compiler channels remain distinct.
- `v3-tcl9.1-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/stdout.tsv). Original v3 9.1.0 stdout.tsv; original process/compiler channels remain distinct.
- `v3-tcl9.1-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/stderr). Original v3 9.1.0 stderr; original process/compiler channels remain distinct.
- `v3-tcl9.1-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/compile.stderr). Original v3 9.1.0 compile.stderr; original process/compiler channels remain distinct.
- `v1-jim-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/receipt.json). Original v1 jim receipt.json; original process/compiler channels remain distinct.
- `v1-jim-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/stdout.tsv). Original v1 jim stdout.tsv; original process/compiler channels remain distinct.
- `v1-jim-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/stderr). Original v1 jim stderr; original process/compiler channels remain distinct.
- `v1-jim-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1/jim/compile.stderr). Original v1 jim compile.stderr; original process/compiler channels remain distinct.
- `v2-jim-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/receipt.json). Original v2 jim receipt.json; original process/compiler channels remain distinct.
- `v2-jim-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/stdout.tsv). Original v2 jim stdout.tsv; original process/compiler channels remain distinct.
- `v2-jim-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/stderr). Original v2 jim stderr; original process/compiler channels remain distinct.
- `v2-jim-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/jim/compile.stderr). Original v2 jim compile.stderr; original process/compiler channels remain distinct.
- `v3-jim-receipt-json` (provider): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/receipt.json). Original v3 jim receipt.json; original process/compiler channels remain distinct.
- `v3-jim-stdout-tsv` (observation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/stdout.tsv). Original v3 jim stdout.tsv; original process/compiler channels remain distinct.
- `v3-jim-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/stderr). Original v3 jim stderr; original process/compiler channels remain distinct.
- `v3-jim-compile-stderr` (limitation): [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/jim/compile.stderr). Original v3 jim compile.stderr; original process/compiler channels remain distinct.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Immutable original runners preserve absolute capture/output paths and source/library/header/Makefile digests. No executable is installed in the fixture; rerun must select the same independently provisioned native provider and a new output directory. Private fields require configured AC_FLAGS. Successful process0 is not an optimiser/runtime pass. V2 C9.0 failed compilation, with no native result rows.
