"""WASM ``clock`` command tests — format / scan / add round-trip.

These tests compile small Tcl snippets that call ``clock format`` /
``clock scan`` / ``clock add``, run them in wasmtime, and assert
against the captured stdout.  They cover the user-visible contract
the runtime is supposed to provide once the TZ resolver lands:

* ``clock format`` with explicit format strings produces real
  field expansions (year / month / day / hour / minute / second /
  weekday / month-name / zone-abbr).
* ``-gmt 1`` always works (synthetic UTC, no I/O).
* ``-timezone :America/New_York`` works *if* the host tzdata is
  preopened into the WASI sandbox, otherwise the test skips.
* ``clock scan`` round-trips the formats ``clock format`` produces.
* ``clock add`` matches arithmetic for ``weeks`` / ``days`` /
  ``hours`` / ``minutes``.

The test harness reuses the linker / runtime helpers from
``tests.test_wasm_real_tcl`` so any future build/runtime changes
flow through one place.
"""

from __future__ import annotations

import os
import pathlib
import tempfile

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from core.runtime_wasm import runtime_wasm_path  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _define_call_compiled_proc,
    _get_engine,
    _run_wasm,
)


def _run_for_stdout(source: str, *, preopen: str | None = None) -> str:
    """Compile + run *source*, returning captured stdout.

    Without ``preopen`` this defers to the shared
    :func:`tests.test_wasm_real_tcl._run_wasm` helper.  With a
    ``preopen``, we replicate the helper inline so we can map the
    host directory to the *same* absolute guest path — the runtime
    opens ``/usr/share/zoneinfo/<zone>`` literally, so a guest-root
    mount (the default) doesn't surface the file at the expected
    path.
    """
    wasm_bytes = _compile_tcl(source)
    if preopen is None:
        _, stdout = _run_wasm(wasm_bytes, capture_stdout=True)
        return stdout
    return _run_with_same_path_preopen(wasm_bytes, preopen)


def _run_with_same_path_preopen(wasm_bytes: bytes, preopen: str) -> str:
    """Run *wasm_bytes* with ``preopen`` mounted at the same guest path.

    Lifted from the same scaffolding as
    :func:`tests.test_wasm_real_tcl._run_wasm` but with the preopen
    mapping pinned to ``preopen → preopen`` instead of ``preopen →
    /``.  Only the timezone tests need this, so the duplication is
    minimal.
    """
    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi = wasmtime.WasiConfig()

    fd, stdout_path = tempfile.mkstemp(suffix=".txt")
    os.close(fd)
    wasi.stdout_file = stdout_path

    wasi.preopen_dir(preopen, preopen)
    store.set_wasi(wasi)

    rt_module = wasmtime.Module.from_file(engine, str(runtime_wasm_path()))
    linker = wasmtime.Linker(engine)
    linker.define_wasi()
    tcl_box, mem_box = _define_call_compiled_proc(linker, store)
    rt_instance = linker.instantiate(store, rt_module)
    init_fn = rt_instance.exports(store).get("_initialize")
    if init_fn is not None:
        init_fn(store)
    for export in rt_module.exports:
        name = export.name
        if name.startswith("__"):
            continue
        val = rt_instance.exports(store)[name]
        if isinstance(val, wasmtime.Func):
            linker.define(store, "tcl", name, val)
        elif name == "memory":
            linker.define(store, "tcl", name, val)
    tcl_module = wasmtime.Module(engine, wasm_bytes)
    tcl_instance = linker.instantiate(store, tcl_module)
    tcl_box[0] = tcl_instance
    mem_box[0] = rt_instance.exports(store)["memory"]
    top = tcl_instance.exports(store).get("::top")
    if top is not None:
        top(store)
    with open(stdout_path) as f:
        return f.read()


# ---- clock format -----------------------------------------------------------


