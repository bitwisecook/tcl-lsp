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
            "ltm_persistence_universal",
            module="ltm",
            object_types=("persistence universal",),
        ),
        header_types=(("ltm", "persistence universal"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_persistence_universal",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
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
                name="method",
                value_type="enum",
                enum_values=("hash", "insert", "passive", "rewrite"),
            ),
            BigipPropertySpec(
                name="mirror", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="rule", value_type="reference", allow_none=True, references=("ltm_rule",)
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
