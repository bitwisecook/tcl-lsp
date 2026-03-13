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
            "ltm_profile_georedundancy",
            module="ltm",
            object_types=("profile georedundancy",),
        ),
        header_types=(("ltm", "profile georedundancy"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_georedundancy",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="group-id", value_type="integer"),
            BigipPropertySpec(name="local-site-id", value_type="integer"),
            BigipPropertySpec(name="prefix", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="metadata-refresh-interval-ms", value_type="integer"),
            BigipPropertySpec(name="message-send-max-retries", value_type="integer"),
            BigipPropertySpec(name="message-timeout-ms", value_type="integer"),
            BigipPropertySpec(name="read-broker-list", value_type="string"),
            BigipPropertySpec(name="remote-site-id", value_type="integer"),
            BigipPropertySpec(name="transport-name", value_type="string"),
            BigipPropertySpec(name="write-broker-list", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
