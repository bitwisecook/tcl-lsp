# How are redacted configuration credentials recreated and verified?

Redacted evidence uses `<REDACTED_PASSWORD>`, `<REDACTED_PASSWORD_HASH>`, `<REDACTED_API_KEY>`, `<REDACTED_PRIVATE_KEY>`, or `<GENERATED_CERTIFICATE_PEM>`. These are disclosure placeholders, not deployable values. Generate replacements locally, configure the intended account or service, and record only the resulting completion code or status. A redacted snapshot cannot reproduce its original credential bytes.

## Certificates and signing keys

The [certificate recipe](../../../../rust/tcl-bigip-query/tests/fixtures/certificate-generation.md) creates a private temporary key and a self-signed test certificate, verifies the created certificate, and compares its public key with the generated key. Use a certificate signing request and the intended CA for a deployed certificate; a self-signed fixture does not recreate its original issuer. The [vendor-root recipe](../../../../rust/tcl-sslictcl/data/README.md) retrieves pinned public roots outside the checkout and verifies their fingerprints. No original certificate or key bytes belong in the evidence.

## Readable passwords

This recipe guarantees upper and lower case, digits, and punctuation. Its alphabet excludes commonly confused letters and digits. The punctuation has the same location on standard UK and US layouts. The value has four groups of six characters, separated by hyphens. It goes to a private file outside the checkout:

```sh
umask 077
CREDENTIAL_DIR=$(mktemp -d /tmp/tcl-credentials.XXXXXX)
trap 'rm -rf "$CREDENTIAL_DIR"' EXIT
python3 - "$CREDENTIAL_DIR/password" <<'PY'
import secrets
import sys
from pathlib import Path
buckets = ['acdefhjkmnprtuvwxy', 'ACDEFHJKMNPRTUVWXY', '3479', '!']
characters = [secrets.choice(bucket) for bucket in buckets]
characters += [secrets.choice(''.join(buckets)) for _ in range(20)]
secrets.SystemRandom().shuffle(characters)
value = '-'.join(''.join(characters[start:start + 6]) for start in range(0, 24, 6))
Path(sys.argv[1]).write_text(value + '\n')
PY
python3 - "$CREDENTIAL_DIR/password" <<'PY'
import getpass
import secrets
import sys
from pathlib import Path
created = Path(sys.argv[1]).read_text().rstrip('\n')
assert len(created) == 27
assert secrets.compare_digest(created, getpass.getpass('Enter the locally created password: '))
print('password comparison passed')
PY
```

Set the account password through the service's interactive password prompt or an approved credential-store integration. Verify a fresh authentication attempt against that account. The local comparison checks the generated file; it does not prove that an account update succeeded. Keep password hashes, fallback passwords, recovery codes, and authentication request bodies out of captured logs. Never put the value in command-line arguments.

## API keys and tokens

A service-issued API key must be created through that service's supported credential flow. Random bytes from OpenSSL are suitable only when your own service explicitly accepts a newly registered opaque token:

```sh
openssl rand -hex -out "$CREDENTIAL_DIR/token" 32
python3 - "$CREDENTIAL_DIR/token" <<'PY'
import re
import sys
from pathlib import Path
value = Path(sys.argv[1]).read_text().strip()
assert re.fullmatch(r'[0-9a-f]{64}', value)
print('token format passed')
PY
```

Register the token privately, then verify one authenticated request to the intended API. Read the token from the credential store or a private curl configuration file. Record the HTTP status, excluding response headers, cookies, and bodies that can issue another token. Format validation alone does not prove registration or authorisation. The [OpenSSL random-generation manual](https://docs.openssl.org/3.0/man1/openssl-rand/) describes the command.

## SSH keys and hardware credentials

Generate a fresh key outside the checkout, using the interactive passphrase prompt:

```sh
ssh-keygen -t ed25519 -f "$CREDENTIAL_DIR/ssh-key"
ssh-keygen -y -f "$CREDENTIAL_DIR/ssh-key" > "$CREDENTIAL_DIR/derived.pub"
cmp -s "$CREDENTIAL_DIR/derived.pub" "$CREDENTIAL_DIR/ssh-key.pub"
```

An SSH public-key file contains an optional comment. If one was supplied, compare the first two fields of both public files instead. Install the public key through the intended service, then verify a fresh authentication using that key. Keep the key files and fingerprints in the local credential workflow, not the evidence.

For an attached FIDO authenticator, OpenSSH supports:

```sh
ssh-keygen -t ed25519-sk -O verify-required -f "$CREDENTIAL_DIR/ssh-hardware-key"
```

Follow the authenticator's user-verification prompt and verify a fresh SSH authentication. This creates an SSH hardware credential. A browser passkey instead requires the website's WebAuthn registration ceremony and authenticator; create it through that website, then verify a separate sign-in. Preserve only success/failure and the account/credential role as a placeholder, excluding private handles, credential IDs, challenge values, attestation objects, and recovery material. See [OpenSSH's key-generation manual](https://man.openbsd.org/ssh-keygen).
