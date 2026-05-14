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
            "ltm_profile_classification",
            module="ltm",
            object_types=("profile classification",),
        ),
        header_types=(("ltm", "profile classification"),),
        properties=(
            BigipPropertySpec(name="app-detection", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_classification",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="irule-event", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(name="log-publisher", value_type="string"),
            BigipPropertySpec(name="preset", value_type="string"),
            BigipPropertySpec(name="urlcat", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(
                name="avr-publisher",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="avr-stat-collect",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="log-unclassified-domain",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
