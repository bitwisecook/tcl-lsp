# Original command-query surfaces

[The portable input](input-portable.tcl) runs exact original Tcl source in
fresh C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and latest retained Jim interpreters.
Each provider under `portable/` retains33 observations:25 command queries,
six explicit setup results and two current-namespace presentations. All six
complete with process status0 and empty stderr. Queried commands are actual
source declarations; returned names are compared as output bytes, never parsed
back into namespace or command identity.

The helper uses `uplevel 1` to preserve the calling namespace frame and catches
each observed operation independently. A successful outer process includes
intentionally caught command/feature errors; it cannot certify setup success.

| Surface | Real observations |
| --- | --- |
| Bare/full command query, rooted names, root/current shadowing | Full results remain provider-specific raw bytes |
| `-c`, `-comman` options | Accepted C8.5–9.1; rejected C8.4 and latest Jim |
| Namespace path order, current-over-path and path-over-root | Setup accepted C8.5–9.1; all three path setup attempts explicitly fail C8.4/latestJim |
| `interp alias` query | Setup accepted C5; latestJim rejects this exact interp alias form and subsequent query reports absence |
| Export/import command query | Exact setup succeeds all6; reports imported name in target namespace |
| Single-colon root command versus its displayed absolute spelling | C reports the relative colon-name query; its displayed spelling does not provide an inverse identity operation; Jim reports absence for this exact source declaration/query form |
| Nested `a:/b` and `a/:b` displays | All6 display equal namespace strings; C8.6–9.1 command queries report equal marker display bytes and display requery misses; latestJim display requery succeeds; C8.4/8.5 marker query itself misses |

The C8.4/8.5 colon-context misses are complete source observations, including
procedure publication and entered frame behavior. They do not prove a pure
command-table resolver must miss an independently allocated authentic slot.
The source declarations and command queries need separate measurements before
those outcomes can become narrower adapter expectations. Equal displayed
names supply no identity equivalence or deduplication rule.

[The initial input](input.tcl) additionally requested `info library` for
identity. All five C captures succeeded and retain matching library identity.
Jim rejected that identity operation before any query. Those original captures
remain intact. The portable input changes only the identity header to
`info patchlevel`; all33 C observation bytes match their initial C captures
exactly. The failed Jim startup supplies no command-query result.

Each provenance identifies exact input/binary/init hashes and interpreter
identity. The C binaries/init hashes match retained original-source receipts.
Jim is commit `5bac7c99ad65864c87da513e22e2f01703fa4e03`, reported
`0.84-9-g5bac7c9`; upstream latest verification is the separately identified
parent measurement. This experiment does not use Tcltest, measure F5 or VM
behavior, establish native C object allocation equivalence, or derive identity
from output. Compiler selection is separately measured under
`../namespace-which-compiler/`.
