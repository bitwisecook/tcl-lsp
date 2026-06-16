"""``f5 encrypt-secrets`` / ``f5 decrypt-secrets`` — master-key crypto
for the credential-bearing values in a bigip.conf / SCF.

Both verbs walk every secret-bearing property (``passphrase``,
``password``, ``secret``, ``shared-secret``, …) and run its value
through the F5 unit master key supplied by the user — the base64 key
that ``f5mku -K`` prints on the device.  ``encrypt-secrets`` wraps
clear-text values in the ``$M$<salt>$<base64>`` envelope BIG-IP stores;
``decrypt-secrets`` recovers the clear text from that envelope.  Values
already in the target form are left untouched, so both verbs are
idempotent and safe to re-run.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.rewrite import rewrite_secrets
from tooling.f5 import f5mku
from tooling.f5.f5_remote.secret_input import SecretInputError, resolve_secret

from ._emit import add_format_arg, render_config
from ._paths import add_passphrase_args, provider_from_args, read_path
from ._registry import verb

_F5MKU_ENV = "F5MKU"
_KEY_PROMPT = "F5 MKU Key: "


def _add_key_args(p: argparse.ArgumentParser) -> None:
    """Add the shared master-key source flags."""
    group = p.add_argument_group("master key")
    group.add_argument(
        "-k",
        "--f5mku",
        metavar="KEY",
        help=(
            "base64 unit master key (the value `f5mku -K` prints on the "
            f"device).  Falls back to ${_F5MKU_ENV}, then a secure "
            "terminal prompt."
        ),
    )
    group.add_argument(
        "--f5mku-file",
        metavar="FILE",
        help="read the base64 master key from FILE (e.g. `f5mku -K > key.txt`).",
    )
    group.add_argument(
        "--no-key-prompt",
        action="store_true",
        help="never prompt on the terminal for the master key; require a flag or $F5MKU instead.",
    )


def _resolve_key(args: argparse.Namespace) -> str:
    """Resolve the master key from --f5mku / --f5mku-file / $F5MKU / prompt.

    The base64 key is whitespace-trimmed (``strip=True``) so a file or
    env value carrying a trailing newline — ``f5mku -K > key.txt`` — still
    works.
    """
    try:
        key = resolve_secret(
            explicit=args.f5mku,
            file=args.f5mku_file,
            env_var=_F5MKU_ENV,
            allow_prompt=not getattr(args, "no_key_prompt", False),
            prompt=_KEY_PROMPT,
            strip=True,
        )
    except SecretInputError as exc:
        raise ValueError(str(exc)) from exc
    if key:
        return key
    raise ValueError(
        f"no master key supplied; pass --f5mku KEY, --f5mku-file FILE, set ${_F5MKU_ENV}, "
        "or run in an interactive terminal to be prompted"
    )


@verb(
    "encrypt-secrets",
    aliases=("encrypt",),
    help="Encrypt clear-text secrets in a bigip.conf under the f5mku master key.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_encrypt(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Wrap every clear-text secret value (passphrase, password, "
        "secret, shared-secret, auth-password, priv-password, "
        "encrypted-password) in the `$M$<salt>$<base64>` envelope BIG-IP "
        "stores, using the unit master key.  Values already encrypted "
        "are left untouched, so the verb is idempotent."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 encrypt-secrets bigip.conf -k BHDLd0bbao1VlwpTk1sioQ== -o sealed.conf\n"
        "  f5mku -K > key.txt; f5 encrypt-secrets bigip.conf --f5mku-file key.txt\n"
        "  F5MKU=BHDLd0bbao1VlwpTk1sioQ== f5 encrypt-secrets bigip.conf\n"
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument(
        "-o", "--output", metavar="FILE", help="Write the result here (default: stdout)."
    )
    _add_key_args(p)
    p.add_argument(
        "--salt",
        metavar="XX",
        help=(
            "Force this two-character salt instead of a random one "
            "(deterministic output; mainly for testing)."
        ),
    )
    add_passphrase_args(p)
    add_format_arg(p, tmsh_default_verb="modify")
    p.set_defaults(handler=_run_encrypt)


@verb(
    "decrypt-secrets",
    aliases=("decrypt",),
    help="Decrypt `$M$...` secrets in a bigip.conf with the f5mku master key.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_decrypt(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Recover the clear text of every `$M$<salt>$<base64>` secret "
        "value (passphrase, password, secret, shared-secret, "
        "auth-password, priv-password, encrypted-password) using the "
        "unit master key.  Values that are not in the encrypted envelope "
        "are left untouched."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 decrypt-secrets bigip.conf -k BHDLd0bbao1VlwpTk1sioQ==\n"
        "  f5mku -K > key.txt; f5 decrypt-secrets bigip.conf --f5mku-file key.txt\n"
        "  F5MKU=BHDLd0bbao1VlwpTk1sioQ== f5 decrypt-secrets bigip.conf -o clear.conf\n"
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument(
        "-o", "--output", metavar="FILE", help="Write the result here (default: stdout)."
    )
    _add_key_args(p)
    add_passphrase_args(p)
    add_format_arg(p, tmsh_default_verb="modify")
    p.set_defaults(handler=_run_decrypt)


def _run_encrypt(args: argparse.Namespace) -> int:
    return _run(args, mode="encrypt")


def _run_decrypt(args: argparse.Namespace) -> int:
    return _run(args, mode="decrypt")


def _run(args: argparse.Namespace, *, mode: str) -> int:
    try:
        key = _resolve_key(args)
    except (ValueError, OSError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    # Fail fast on a bad key before touching the file.
    try:
        f5mku.encrypt("probe", key, salt="00")
    except f5mku.F5MkuError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        _, source = read_path(args.path, passphrase_provider=provider_from_args(args))
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if mode == "encrypt":
        salt = getattr(args, "salt", None)

        def transform(value: str) -> str | None:
            if f5mku.is_ciphertext(value):
                return None  # already encrypted
            return f5mku.encrypt(value, key, salt=salt)
    else:

        def transform(value: str) -> str | None:
            if not f5mku.is_ciphertext(value):
                return None  # not encrypted
            return f5mku.decrypt(value, key)

    try:
        report = rewrite_secrets(source, transform)
    except f5mku.F5MkuError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    rendered = render_config(
        report.new_source,
        fmt=args.output_format,
        tmsh_verb="modify",
        transaction=getattr(args, "output_transaction", False),
    )
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)

    verb_name = "encrypted" if mode == "encrypt" else "decrypted"
    print(
        f"{verb_name}: {report.transformed} secret(s); {report.skipped} left untouched",
        file=sys.stderr,
    )
    return 0
