"""UCS (BIG-IP backup archive) handling, including encrypted archives.

A UCS file is a gzip-compressed tar containing a snapshot of
``/config``.  This module provides the inverse of ``tmsh load sys ucs``:
extract one or more UCS archives and reassemble a single SCF (Single
Configuration File) text by concatenating the relevant ``bigip*.conf``
members in a deterministic order.

BIG-IP can also encrypt a UCS with a passphrase
(``tmsh save sys ucs <name> passphrase <pass>``).  Per F5 KB K5437 the
archive is then a GnuPG **symmetric** (passphrase) OpenPGP message whose
plaintext is the ordinary gzip tar.  BIG-IP uses 128-bit AES, but the
decryptor accepts AES-128/192/256 so archives produced with other GnuPG
settings still load.  We decrypt it two ways, in order:

1. Shell out to ``gpg``/``gpg2`` if present — exactly what BIG-IP itself
   uses.  The decrypted bytes are streamed back over a pipe and never
   touch disk.
2. Otherwise fall back to the dependency-free pure-Python decryptor in
   :mod:`._openpgp` (so the zipapp can decrypt a UCS on a host with no
   GnuPG, where the C-extension ``cryptography`` cannot be bundled).

Either way the decrypted UCS lives only in memory; :func:`ucs_to_scf`
feeds it straight to :mod:`tarfile` through an :class:`io.BytesIO`, so
the gzip tar is unpacked in RAM as well — a UCS routinely holds SSL
private keys, so it must never be written out in the clear.

The passphrase is taken (in order) from an explicit value, an
environment variable (default ``F5_UCS_PASSPHRASE``), or a secure
``getpass`` prompt on an interactive terminal.
"""

from __future__ import annotations

import atexit
import contextlib
import functools
import io
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from collections.abc import Iterator
from pathlib import Path
from typing import Callable

# Order matters: base must come first so partition declarations exist
# before objects that reference them.
_SCF_MEMBER_ORDER = (
    "config/bigip_base.conf",
    "config/bigip.conf",
    "config/bigip_gtm.conf",
    "config/bigip_user.conf",
    "config/bigip_script.conf",
)

#: Default environment variable consulted for a UCS decryption passphrase.
DEFAULT_PASSPHRASE_ENV = "F5_UCS_PASSPHRASE"

#: A zero-argument callable that yields the passphrase on demand.  Used so
#: the (possibly interactive) passphrase resolution happens lazily — only
#: when an encrypted archive is actually encountered.
PassphraseProvider = Callable[[], str]


class UcsPassphraseError(ValueError):
    """No passphrase was available to decrypt an encrypted UCS archive."""


class UcsDecryptionError(ValueError):
    """An encrypted UCS archive could not be decrypted (usually wrong pass)."""


def is_ucs_bytes(data: bytes) -> bool:
    """Return True if *data* looks like a plain UCS archive (gzip magic)."""
    return len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B


def is_pgp_bytes(data: bytes) -> bool:
    """Return True if *data* looks like an OpenPGP message (encrypted UCS).

    Recognises both ASCII-armored messages and the binary form BIG-IP
    actually emits: the first byte is an OpenPGP packet header whose tag
    is one that begins an encrypted message (a symmetric- or public-key
    session-key packet, or an encrypted-data packet).  An F5 encrypted
    UCS starts with a Symmetric-Key Encrypted Session Key packet
    (tag 3 → first byte ``0x8C``).
    """
    # Armored input may carry leading whitespace; inspect only a small
    # bounded prefix so large inputs are never copied.
    if data[:64].lstrip().startswith(b"-----BEGIN PGP"):
        return True
    if not data:
        return False
    first = data[0]
    if not first & 0x80:  # not an OpenPGP packet header
        return False
    tag = (first & 0x3F) if (first & 0x40) else ((first >> 2) & 0x0F)
    # 1 = PKESK, 3 = SKESK, 9 = SED, 18 = SEIPD.
    return tag in (1, 3, 9, 18)


# ---------------------------------------------------------------------------
# Passphrase resolution
# ---------------------------------------------------------------------------