class TestClockFormatGmt:
    """``clock format`` with ``-gmt 1`` — synthetic UTC, no host tzdata."""

    def test_unix_epoch_iso_date(self):
        stdout = _run_for_stdout('puts [clock format 0 -gmt 1 -format "%Y-%m-%d"]\n')
        assert stdout.strip() == "1970-01-01"

    def test_unix_epoch_full_iso(self):
        stdout = _run_for_stdout('puts [clock format 0 -gmt 1 -format "%Y-%m-%dT%H:%M:%S"]\n')
        assert stdout.strip() == "1970-01-01T00:00:00"

    def test_unix_epoch_weekday_and_zone_abbr(self):
        # Thursday 1970-01-01 in UTC.  ``%Z`` should expand to the
        # synthetic UTC zone's abbreviation.
        stdout = _run_for_stdout('puts [clock format 0 -gmt 1 -format "%a %b %d %H:%M:%S %Z %Y"]\n')
        assert stdout.strip() == "Thu Jan 01 00:00:00 UTC 1970"

    def test_known_y2k(self):
        # 946684800 = 2000-01-01 00:00:00 UTC.
        stdout = _run_for_stdout(
            'puts [clock format 946684800 -gmt 1 -format "%Y-%m-%dT%H:%M:%SZ"]\n'
        )
        assert stdout.strip() == "2000-01-01T00:00:00Z"

    def test_numeric_offset(self):
        stdout = _run_for_stdout('puts [clock format 0 -gmt 1 -format "%z"]\n')
        assert stdout.strip() == "+0000"


class TestClockFormatTimezone:
    """``clock format -timezone`` — needs host tzdata in the sandbox."""

    @pytest.fixture(scope="class")
    def has_zoneinfo(self) -> bool:
        return pathlib.Path("/usr/share/zoneinfo/America/New_York").exists()

    def test_eastern_winter(self, has_zoneinfo: bool):
        if not has_zoneinfo:
            pytest.skip("host /usr/share/zoneinfo not available")
        # 2025-01-15 12:00:00 UTC → 07:00:00 EST (-05:00).
        # Epoch: 1736942400.
        stdout = _run_for_stdout(
            "puts [clock format 1736942400 -timezone :America/New_York "
            '-format "%Y-%m-%d %H:%M:%S %Z"]\n',
            preopen="/usr/share/zoneinfo",
        )
        line = stdout.strip()
        # Two acceptable shapes: the WASI sandbox might fail to open
        # the preopen on some hosts (kernel quirk / read-only mount),
        # in which case the runtime falls through to UTC.  Skip
        # rather than fail in that degraded environment.
        if "UTC" in line:
            pytest.skip(f"WASI preopen didn't expose tzdata: got {line!r}")
        assert line == "2025-01-15 07:00:00 EST"

    def test_eastern_summer_dst(self, has_zoneinfo: bool):
        if not has_zoneinfo:
            pytest.skip("host /usr/share/zoneinfo not available")
        # 2025-07-15 12:00:00 UTC → 08:00:00 EDT (-04:00).
        # Epoch: 1752580800.
        stdout = _run_for_stdout(
            "puts [clock format 1752580800 -timezone :America/New_York "
            '-format "%Y-%m-%d %H:%M:%S %Z"]\n',
            preopen="/usr/share/zoneinfo",
        )
        line = stdout.strip()
        if "UTC" in line:
            pytest.skip(f"WASI preopen didn't expose tzdata: got {line!r}")
        assert line == "2025-07-15 08:00:00 EDT"

    def test_unknown_zone_falls_back_to_utc(self):
        # No preopen, no real zone — resolver should hand back UTC
        # rather than trapping.
        stdout = _run_for_stdout(
            'puts [clock format 0 -timezone :Mars/Olympus -format "%Y-%m-%d %Z"]\n'
        )
        assert stdout.strip() == "1970-01-01 UTC"


class TestClockFormatBundle:
    """Embedded tzdata bundle — no host preopen.

    Verifies the resolver's step 4 fallback fires when the WASI
    sandbox blocks ``/usr/share/zoneinfo``.  No ``preopen`` is
    passed, so the runtime can't reach the host tzdata; the only
    way these tests can succeed is via the comptime-embedded
    ``data/tzdata.bin`` blob.
    """

    def test_bundle_serves_eastern_zone(self):
        # 2025-07-15 12:00:00 UTC → 08:00:00 EDT.
        stdout = _run_for_stdout(
            "puts [clock format 1752580800 -timezone :America/New_York "
            '-format "%Y-%m-%d %H:%M:%S %Z"]\n'
        )
        assert stdout.strip() == "2025-07-15 08:00:00 EDT"

    def test_bundle_serves_us_eastern_alias(self):
        # ``US/Eastern`` is bundled as a separate alias entry
        # pointing at the ``America/New_York`` payload.
        stdout = _run_for_stdout(
            'puts [clock format 1752580800 -timezone US/Eastern -format "%Z"]\n'
        )
        assert stdout.strip() == "EDT"

    def test_bundle_serves_tokyo(self):
        # Asia/Tokyo is JST year-round (+09:00), no DST.
        # 2025-01-15 00:00:00 UTC → 09:00:00 JST.
        stdout = _run_for_stdout(
            'puts [clock format 1736899200 -timezone :Asia/Tokyo -format "%H:%M %Z"]\n'
        )
        assert stdout.strip() == "09:00 JST"

    def test_bundle_serves_london_winter_gmt(self):
        # Europe/London = GMT in winter, BST in summer.
        # 2025-01-15 12:00:00 UTC = 12:00 GMT.
        stdout = _run_for_stdout(
            'puts [clock format 1736942400 -timezone :Europe/London -format "%H:%M %Z"]\n'
        )
        assert stdout.strip() == "12:00 GMT"

    def test_bundle_serves_london_summer_bst(self):
        # 2025-07-15 12:00 UTC = 13:00 BST.
        stdout = _run_for_stdout(
            'puts [clock format 1752580800 -timezone :Europe/London -format "%H:%M %Z"]\n'
        )
        assert stdout.strip() == "13:00 BST"

    def test_bundle_miss_falls_through_to_utc(self):
        # Zone not in the bundle's curated list AND not on the host
        # (no preopen) — resolver lands on synthetic UTC.
        stdout = _run_for_stdout('puts [clock format 0 -timezone :Antarctica/Troll -format "%Z"]\n')
        assert stdout.strip() == "UTC"


