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
            "ltm_dns_dnssec_zone",
            module="ltm",
            object_types=("dns dnssec zone",),
        ),
        header_types=(("ltm", "dns dnssec zone"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ds-algorithm", value_type="enum", enum_values=("sha1", "sha256", "sha384")
            ),
            BigipPropertySpec(
                name="ds-algorithms",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="secure", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="indicate-authenticated",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="keys", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="nsec3-algorithm", value_type="string"),
            BigipPropertySpec(name="nsec3-iterations", value_type="integer"),
            BigipPropertySpec(
                name="publish-cds-cdnskey", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
