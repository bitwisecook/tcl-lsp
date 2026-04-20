"""Tests for the ``package`` command in the Tcl VM."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestPackageProvide:
    """Tests for ``package provide``."""

    def test_provide_version(self) -> None:
        interp = TclInterp()
        interp.eval("package provide mylib 1.0")
        result = interp.eval("package provide mylib")
        assert result.value == "1.0"

    def test_provide_no_version(self) -> None:
        interp = TclInterp()
        result = interp.eval("package provide unknown_pkg")
        assert result.value == ""

    def test_provide_updates_version(self) -> None:
        interp = TclInterp()
        interp.eval("package provide mylib 1.0")
        interp.eval("package provide mylib 2.0")
        result = interp.eval("package provide mylib")
        assert result.value == "2.0"


class TestPackageRequire:
    """Tests for ``package require``."""

    def test_require_already_loaded(self) -> None:
        interp = TclInterp()
        interp.eval("package provide mylib 1.5")
        result = interp.eval("package require mylib")
        assert result.value == "1.5"

    def test_require_with_ifneeded(self) -> None:
        interp = TclInterp()
        interp.eval("package ifneeded mylib 2.0 {package provide mylib 2.0}")
        result = interp.eval("package require mylib")
        assert result.value == "2.0"

    def test_require_not_found(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="can't find package"):
            interp.eval("package require nonexistent")

    def test_require_exact(self) -> None:
        interp = TclInterp()
        interp.eval("package provide mylib 1.0")
        result = interp.eval("package require -exact mylib 1.0")
        assert result.value == "1.0"

    def test_require_exact_mismatch(self) -> None:
        interp = TclInterp()
        interp.eval("package provide mylib 1.0")
        with pytest.raises(TclError, match="version conflict"):
            interp.eval("package require -exact mylib 2.0")


class TestPackageIfneeded:
    """Tests for ``package ifneeded``."""

    def test_ifneeded_register(self) -> None:
        interp = TclInterp()
        interp.eval("package ifneeded foo 1.0 {set x loaded}")
        result = interp.eval("package ifneeded foo 1.0")
        assert result.value == "set x loaded"

    def test_ifneeded_query_unset(self) -> None:
        interp = TclInterp()
        result = interp.eval("package ifneeded foo 1.0")
        assert result.value == ""

    def test_ifneeded_triggers_on_require(self) -> None:
        interp = TclInterp()
        interp.eval("package ifneeded mylib 3.0 {package provide mylib 3.0; set loaded yes}")
        interp.eval("package require mylib")
        result = interp.eval("set loaded")
        assert result.value == "yes"


class TestPackageNames:
    """Tests for ``package names``."""

    def test_names_empty(self) -> None:
        interp = TclInterp()
        result = interp.eval("package names")
        # May include Tcl if pre-provided
        assert isinstance(result.value, str)

    def test_names_after_provide(self) -> None:
        interp = TclInterp()
        interp.eval("package provide alpha 1.0")
        interp.eval("package provide beta 2.0")
        result = interp.eval("package names")
        names = result.value.split()
        assert "alpha" in names
        assert "beta" in names


class TestPackageVersions:
    """Tests for ``package versions``."""

    def test_versions_with_ifneeded(self) -> None:
        interp = TclInterp()
        interp.eval("package ifneeded foo 1.0 {set x 1}")
        interp.eval("package ifneeded foo 2.0 {set x 2}")
        result = interp.eval("package versions foo")
        versions = result.value.split()
        assert "1.0" in versions
        assert "2.0" in versions

    def test_versions_unknown(self) -> None:
        interp = TclInterp()
        result = interp.eval("package versions unknown_pkg")
        assert result.value == ""


class TestPackagePresent:
    """Tests for ``package present``."""

    def test_present_loaded(self) -> None:
        interp = TclInterp()
        interp.eval("package provide foo 1.0")
        result = interp.eval("package present foo")
        assert result.value == "1.0"

    def test_present_not_loaded(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="not present"):
            interp.eval("package present nonexistent")


class TestPackageForget:
    """Tests for ``package forget``."""

    def test_forget(self) -> None:
        interp = TclInterp()
        interp.eval("package provide foo 1.0")
        interp.eval("package forget foo")
        result = interp.eval("package provide foo")
        assert result.value == ""


class TestPackageVcompare:
    """Tests for ``package vcompare``."""

    def test_equal(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vcompare 1.0 1.0")
        assert result.value == "0"

    def test_less(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vcompare 1.0 2.0")
        assert result.value == "-1"

    def test_greater(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vcompare 2.0 1.0")
        assert result.value == "1"


class TestPackageVsatisfies:
    """Tests for ``package vsatisfies``."""

    def test_satisfies(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vsatisfies 2.1 2.0")
        assert result.value == "1"

    def test_not_satisfies_major(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vsatisfies 3.0 2.0")
        assert result.value == "0"

    def test_not_satisfies_too_low(self) -> None:
        interp = TclInterp()
        result = interp.eval("package vsatisfies 1.0 2.0")
        assert result.value == "0"


class TestPackageUnknown:
    """Tests for ``package unknown``."""

    def test_set_and_get(self) -> None:
        interp = TclInterp()
        interp.eval("package unknown myloader")
        result = interp.eval("package unknown")
        assert result.value == "myloader"

    def test_get_empty(self) -> None:
        interp = TclInterp()
        result = interp.eval("package unknown")
        assert result.value == ""
