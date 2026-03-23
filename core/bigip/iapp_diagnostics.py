"""Cross-file diagnostics for iApp presentation ↔ implementation.

Validates that:
- Implementation variables reference fields that exist in the presentation
- Presentation fields are actually used by the implementation
- ``#include`` directives point to files that exist

Diagnostic codes
================

- **IAPP7001** (WARNING): implementation references variable not defined
  in presentation
- **IAPP7002** (HINT): presentation field never referenced in implementation
- **IAPP7003** (WARNING): ``#include`` file not found
"""

from __future__ import annotations

from ..analysis.semantic_model import Diagnostic, Range, Severity
from ..parsing.tokens import SourcePosition
from .apl_model import AplModel
from .iapp_vars import IappVarRef


def validate_iapp_presentation(
    apl_model: AplModel,
    impl_var_refs: list[IappVarRef] | None = None,
) -> list[Diagnostic]:
    """Validate an APL presentation model, optionally cross-checked against
    implementation variable references.

    Parameters
    ----------
    apl_model:
        Parsed APL presentation model.
    impl_var_refs:
        Variable references extracted from a sibling implementation file.
        If ``None``, only presentation-local checks are performed.
    """
    diagnostics: list[Diagnostic] = []

    # --- IAPP7003: #include not found ---
    for inc in apl_model.includes:
        if not inc.resolved:
            diagnostics.append(
                Diagnostic(
                    message=f'#include file not found: "{inc.path}"',
                    range=Range(
                        # offset=0 is a placeholder; AplInclude only tracks line number
                        start=SourcePosition(line=inc.line, character=0, offset=0),
                        end=SourcePosition(line=inc.line, character=0, offset=0),
                    ),
                    severity=Severity.WARNING,
                    code="IAPP7003",
                )
            )

    if impl_var_refs is None:
        return diagnostics

    # Build set of referenced APL names from implementation
    referenced_apl_names: set[str] = set()
    for ref in impl_var_refs:
        referenced_apl_names.add(ref.apl_name)

    # NOTE: IAPP7001 (undefined variable) is emitted by
    # validate_iapp_implementation() so that the diagnostic is positioned
    # in the implementation file.  We intentionally skip it here to avoid
    # duplicate diagnostics.

    # --- IAPP7002: unused presentation field ---
    for qname, apl_field in apl_model.all_fields.items():
        if qname not in referenced_apl_names:
            # Only report for non-message fields (messages are display-only)
            if apl_field.field_type == "message":
                continue
            diagnostics.append(
                Diagnostic(
                    message=(
                        f"Presentation field '{qname}' is never referenced "
                        f"in the implementation (expected $::{apl_field.qualified_name.replace('.', '__')})"
                    ),
                    range=apl_field.range,
                    severity=Severity.HINT,
                    code="IAPP7002",
                )
            )

    return diagnostics


def validate_iapp_implementation(
    impl_var_refs: list[IappVarRef],
    apl_model: AplModel | None = None,
) -> list[Diagnostic]:
    """Validate iApp implementation references against presentation.

    This is called for the implementation file and produces diagnostics
    positioned in the implementation source.

    Parameters
    ----------
    impl_var_refs:
        Variable references from the implementation Tcl source.
    apl_model:
        Parsed APL presentation model from a sibling file.
        If ``None``, no cross-validation is performed.
    """
    if apl_model is None:
        return []

    diagnostics: list[Diagnostic] = []

    for ref in impl_var_refs:
        if ref.apl_name not in apl_model.all_fields:
            diagnostics.append(
                Diagnostic(
                    message=(
                        f"Variable ${ref.tcl_name} references presentation "
                        f"field '{ref.apl_name}' which is not defined in the presentation"
                    ),
                    range=ref.range,
                    severity=Severity.WARNING,
                    code="IAPP7001",
                )
            )

    return diagnostics
