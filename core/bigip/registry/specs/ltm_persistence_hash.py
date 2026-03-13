from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_persistence_hash",
            module="ltm",
            object_types=("persistence hash",),
        ),
        header_types=(("ltm", "persistence hash"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_persistence_hash",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hash-algorithm", value_type="enum", enum_values=("carp", "default")
            ),
            BigipPropertySpec(name="hash-buffer-limit", value_type="integer"),
            BigipPropertySpec(name="hash-end-pattern", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="hash-length", value_type="integer"),
            BigipPropertySpec(name="hash-offset", value_type="integer"),
            BigipPropertySpec(name="hash-start-pattern", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="match-across-pools", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-services", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-virtuals", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="mirror", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="rule", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
