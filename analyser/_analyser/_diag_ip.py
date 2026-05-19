from __future__ import annotations

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.cfg import CFGFunction
from compiler.core_analyses import FunctionAnalysis, LatticeKind

from ..semantic_model import Diagnostic, Range, Severity


class _AnalyserDiagIPMixin(_Base):
    """W124 diagnostics: invalid IP address literals."""

    def _emit_invalid_ip_diagnostics(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
    ) -> None:
        """W124: flag invalid IP address literals discovered via SCCP constants.

        Walks all SSA constants looking for IPv4/IPv6 candidates via regex,
        validates them with ``ip_utils.parse_ip()``, and emits diagnostics at
        the definition site.  Use sites get ``related_ranges`` links.
        """
        from shared.ip_utils import IPV6_RE, parse_ip

        if analysis.def_use_chains is None:
            return

        _DOTTED_QUAD_LOOSE = re.compile(r"\b(\d{1,4})\.(\d{1,4})\.(\d{1,4})\.(\d{1,4})\b")
        # Track emitted definition offsets to avoid duplicates when
        # multiple SSA values point at the same assignment.
        seen_offsets: set[int] = set()

        for key, lattice_val in analysis.values.items():
            if lattice_val.kind is not LatticeKind.CONST:
                continue
            val = lattice_val.value
            if not isinstance(val, str):
                continue

            # --- IPv4 candidates ---
            for m in _DOTTED_QUAD_LOOSE.finditer(val):
                # Skip version-number patterns: preceded by '/'
                if m.start() > 0 and val[m.start() - 1] == "/":
                    continue
                octets_str = [m.group(i) for i in range(1, 5)]
                msg: str | None = None
                severity = Severity.ERROR
                for i, octet_s in enumerate(octets_str):
                    v = int(octet_s)
                    if v > 255:
                        msg = (
                            f"IPv4 octet {i + 1} ({octet_s}) exceeds 255 "
                            "— this is not a valid IP address."
                        )
                        break
                    if (
                        len(octet_s) > 1
                        and octet_s[0] == "0"
                        and all(c in "01234567" for c in octet_s)
                    ):
                        msg = (
                            f"IPv4 octet {i + 1} ({octet_s}) has a leading zero "
                            "— may be interpreted as octal in some contexts."
                        )
                        severity = Severity.WARNING
                        break
                if msg is not None:
                    self._emit_ip_diag(cfg, analysis, key, msg, severity, seen_offsets)
                    break  # one diagnostic per SSA value

            # --- IPv6 candidates ---
            for m in IPV6_RE.finditer(val):
                candidate = m.group(1)
                if parse_ip(candidate) is None:
                    self._emit_ip_diag(
                        cfg,
                        analysis,
                        key,
                        f"Invalid IPv6 address '{candidate}'.",
                        Severity.ERROR,
                        seen_offsets,
                    )
                    break  # one diagnostic per SSA value

    def _emit_ip_diag(
        self,
        cfg: CFGFunction,
        analysis: FunctionAnalysis,
        key: tuple[str, int],
        message: str,
        severity: Severity,
        seen_offsets: set[int],
    ) -> None:
        """Emit a W124 diagnostic at the definition site with related-info on uses."""
        assert analysis.def_use_chains is not None
        var_name, version = key
        chain = analysis.def_use_chains.chain_for(var_name, version)
        if chain is None:
            return

        # Find definition range
        def_site = chain.definition
        block = cfg.blocks.get(def_site.block)
        if block is None:
            return
        if def_site.statement_index < 0 or def_site.statement_index >= len(block.statements):
            return
        stmt = block.statements[def_site.statement_index]
        def_range = getattr(stmt, "range", None)
        if def_range is None:
            return

        # Skip if we already emitted a W124 for this exact source location
        if def_range.start.offset in seen_offsets:
            return
        seen_offsets.add(def_range.start.offset)

        # Collect use-site ranges for related information
        related: list[tuple[Range, str]] = []
        for use in chain.uses:
            use_block = cfg.blocks.get(use.block)
            if use_block is None:
                continue
            if 0 <= use.statement_index < len(use_block.statements):
                use_stmt = use_block.statements[use.statement_index]
                use_range = getattr(use_stmt, "range", None)
                if use_range is not None:
                    related.append((use_range, f"'{var_name}' used here"))

        self.result.diagnostics.append(
            Diagnostic(
                range=def_range,
                message=message,
                severity=severity,
                code="W124",
                related_ranges=tuple(related),
            )
        )
