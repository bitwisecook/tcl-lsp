"""End-to-end tests that drive a real, packaged LSP server over JSON-RPC.

See ``conftest.py`` for the shared fixtures.  The design goal is one
long-lived server per session that many tests query — the same surface an
editor (or the Rust port) talks to, so this doubles as a cross-implementation
conformance suite.
"""
