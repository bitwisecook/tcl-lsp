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
            "ltm_profile_icap",
            module="ltm",
            object_types=("profile icap",),
        ),
        header_types=(("ltm", "profile icap"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_icap",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="header-from", value_type="string"),
            BigipPropertySpec(
                name="host",
                value_type="string",
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="preview-length", value_type="integer"),
            BigipPropertySpec(name="referer", value_type="string"),
            BigipPropertySpec(name="uri", value_type="string"),
            BigipPropertySpec(name="user-agent", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
