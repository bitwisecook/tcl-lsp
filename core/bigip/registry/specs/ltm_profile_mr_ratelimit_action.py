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
            "ltm_profile_mr_ratelimit_action",
            module="ltm",
            object_types=("profile mr-ratelimit-action",),
        ),
        header_types=(("ltm", "profile mr-ratelimit-action"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_mr_ratelimit_action",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="priority-1",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "none",
                    "delay-25",
                    "delay-50",
                    "delay-100",
                    "return-25",
                    "return-50",
                    "return-100",
                    "drop-25",
                    "drop-50",
                    "drop-100",
                ),
            ),
            BigipPropertySpec(
                name="priority-2",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "none",
                    "delay-25",
                    "delay-50",
                    "delay-100",
                    "return-25",
                    "return-50",
                    "return-100",
                    "drop-25",
                    "drop-50",
                    "drop-100",
                ),
            ),
            BigipPropertySpec(
                name="priority-3",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "none",
                    "delay-25",
                    "delay-50",
                    "delay-100",
                    "return-25",
                    "return-50",
                    "return-100",
                    "drop-25",
                    "drop-50",
                    "drop-100",
                ),
            ),
            BigipPropertySpec(
                name="priority-4",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "none",
                    "delay-25",
                    "delay-50",
                    "delay-100",
                    "return-25",
                    "return-50",
                    "return-100",
                    "drop-25",
                    "drop-50",
                    "drop-100",
                ),
            ),
        ),
    )
