"""Front-end-behaviour contract tests for the command and graph registries.

These tests drive the real ``tcl`` and ``f5`` front-ends (plus the
temporary ``scripts/registry/dump.py`` dumper) and assert their output
against the language-agnostic golden fixtures under
``tests/baselines/registry/``.  The fixtures are the registry shape
contract; a Rust front-end re-implementing the ``command-info`` /
``event-info`` verbs — and a registry dumper of its own — is validated
against the same files.
"""