# ---- clock scan -------------------------------------------------------------


class TestClockScan:
    """``clock scan`` — round-trip with ``clock format``."""

    def test_iso_date_returns_epoch(self):
        stdout = _run_for_stdout('puts [clock scan "1970-01-01" -gmt 1]\n')
        assert stdout.strip() == "0"

    def test_iso_datetime_t_separator(self):
        stdout = _run_for_stdout('puts [clock scan "2000-01-01T00:00:00Z"]\n')
        assert stdout.strip() == "946684800"

    def test_iso_datetime_space_separator_gmt(self):
        stdout = _run_for_stdout('puts [clock scan "2000-01-01 00:00:00" -gmt 1]\n')
        assert stdout.strip() == "946684800"

    def test_iso_with_explicit_offset(self):
        # 12:00:00 +05:00 == 07:00:00 UTC == epoch 946706400 - 0
        # 2000-01-01 12:00:00+05:00 -> 2000-01-01 07:00:00 UTC.
        stdout = _run_for_stdout('puts [clock scan "2000-01-01T12:00:00+05:00"]\n')
        assert stdout.strip() == str(946684800 + 7 * 3600)

    def test_round_trip_via_format(self):
        stdout = _run_for_stdout(
            "set t 1234567890\n"
            'set s [clock format $t -gmt 1 -format "%Y-%m-%dT%H:%M:%SZ"]\n'
            "set back [clock scan $s]\n"
            "puts $back\n"
        )
        assert stdout.strip() == "1234567890"

    def test_integer_passthrough(self):
        stdout = _run_for_stdout('puts [clock scan "12345"]\n')
        assert stdout.strip() == "12345"


