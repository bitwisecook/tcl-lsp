# Original hosted namespace and cleanup evidence bounds

Original-program cleanup proves only its retained ownership plan and measured
checks. It does not establish total interpreter cleanup or locate a procedure
whose expected qualified name is missing. No unproven fallback name is safe
to delete on the strength of these receipts.

In [exact1/records.json](nh20261012exact1/records.json), `computed-qualified`
evaluates the byte-identical original prefix containing
`namespace eval ::nh {proc emit ...}`. Its final command reports missing
`::nh::emit` in CLI, iApp implementation and iCall. The ownership guards report
execution namespace `::script` in CLI and `::` in iApp/iCall; cleanup returns
`ownership-cleanup ... remaining {}`. Those checks concern the planned
`::nh` namespace and frame variables `ns` and `head`. They do not inspect
`::emit`, `::script::emit` or another fallback owner.

The direct namespace controls in [cap2/records.json](nh20261012cap2/records.json)
(`namespace.eval`, JSON pointers `/2`, `/19`, `/223`) and
[APLcap1/records.json](nh20261012aplcap1/records.json) (`/2`) return the requested
run-owned namespace from `namespace current`. Their dynamic-procedure controls
also return `TARGET`. These results prove those particular operations; they
do not locate the original nested procedure definitions or demonstrate a
general namespace-owner mechanism. The missing original target alone could
reflect an absent definition or a different definition owner.

In [exact2/records.json](nh20261012exact2/records.json), `finite-factory` returns
`red blue` from the original `::nh::red` and `::nh::blue` calls in all three TMM
contexts that reached it. This is a measured context difference from the
hosted original factory failures, without attributing an implementation cause.
Its cleanup plan removes only names whose pre-existing absence was proved.
Existing `::forward` and frame-variable collisions remain explicit unreached
guards; those names were not acquired or removed.

The exact-program materialisation and raw-source digests are retained in each
sealed run's records, reachability review where applicable, and MANIFEST.sha256.
Their measured outer cleanup completion 0 and `remaining {}` values establish
the listed checks only. An independent read-only surface capture is needed
to locate any definitions outside those planned names or assess remaining
interpreter state. The current evidence does not justify repairing or deleting
such unowned names.

| Evidence bundle | records.json SHA-256 | Selected JSON pointers |
| --- | --- | --- |
| [nh20261012exact1/records.json](nh20261012exact1/records.json) | `7635625d658d6d823bd806fe1a869b65388f1e26f359f3ae4941155e6e24c7da` | `/2` iapp-implementation; `/47` icall; `/92` tmsh-cli-script |
| [nh20261012exact2/records.json](nh20261012exact2/records.json) | `cb2edaf2bdcbb1f4d17571ae54b46a03860fb737e19c2863b49e681ca86f91ba` | `/20` tmm-RULE_INIT; `/200` tmm-procedure; `/440` tmm-traffic-event |
| [nh20261012cap2/records.json](nh20261012cap2/records.json) | `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` | `/2` iapp-implementation; `/19` icall; `/223` tmsh-cli-script |
| [nh20261012aplcap1/records.json](nh20261012aplcap1/records.json) | `141d74bc12631a488c5fa964751c7d9ea9cdc9573f969a84372a2b8e862781e1` | `/2` iapp-apl |

A separately guarded unique-name [definition-owner control](../../../f5/answers/nh20261012nsowner1/README.md) now measures the owner difference directly. In that control, tmsh CLI (`/28`), iApp implementation (`/0`) and iCall (`/1`) report the requested namespace from inside `namespace eval`, yet catalogue/body/call receipts locate the new procedure globally. Stock Tcl and all measured TMM identities place it in the requested namespace. The control guards all requested/caller/global candidates and proves every exact candidate absent after cleanup. Its records SHA-256 is `33dc654ae53a29598bf5982542a4a4d582707afe0db10a0e88b508c5b23e6211`.

This establishes the unique control's definition placement and cleanup bounds. It does not inspect the original `::emit`, `::red`, `::blue` or other previous slots, so it does not retroactively establish where the original programs left definitions or that those prior interpreters were entirely cleaned. The original-program remaining-state limitation above remains.

The later [unique-name namespace-owner control](../../../f5/answers/nh20261012nsowner1/README.md)
and [actual APL callback](../../../f5/answers/nh20261012aplnsowner1/ANSWERS.md)
inspect all newly authored candidate slots and preserve their own cleanup
receipts. Those bounded results distinguish reported eval namespace from an
observed command owner, without locating earlier original-program definitions
or changing the cleanup limits above.