def _stdin_is_a_tty() -> bool:
    try:
        return sys.stdin is not None and sys.stdin.isatty()
    except (ValueError, AttributeError):  # pragma: no cover - closed stdin
        return False


def resolve_passphrase(
    *,
    env_var: str | None = DEFAULT_PASSPHRASE_ENV,
    explicit: str | None = None,
    allow_prompt: bool = True,
    prompt: str = "Enter UCS passphrase: ",
) -> str:
    """Resolve a UCS passphrase from an explicit value, env var, or prompt.

    Resolution order: *explicit* (if non-empty) → ``$env_var`` (if set
    and non-empty) → a secure :func:`getpass.getpass` prompt when
    *allow_prompt* is true and stdin is an interactive terminal.  Raises
    :class:`UcsPassphraseError` when none of those yields a passphrase
    (e.g. non-interactive with the env var unset), so batch runs fail
    fast with a clear message instead of blocking on a prompt.
    """
    if explicit:
        return explicit
    if env_var:
        from_env = os.environ.get(env_var)
        if from_env:
            return from_env
    if allow_prompt and _stdin_is_a_tty():
        import getpass

        try:
            entered = getpass.getpass(prompt)
        except (EOFError, KeyboardInterrupt) as exc:  # pragma: no cover - tty only
            raise UcsPassphraseError("passphrase entry cancelled") from exc
        if entered:
            return entered
    raise UcsPassphraseError(
        "this UCS archive is encrypted and requires a passphrase; set the "
        f"{env_var} environment variable or run in an interactive terminal "
        "to be prompted"
        if env_var
        else "this UCS archive is encrypted and requires a passphrase"
    )


def make_passphrase_provider(
    *,
    env_var: str | None = DEFAULT_PASSPHRASE_ENV,
    explicit: str | None = None,
    allow_prompt: bool = True,
) -> PassphraseProvider:
    """Return a cached, lazy provider that resolves the passphrase once.

    The wrapped :func:`resolve_passphrase` call is deferred until the
    provider is first invoked (i.e. until an encrypted archive is hit)
    and the result is memoised, so processing several encrypted inputs
    in one run only prompts once.
    """
    cache: list[str] = []

    def provider() -> str:
        if not cache:
            cache.append(
                resolve_passphrase(env_var=env_var, explicit=explicit, allow_prompt=allow_prompt)
            )
        return cache[0]

    return provider


# ---------------------------------------------------------------------------
# GnuPG discovery + decryption (primary path)
# ---------------------------------------------------------------------------


@functools.lru_cache(maxsize=1)
def _resolve_gpg() -> tuple[str, bool] | None:
    """Return ``(gpg_path, supports_loopback)`` or None if no gpg is found."""
    for name in ("gpg", "gpg2"):
        path = shutil.which(name)
        if path is not None:
            return path, _gpg_supports_loopback(path)
    return None


def _gpg_supports_loopback(path: str) -> bool:
    """True if *path* is GnuPG ≥ 2.1 (needs ``--pinentry-mode loopback``).

    GnuPG 1.x reads ``--passphrase-fd`` directly; 2.1+ requires loopback
    pinentry mode to accept a passphrase on a file descriptor.
    """
    try:
        out = subprocess.run([path, "--version"], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):  # pragma: no cover - defensive
        return True
    match = re.search(r"(\d+)\.(\d+)\.(\d+)", out.stdout)
    if not match:
        return True
    major, minor = int(match.group(1)), int(match.group(2))
    return (major, minor) >= (2, 1)


def _gpg_install_hint() -> str:
    """A platform-appropriate one-liner on how to install GnuPG."""
    if sys.platform == "darwin":
        return "install GnuPG, e.g. `brew install gnupg`"
    if sys.platform.startswith("win"):
        return "install Gpg4win (https://www.gpg4win.org/) or `winget install GnuPG.Gpg4win`"
    if sys.platform.startswith("linux"):
        for tool, cmd in (
            ("apt-get", "apt install gnupg"),
            ("dnf", "dnf install gnupg2"),
            ("yum", "yum install gnupg2"),
            ("zypper", "zypper install gpg2"),
            ("pacman", "pacman -S gnupg"),
            ("apk", "apk add gnupg"),
            ("emerge", "emerge app-crypt/gnupg"),
        ):
            if shutil.which(tool):
                return f"install GnuPG, e.g. `{cmd}`"
        return "install GnuPG (the `gpg` command) with your platform's package manager"
    return "install GnuPG from https://gnupg.org/download/"