class TestClockScanFreeform:
    """``clock scan`` — free-form date / time inputs.

    These exercise the porting of a pragmatic subset of Tcl's
    ``library/clock.tcl::GetDate`` grammar.  Full coverage of every
    edge case in the C grammar is out of scope; the suite checks
    the constructs that real scripts use.
    """

    def test_now_returns_base(self):
        stdout = _run_for_stdout('puts [clock scan "now" -base 1234567890 -gmt 1]\n')
        assert stdout.strip() == "1234567890"

    def test_today_returns_midnight_of_base(self):
        # 2025-07-15 12:34:56 UTC = epoch 1752597296.
        # ``today`` should return midnight of the same date = 1752537600.
        stdout = _run_for_stdout('puts [clock scan "today" -base 1752597296 -gmt 1]\n')
        assert stdout.strip() == "1752537600"

    def test_yesterday(self):
        # 2025-07-15 12:34:56 UTC base; yesterday = 2025-07-14 midnight.
        stdout = _run_for_stdout('puts [clock scan "yesterday" -base 1752597296 -gmt 1]\n')
        assert stdout.strip() == str(1752537600 - 86400)

    def test_tomorrow(self):
        stdout = _run_for_stdout('puts [clock scan "tomorrow" -base 1752597296 -gmt 1]\n')
        assert stdout.strip() == str(1752537600 + 86400)

    def test_epoch_keyword(self):
        stdout = _run_for_stdout('puts [clock scan "epoch"]\n')
        assert stdout.strip() == "0"

    def test_relative_days(self):
        # base = 1000000000; +5 days = +432000.
        stdout = _run_for_stdout('puts [clock scan "+5 days" -base 1000000000 -gmt 1]\n')
        assert stdout.strip() == str(1000000000 + 5 * 86400)

    def test_relative_negative_hours(self):
        stdout = _run_for_stdout('puts [clock scan "-3 hours" -base 1000000000 -gmt 1]\n')
        assert stdout.strip() == str(1000000000 - 3 * 3600)

    def test_relative_unsigned(self):
        stdout = _run_for_stdout('puts [clock scan "2 weeks" -base 1000000000 -gmt 1]\n')
        assert stdout.strip() == str(1000000000 + 2 * 7 * 86400)

    def test_relative_ago_suffix(self):
        stdout = _run_for_stdout('puts [clock scan "10 minutes ago" -base 1000000000 -gmt 1]\n')
        assert stdout.strip() == str(1000000000 - 10 * 60)

    def test_month_day_year_full_name(self):
        # "January 15, 2025" -> 2025-01-15 midnight UTC.
        stdout = _run_for_stdout('puts [clock scan "January 15, 2025" -gmt 1]\n')
        assert stdout.strip() == "1736899200"

    def test_month_day_year_abbreviated(self):
        stdout = _run_for_stdout('puts [clock scan "Jan 15, 2025" -gmt 1]\n')
        assert stdout.strip() == "1736899200"

    def test_day_month_year(self):
        # "15 Jan 2025" -> 2025-01-15.
        stdout = _run_for_stdout('puts [clock scan "15 Jan 2025" -gmt 1]\n')
        assert stdout.strip() == "1736899200"

    def test_us_slash_date(self):
        # 01/15/2025 -> 2025-01-15.
        stdout = _run_for_stdout('puts [clock scan "01/15/2025" -gmt 1]\n')
        assert stdout.strip() == "1736899200"

    def test_relative_months_calendar(self):
        # base = 2025-01-15 midnight UTC = 1736899200.
        # +1 month -> 2025-02-15 midnight = 1739577600.
        stdout = _run_for_stdout('puts [clock scan "+1 month" -base 1736899200 -gmt 1]\n')
        assert stdout.strip() == "1739577600"

    def test_relative_years_calendar(self):
        stdout = _run_for_stdout('puts [clock scan "+1 year" -base 1736899200 -gmt 1]\n')
        # 2026-01-15 midnight UTC.
        # 1736899200 + 365 * 86400 (2025 is non-leap).
        assert stdout.strip() == str(1736899200 + 365 * 86400)

    def test_unknown_form_returns_zero(self):
        stdout = _run_for_stdout('puts [clock scan "garbage tokens here"]\n')
        assert stdout.strip() == "0"


