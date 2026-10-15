# Original Jim namespace-info projection observations

Question: what actual List/member headers and bytes does the original namespace-info
helper return when a once-mutated canonical helper routes one internal query through
another genuine original namespace-info invocation? Which observations distinguish
the actual lmap/string-cat transformations from mutable implementations that return
an existing input or a new literal?

`probe.c` contains every exact input, evaluated in a fresh original interpreter per
case after the original core commands and `Jim_InitStaticExtensions`. The canonical
wrapper delegates to the original command except for one rooted pattern. It causes
two actual original helper projections for GENUINE, and empty actual lists for EMPTY.
The mutable lmap returns its actual input List. The mutable string-cat returns the
actual source literal `FORGED` and delegates other operations.

`capture01` records a fresh 2026-10-15 13:49 UTC upstream check matching clean local
`5bac7c99ad65864c87da513e22e2f01703fa4e03`, compiler/link-library/configuration/source
hashes, the compiled observer hash, every invocation, stdout/stderr/completion, and
247 unchanged provider inputs. The original namespace suite passes 79/79 using the
exact provider's original `tests/namespace.test`, `tests/testing.tcl` and
`tests/tcltest.tcl`. The primary C probe does not use Tcltest.

The 15 execution rows are copied byte-exactly to the Runtime observations fixture.
Its setup Tcl fixture is the concatenation of the actual C setup string literals;
this derivation does not evaluate a replacement provider. HEADER is observed before
any result getter/conversion. MEMBER records primary/residency/native references
before reaching that member's original String getter. RESULT then reaches the
original result String getter. No extra C observer native references are held.

GENUINE returns one fresh String member with four leading colons because the two
original helper lmaps each prepend `::`. EMPTY remains a stringless List. Returning
lmap's input instead preserves the actual table key and its extra table reference;
FORGED_CAT returns the actual user-procedure Source literal. These are observations,
not proposed rules for reconstructing command identity from returned text.

Runtime positive controls compare GENUINE/EMPTY header and value observations.
Separate software authority controls deliberately reject missing real transformation
producers for the two mutable cases; they do not claim conformance to the original
interpreter's successful mutable-case completion. No Rust tests have been run by
this experiment. Local relative dictionary projections, arbitrary callback data
flows, malformed inputs and other implementation permutations remain outside this
capture.

Reproduce with the repository-provisioned original provider:

```
python3 experiments/name-handling/results/jim-namespace-info-projection/capture.py \
  --sdk-dir /workspace/tcl-lsp/tmp/jimtcl-oracle \
  --out /tmp/fresh-jim-namespace-info-projection
```

Run `verify.py --current-provider` for retained checksum and pairing checks without
executing an interpreter. Generated observer binaries are rebuildable; their exact
hash remains recorded when the executable itself is absent.
