"""Tests for the VM's safe-mode whitelist support.

The safe-mode feature lets callers construct a ``TclInterp`` that only
permits a fixed set of commands.  It powers sandboxed manifest evaluation
for the tclpkg package manager (see ``tclpkg/manifest.py``) and is also
exposed through ``interp create -safe`` / ``interp issafe``.
"""

from __future__ import annotations

import pytest

from tooling.vm.interp import TclInterp
from tooling.vm.types import TclError


class TestSafeModeWhitelist:
    """Tests for the ``safe`` / ``safe_whitelist`` constructor arguments."""

    def test_default_interp_is_not_safe(self) -> None:
        interp = TclInterp()
        assert interp._is_safe is False
        assert interp._safe_whitelist is None

    def test_safe_with_whitelist_allows_listed_invoked_commands(self) -> None:
        """Non-specialised commands on the whitelist dispatch normally."""
        interp = TclInterp(safe=True, safe_whitelist=frozenset({"puts"}))
        # ``puts`` reaches ``_invoke_inner`` because it is not a specialised
        # bytecode opcode; the whitelist allows it through and it returns "".
        assert interp.eval("puts -nonewline {}").value == ""

    def test_safe_refuses_non_whitelisted_invoked_command(self) -> None:
        interp = TclInterp(safe=True, safe_whitelist=frozenset({"set"}))
        with pytest.raises(TclError, match="not permitted in safe mode"):
            interp.eval("puts hello")

    def test_safe_without_whitelist_rejects_invoked_commands(self) -> None:
        """Without a whitelist, every dispatch-routed command is refused.

        Note: specialised bytecode opcodes such as ``set`` compile to
        direct VM instructions and bypass command dispatch entirely — the
        safe-mode gate is an INVOKE-level filter, matching how tclpkg
        manifest directives are implemented.  See ``tclpkg/manifest.py``.
        """
        interp = TclInterp(safe=True)
        with pytest.raises(TclError, match="not permitted in safe mode"):
            interp.eval("puts hello")

    def test_safe_whitelist_can_include_custom_commands(self) -> None:
        from tooling.vm.types import TclResult

        calls: list[list[str]] = []

        def _directive(interp: TclInterp, args: list[str]) -> TclResult:
            calls.append(args)
            return TclResult(value="")

        interp = TclInterp(safe=True, safe_whitelist=frozenset({"package", "set"}))
        interp.register_command("package", _directive)
        interp.eval("package myapp")
        assert calls == [["myapp"]]

    def test_safe_whitelist_rejects_exec(self) -> None:
        interp = TclInterp(safe=True, safe_whitelist=frozenset({"set"}))
        with pytest.raises(TclError, match="exec"):
            interp.eval("exec ls /")

    def test_safe_whitelist_rejects_open(self) -> None:
        interp = TclInterp(safe=True, safe_whitelist=frozenset({"set"}))
        with pytest.raises(TclError, match="open"):
            interp.eval("open /etc/passwd")

    def test_safe_whitelist_rejects_source(self) -> None:
        interp = TclInterp(safe=True, safe_whitelist=frozenset({"set"}))
        with pytest.raises(TclError, match="source"):
            interp.eval("source /tmp/x.tcl")


class TestInterpIssafeCommand:
    """Tests for the ``interp issafe`` reflection command."""

    def test_issafe_reports_false_for_default_interp(self) -> None:
        interp = TclInterp()
        result = interp.eval("interp issafe")
        assert result.value == "0"

    def test_issafe_reports_true_for_safe_interp(self) -> None:
        interp = TclInterp(
            safe=True,
            safe_whitelist=frozenset({"interp"}),
        )
        result = interp.eval("interp issafe")
        assert result.value == "1"


class TestInterpCreateSafe:
    """Tests for ``interp create -safe`` producing a safe child interp."""

    def test_create_safe_child_has_empty_whitelist(self) -> None:
        interp = TclInterp()
        interp.eval("interp create -safe child")
        # The child exists in the registry — parent can ask if it's safe.
        result = interp.eval("interp issafe child")
        assert result.value == "1"

    def test_create_non_safe_child_is_not_safe(self) -> None:
        interp = TclInterp()
        interp.eval("interp create kid")
        result = interp.eval("interp issafe kid")
        assert result.value == "0"
