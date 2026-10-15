# Descendants created during namespace teardown

Each version directory retains the exact original SDK `stdout.tsv` bytes from
[the canonical command and unset callback capture](../../../../../experiments/name-handling/results/original-dying-descendant-command-sdk-20261014/README.md).
That archive records the probe source, release provenance and verified hashes.

The Runtime and VM tests use four fixed targets in actual unset callbacks and
command deletion callbacks for each of Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and
9.1.0. They compare the created namespace name, its relationship to the retained
original ancestor, child and scalar visibility, contents, final cleanup and
exactly one callback.
The Runtime command case uses an actual object-command delete procedure; the VM
case uses its actual native command retirement callback.

The tests deliberately leave the public visibility of the dying parent outside
this acceptance scope: the capture records Tcl 8.4 returning true while the
later releases return false. The existing-child case proves original-ancestor
placement and post-creation reentry identity; the probe did not retain the
child's earlier allocation pointer. These results do not establish Jim, F5 or
full lifecycle compatibility.
