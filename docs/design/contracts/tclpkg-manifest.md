# tclpkg manifest contracts

## Symptom

The ``tclpkg.tcl`` manifest fails to parse, or an unexpected command runs
during evaluation.

## Decision rules / contracts

1. Evaluated in ``TclInterp(safe=True, safe_whitelist=MANIFEST_DIRECTIVES)``.
2. 13 whitelisted directives: ``package``, ``version``, ``description``,
   ``license``, ``author``, ``homepage``, ``tcl``, ``require``,
   ``dev-require``, ``replace``, ``exclude``, ``provides``, ``entry``.
3. Any non-whitelisted command raises ``TclError`` at the INVOKE level.
4. ``package`` and ``version`` are required; all others are optional.
5. ``version`` must match semver 2.0 (with Tcl-style ``a1``/``b2``/``rc1``).
6. ``tcl`` constraint defaults to ``>=8.6`` when omitted.
7. ``require``/``dev-require`` accept ``name minver ?-source URL?``.
8. ``replace`` overrides at root level only; transitive replace is ignored.
9. Duplicate ``package`` directive raises immediately.
10. The same package in both ``require`` and ``dev-require`` is rejected.

## File-path anchors

- ``tooling/tclpkg/manifest.py`` — ``ManifestAST``, ``load_manifest_text()``, ``_build_directives()``
- ``tooling/vm/interp.py:102`` — ``TclInterp.__init__(safe=, safe_whitelist=)``
- ``tooling/vm/interp.py:530`` — safe-mode check in ``_invoke_inner``

## Test anchors

- ``tests/tooling/tclpkg/test_manifest.py`` — 29 tests covering all directives and refusals