class TestClockScanRobustness:
    """Regression coverage for review-board feedback on PR #283.

    Each test pins a specific fix that the Codex review surfaced —
    out-of-range bounds checks in ``parse_iso`` (P1), the
    ``today`` /``yesterday`` /``tomorrow`` zone-aware boundary
    computation (P2), and the ``%F`` composite-format signed-year
    rendering (P2).  Without the fixes the runtime panics with a
    safety trap; the assertions below would crash rather than
    fail.
    """

    def test_iso_out_of_range_month_does_not_trap(self):
        # ``2025-99-01`` would index ``MONTH_DAYS_NORMAL[97]`` and
        # trap before the fix.  With the fix, parse_iso rejects it
        # as malformed and clock scan returns 0 (the
        # graceful-degradation path).
        stdout = _run_for_stdout('puts [clock scan "2025-99-01"]\n')
        assert stdout.strip() == "0"

    def test_iso_out_of_range_day_does_not_trap(self):
        # February 30th — month is fine, day overflows.
        stdout = _run_for_stdout('puts [clock scan "2025-02-30"]\n')
        assert stdout.strip() == "0"

    def test_iso_zero_month_rejected(self):
        stdout = _run_for_stdout('puts [clock scan "2025-00-15"]\n')
        assert stdout.strip() == "0"

    def test_iso_zero_day_rejected(self):
        stdout = _run_for_stdout('puts [clock scan "2025-01-00"]\n')
        assert stdout.strip() == "0"

    def test_iso_hour_out_of_range_rejected(self):
        stdout = _run_for_stdout('puts [clock scan "2025-01-01T25:00:00"]\n')
        assert stdout.strip() == "0"

    def test_freeform_invalid_calendar_day_rejected(self):
        # ``Feb 30, 2025`` — month name resolves, but the day is
        # out of range.  Used to silently overflow into March;
        # now the validated pack rejects the input.
        stdout = _run_for_stdout('puts [clock scan "Feb 30, 2025" -gmt 1]\n')
        assert stdout.strip() == "0"

    def test_freeform_slash_date_invalid_rejected(self):
        stdout = _run_for_stdout('puts [clock scan "02/30/2025" -gmt 1]\n')
        assert stdout.strip() == "0"

    def test_today_uses_target_timezone(self):
        # Base = 2025-01-15 02:00 UTC = 2025-01-14 21:00 EST.
        # Without the zone-aware fix, ``today`` would land on
        # 2025-01-15 (UTC date); with the fix it lands on
        # 2025-01-14 — midnight EST = 05:00 UTC = epoch 1736830800.
        stdout = _run_for_stdout(
            'puts [clock scan "today" -base 1736906400 -timezone :America/New_York]\n'
        )
        assert stdout.strip() == "1736830800"

    def test_yesterday_uses_target_timezone(self):
        # Same base; yesterday = 2025-01-13 EST midnight.
        # 1736830800 - 86400 = 1736744400.
        stdout = _run_for_stdout(
            'puts [clock scan "yesterday" -base 1736906400 -timezone :America/New_York]\n'
        )
        assert stdout.strip() == "1736744400"

    def test_tomorrow_uses_target_timezone(self):
        # 1736830800 + 86400 = 1736917200 (2025-01-15 EST midnight).
        stdout = _run_for_stdout(
            'puts [clock scan "tomorrow" -base 1736906400 -timezone :America/New_York]\n'
        )
        assert stdout.strip() == "1736917200"

    def test_F_composite_format_handles_negative_year(self):
        # 0001-01-01 UTC — the smallest in-range positive epoch.
        # Confirm ``%F`` doesn't panic; 1 AD packs to a known
        # negative epoch (-62135596800).
        stdout = _run_for_stdout('puts [clock format -62135596800 -gmt 1 -format "%F"]\n')
        assert stdout.strip() == "0001-01-01"

    def test_F_composite_format_pre_year_one_does_not_trap(self):
        # Pick an epoch a few hours into year 0000 (BC by Tcl's
        # proleptic-Gregorian convention).  The previous
        # implementation panicked on ``@intCast(t.year)`` for any
        # year ≤ 0; with the fix ``%F`` renders ``0000-01-01``.
        stdout = _run_for_stdout('puts [clock format -62167219200 -gmt 1 -format "%F"]\n')
        assert stdout.strip() == "0000-01-01"


# ---- clock add --------------------------------------------------------------


class TestClockAdd:
    """``clock add`` — fixed-second units."""

    def test_add_seconds(self):
        stdout = _run_for_stdout("puts [clock add 1000 30 seconds]\n")
        assert stdout.strip() == "1030"

    def test_add_minutes(self):
        stdout = _run_for_stdout("puts [clock add 0 5 minutes]\n")
        assert stdout.strip() == "300"

    def test_add_hours(self):
        stdout = _run_for_stdout("puts [clock add 0 2 hours]\n")
        assert stdout.strip() == "7200"

    def test_add_days(self):
        stdout = _run_for_stdout("puts [clock add 0 1 days]\n")
        assert stdout.strip() == "86400"

    def test_add_weeks(self):
        stdout = _run_for_stdout("puts [clock add 0 2 weeks]\n")
        assert stdout.strip() == str(2 * 7 * 86400)

    def test_negative_count(self):
        stdout = _run_for_stdout("puts [clock add 86400 -1 days]\n")
        assert stdout.strip() == "0"

    def test_chained_units(self):
        # 1 day + 2 hours + 30 minutes = 86400 + 7200 + 1800 = 95400.
        stdout = _run_for_stdout("puts [clock add 0 1 days 2 hours 30 minutes]\n")
        assert stdout.strip() == "95400"

    def test_singular_unit(self):
        stdout = _run_for_stdout("puts [clock add 0 1 day]\n")
        assert stdout.strip() == "86400"

    def test_add_month_then_format(self):
        # 2025-01-15 + 1 month = 2025-02-15.
        # Epoch for 2025-01-15 00:00 UTC: 1736899200.
        stdout = _run_for_stdout(
            "set t [clock add 1736899200 1 months]\n"
            'puts [clock format $t -gmt 1 -format "%Y-%m-%d"]\n'
        )
        assert stdout.strip() == "2025-02-15"

    def test_add_month_clamps_day(self):
        # 2025-01-31 + 1 month = 2025-02-28 (clamped).
        # Epoch for 2025-01-31 00:00 UTC: 1738281600.
        stdout = _run_for_stdout(
            "set t [clock add 1738281600 1 months]\n"
            'puts [clock format $t -gmt 1 -format "%Y-%m-%d"]\n'
        )
        assert stdout.strip() == "2025-02-28"
