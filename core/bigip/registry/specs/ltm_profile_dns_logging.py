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
            "ltm_profile_dns_logging",
            module="ltm",
            object_types=("profile dns-logging",),
        ),
        header_types=(("ltm", "profile dns-logging"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="enable-query-logging", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="enable-response-logging", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="include-complete-answer", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="include-query-id", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="include-source", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="include-timestamp", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="include-view", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="log-publisher", value_type="string"),
        ),
    )