def _gpg_result(proc: subprocess.CompletedProcess[bytes]) -> bytes:
    if proc.returncode != 0:
        lines = proc.stderr.decode("utf-8", "replace").strip().splitlines()
        detail = lines[-1] if lines else f"gpg exited with status {proc.returncode}"
        raise UcsDecryptionError(f"failed to decrypt UCS archive: {detail}")
    return proc.stdout


@contextlib.contextmanager
def _staged_ciphertext(data: bytes) -> Iterator[str]:
    """Stage *encrypted* bytes in a private temp file, guaranteeing cleanup.

    Used only on platforms that can't pass the passphrase over an
    inherited file descriptor (Windows): gpg needs the ciphertext as a
    file argument there.  Only ciphertext — never decrypted plaintext —
    is written.  Removal is guaranteed on normal completion, on any
    exception, and (as a fallback) at interpreter exit via
    :mod:`atexit`.  A hard kill (SIGKILL) can still leave the file
    behind, so we announce its full path on stderr so it can be found
    and deleted manually if that ever happens.
    """
    fd, tmp = tempfile.mkstemp(prefix="f5ucs-", suffix=".pgp")
    removed = False

    def _remove() -> None:
        nonlocal removed
        if removed:
            return
        removed = True
        try:
            os.unlink(tmp)
        except FileNotFoundError:
            pass

    atexit.register(_remove)
    print(
        f"warning: staged the encrypted UCS in temporary file {tmp} — it will "
        "be deleted automatically; remove it manually if this process is "
        "killed before it can clean up.",
        file=sys.stderr,
    )
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(data)
        yield tmp
    finally:
        _remove()
        atexit.unregister(_remove)


