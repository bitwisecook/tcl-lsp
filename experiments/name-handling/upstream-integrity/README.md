# Original C Tcl suite inputs

These receipts retain upstream Git blob identities for `generic/tcl.h` and
every tracked file under `library/` and `tests/` in the manifest-selected Tcl
source tag. The tag-to-commit identities and non-truncated recursive inventories
were read from the linked upstream GitHub API URLs on 13 October 2026. The
immutable commit identifies the original bytes independently of a local source
tree or capture. Release and tag selection remains owned by
`rust/tcl-dialect/data/reference-toolchains.tsv`.

`run_suites.py` compares source bytes with these original Git blob hashes before
starting a capture, then records separate SHA-256 hashes of the validated local
inputs. It checks the original init, Tcltest, package index, suite definitions,
orchestration and support files. Generated build files outside the inventory
do not change the original-input check. A changed reference tag requires a new
reviewed upstream inventory; hashes taken from the mutable local tree cannot
replace it.

The runner requires the matching source tree's built `unix/tclsh`, selects its
shared-library directory and original script library, and measures its exact
release, patchlevel, library path and original Tcltest startup before capturing
suites. A backend-constraint overlay remains an explicitly labelled execution
channel. Its results do not establish execution with unchanged upstream
constraints. Old captures retain their recorded methods and scope.

Run deterministic admission controls with:

```sh
python3 -m unittest discover -s experiments/name-handling/tests
```
