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
            "ltm_dns_dnssec_key",
            module="ltm",
            object_types=("dns dnssec key",),
        ),
        header_types=(("ltm", "dns dnssec key"),),
        properties=(
            BigipPropertySpec(
                name="algorithm",
                value_type="enum",
                enum_values=(
                    "rsasha1",
                    "rsasha256",
                    "rsasha512",
                    "ecdsap256sha256",
                    "ecdsap384sha384",
                ),
            ),
            BigipPropertySpec(
                name="bitwidth", value_type="enum", enum_values=("512", "1024", "2048", "4096")
            ),
            BigipPropertySpec(name="certificate-file", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="expiration-period", value_type="integer"),
            BigipPropertySpec(name="generation", value_type="string"),
            BigipPropertySpec(name="expiration", value_type="string", in_sections=("generation",)),
            BigipPropertySpec(name="rollover", value_type="string", in_sections=("generation",)),
            BigipPropertySpec(name="key-file", value_type="string", in_sections=("generation",)),
            BigipPropertySpec(
                name="key-type",
                value_type="enum",
                in_sections=("generation",),
                enum_values=("ksk", "zsk"),
            ),
            BigipPropertySpec(
                name="rollover-period", value_type="integer", in_sections=("generation",)
            ),
            BigipPropertySpec(
                name="signature-pub-period", value_type="integer", in_sections=("generation",)
            ),
            BigipPropertySpec(
                name="signature-valid-period", value_type="integer", in_sections=("generation",)
            ),
            BigipPropertySpec(name="ttl", value_type="integer", in_sections=("generation",)),
            BigipPropertySpec(
                name="use-fips",
                value_type="enum",
                in_sections=("generation",),
                allow_none=True,
                enum_values=("external", "internal", "none"),
            ),
        ),
    )
