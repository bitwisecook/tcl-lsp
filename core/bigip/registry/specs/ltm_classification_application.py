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
            "ltm_classification_application",
            module="ltm",
            object_types=("classification application",),
        ),
        header_types=(("ltm", "classification application"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="application-id", value_type="integer"),
            BigipPropertySpec(
                name="category",
                value_type="reference",
                references=(
                    "ltm_classification_category",
                    "ltm_classification_stats_url_category",
                    "ltm_classification_url_category",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_bot_defense_anomaly_category",
                    "security_bot_defense_signature_category",
                    "security_dos_bot_signature_category",
                    "security_firewall_ipi_category_info",
                    "security_ip_intelligence_blacklist_category",
                    "security_scrubber_dwbl_scrubber_category_stats",
                    "sys_url_db_url_category",
                ),
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