def _gpg_decrypt(gpg_info: tuple[str, bool], data: bytes, passphrase: str) -> bytes:
    """Decrypt *data* with GnuPG, returning the plaintext bytes in memory.

    The decrypted output is read from gpg's stdout into memory and never
    written to disk.  On POSIX both the ciphertext and the passphrase go
    in over pipes (nothing touches disk at all).  Where inheriting an
    extra file descriptor isn't supported (Windows), the *already
    encrypted* ciphertext is staged in a private temp file and the
    passphrase is fed on stdin — the decrypted plaintext is still only
    ever on the stdout pipe.
    """
    gpg_path, loopback = gpg_info
    argv = [gpg_path, "--batch", "--no-tty", "--quiet", "--yes"]
    if loopback:
        argv += ["--pinentry-mode", "loopback"]
    secret = passphrase.encode("utf-8") + b"\n"

    if os.name == "posix":
        read_fd, write_fd = os.pipe()
        try:
            os.write(write_fd, secret)
        finally:
            os.close(write_fd)
        try:
            proc = subprocess.run(
                argv + ["--passphrase-fd", str(read_fd), "--decrypt"],
                input=data,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                pass_fds=(read_fd,),
            )
        finally:
            os.close(read_fd)
        return _gpg_result(proc)

    # Non-POSIX (Windows): inheriting an extra fd isn't supported, so the
    # passphrase goes on stdin and the (encrypted) ciphertext via a temp
    # file.  The decrypted plaintext is still only ever read from stdout.
    with _staged_ciphertext(data) as tmp:
        proc = subprocess.run(
            argv + ["--passphrase-fd", "0", "--decrypt", tmp],
            input=secret,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return _gpg_result(proc)


def decrypt_ucs_bytes(data: bytes, passphrase: str) -> bytes:
    """Decrypt an OpenPGP-encrypted UCS, returning the gzip-tar plaintext.

    Uses ``gpg``/``gpg2`` when available, otherwise the bundled
    pure-Python decryptor.  Set ``F5_UCS_PYPGP=1`` to force the
    pure-Python path even when gpg is installed.  Raises
    :class:`UcsDecryptionError` on failure (typically a wrong passphrase).
    """
    forced_pure = os.environ.get("F5_UCS_PYPGP") not in (None, "", "0")
    gpg_info = None if forced_pure else _resolve_gpg()
    if gpg_info is not None:
        return _gpg_decrypt(gpg_info, data, passphrase)

    from . import _openpgp

    try:
        return _openpgp.decrypt_symmetric(data, passphrase)
    except _openpgp.OpenPGPDecryptError as exc:
        raise UcsDecryptionError(str(exc)) from exc
    except _openpgp.OpenPGPError as exc:
        # A feature the lightweight decryptor doesn't cover (non-AES
        # cipher, AEAD, legacy SED).  gpg handles these — point at it.
        raise UcsDecryptionError(f"{exc}; {_gpg_install_hint()} for full support") from exc


def decrypt_if_encrypted(
    raw: bytes,
    *,
    passphrase_provider: PassphraseProvider | None = None,
    label: str = "UCS",
) -> bytes:
    """Return plaintext UCS bytes, decrypting *raw* first if it is OpenPGP.

    Plain (gzip) input is returned unchanged.  Encrypted input is
    decrypted via :func:`decrypt_ucs_bytes`, resolving the passphrase
    through *passphrase_provider* (a default env-var/prompt provider is
    used when none is supplied).  The result is validated to be a gzip
    archive so a wrong passphrase that somehow slips past gpg still
    produces a clear error.
    """
    if not is_pgp_bytes(raw):
        return raw
    provider = passphrase_provider or make_passphrase_provider()
    plaintext = decrypt_ucs_bytes(raw, provider())
    if not is_ucs_bytes(plaintext):
        raise UcsDecryptionError(
            f"{label}: decrypted data is not a UCS archive (gzip magic missing) — wrong passphrase?"
        )
    return plaintext


def ucs_to_scf(ucs_bytes: bytes, *, include_extras: bool = False) -> str:
    """Extract *ucs_bytes* and return a concatenated SCF text.

    When *include_extras* is true, any additional ``config/*.conf``
    files not in the canonical order are appended at the end (still
    in deterministic alphabetical order).  Members that are not
    present in the archive are silently skipped.
    """
    chunks: list[str] = []
    seen: set[str] = set()
    extras: list[tuple[str, str]] = []

    # ``tarfile``/gzip raise TarError (and occasionally EOFError) on a
    # corrupt archive or a gzip stream that isn't a tar; normalise those
    # to ValueError so every CLI call site's ``(OSError, ValueError)``
    # handler reports them cleanly instead of crashing with a traceback.
    try:
        with tarfile.open(fileobj=io.BytesIO(ucs_bytes), mode="r:*") as tf:
            members_by_name = {m.name.lstrip("./"): m for m in tf.getmembers() if m.isfile()}

            for canonical in _SCF_MEMBER_ORDER:
                member = members_by_name.get(canonical)
                if member is None:
                    continue
                text = _read_member(tf, member)
                chunks.append(f"#\n# {canonical}\n#\n{text.rstrip()}\n")
                seen.add(canonical)

            if include_extras:
                for name, member in sorted(members_by_name.items()):
                    if name in seen:
                        continue
                    if not name.startswith("config/"):
                        continue
                    if not name.endswith(".conf"):
                        continue
                    text = _read_member(tf, member)
                    extras.append((name, text))
    except (tarfile.TarError, EOFError) as exc:
        raise ValueError(f"invalid UCS archive: {exc}") from exc

    for name, text in extras:
        chunks.append(f"#\n# {name}\n#\n{text.rstrip()}\n")

    return "\n".join(chunks)


def ucs_archive_to_scf(
    raw: bytes,
    *,
    passphrase_provider: PassphraseProvider | None = None,
    include_extras: bool = False,
    label: str = "UCS",
) -> str:
    """Turn raw UCS bytes — encrypted or plain — into SCF text.

    Convenience wrapper that decrypts (when needed) and then extracts,
    keeping the decrypted archive entirely in memory.
    """
    data = decrypt_if_encrypted(raw, passphrase_provider=passphrase_provider, label=label)
    if not is_ucs_bytes(data):
        raise ValueError(f"{label}: not a valid UCS archive")
    return ucs_to_scf(data, include_extras=include_extras)


def read_ucs_member(
    raw: bytes,
    member_path: str,
    *,
    passphrase_provider: PassphraseProvider | None = None,
    label: str = "UCS",
) -> bytes:
    """Return the bytes of a single file member from a UCS archive.

    Decrypts *raw* first when it is OpenPGP-encrypted (a UCS holds SSL
    private keys, so the cleartext stays in memory), then extracts the
    member named by *member_path* — typically a BIG-IP filestore
    ``cache-path`` such as
    ``/config/filestore/files_d/Common_d/certificate_d/:Common:foo.crt_1``.

    Matching is tolerant: the leading ``/`` is optional, and the
    ``:partition:`` filename prefix BIG-IP writes on disk is matched
    even when the ``cache-path`` recorded in the stanza omits it (and
    vice versa).  Raises :class:`KeyError` when no member matches and
    :class:`ValueError` when the archive is corrupt.
    """
    data = decrypt_if_encrypted(raw, passphrase_provider=passphrase_provider, label=label)
    if not is_ucs_bytes(data):
        raise ValueError(f"{label}: not a valid UCS archive")
    want = member_path.lstrip("/").lstrip("./")
    base = want.rsplit("/", 1)[-1]
    # The ``:partition:`` prefix is informational for matching — compare
    # the bare leaf so a stanza cache-path and the on-disk filename agree.
    bare = base.split(":")[-1]
    try:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as tf:
            files = [m for m in tf.getmembers() if m.isfile()]
            by_name = {m.name.lstrip("./"): m for m in files}
            member = by_name.get(want)
            if member is None:
                candidates = [
                    m
                    for m in files
                    if (mbase := m.name.rsplit("/", 1)[-1]) == base or mbase.split(":")[-1] == bare
                ]
                if len(candidates) > 1:
                    # Prefer a filestore path so a same-named pair in
                    # different stores doesn't collide.
                    narrowed = [m for m in candidates if "/filestore/" in m.name]
                    candidates = narrowed or candidates
                member = candidates[0] if candidates else None
            if member is None:
                raise KeyError(member_path)
            fh = tf.extractfile(member)
            return fh.read() if fh is not None else b""
    except (tarfile.TarError, EOFError) as exc:
        raise ValueError(f"invalid UCS archive: {exc}") from exc


def _read_member(tf: tarfile.TarFile, member: tarfile.TarInfo) -> str:
    fh = tf.extractfile(member)
    if fh is None:
        return ""
    raw = fh.read()
    return raw.decode("utf-8", errors="replace")


def extract_ucs_file(
    path: Path | str,
    *,
    passphrase_provider: PassphraseProvider | None = None,
    include_extras: bool = False,
) -> str:
    """Convenience: read a UCS file from disk and return its SCF text.

    Transparently decrypts an encrypted (OpenPGP) UCS using
    *passphrase_provider* (default env-var/prompt resolution).
    """
    data = Path(path).read_bytes()
    if not (is_ucs_bytes(data) or is_pgp_bytes(data)):
        raise ValueError(f"{path}: not a UCS archive (gzip/OpenPGP magic missing)")
    return ucs_archive_to_scf(
        data,
        passphrase_provider=passphrase_provider,
        include_extras=include_extras,
        label=str(path),
    )


def make_test_ucs(members: dict[str, str]) -> bytes:
    """Build a minimal in-memory UCS archive from *members* dict.

    Used by tests; not part of the production fetch path.  Keys are
    paths inside the archive (e.g. ``"config/bigip.conf"``); values
    are file contents.
    """
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tf:
        for name, content in members.items():
            data = content.encode("utf-8")
            info = tarfile.TarInfo(name=name)
            info.size = len(data)
            tf.addfile(info, io.BytesIO(data))
    return buf.getvalue()
