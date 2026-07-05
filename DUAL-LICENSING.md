# Dual Licensing

`tcl-lsp` is offered under **two** licensing options. You may choose whichever
fits your use.

## 1. Open-source license (default): GNU AGPL-3.0-or-later

Unless you have a separate written agreement with the copyright holder, this
software is licensed to you under the **GNU Affero General Public License,
version 3 or (at your option) any later version** (AGPL-3.0-or-later). The full
text is in [`LICENSE`](LICENSE).

The AGPL is a strong copyleft license. In particular, if you modify this
software and make it available to users over a network — for example as part of
a hosted or SaaS product — the AGPL requires you to offer those users the
**complete corresponding source code** of your modified version under the same
terms.

You are free to use, study, modify, and redistribute the software — including
**commercially and for paid products** — provided you comply with the AGPL. For
network and hosted use, "comply" means publishing your corresponding source. If
that copyleft obligation is compatible with your product, no further permission
is required.

## 2. Commercial license (on request)

If you want to use `tcl-lsp` in a product **without** the AGPL's copyleft
obligations — for example, to build a proprietary or closed-source product, or a
hosted/paid service whose source you do not wish to publish — a separate
**commercial license** is available.

To obtain one, contact the copyright holder:

- **James Deucker** (**bitwisecook**)
- GitHub: <https://github.com/bitwisecook>

Commercial licensing terms (scope, pricing, support) are agreed individually.
A commercial license removes the AGPL source-disclosure requirement for the
covered use; it does not change the terms for anyone using the software under
the AGPL.

> Note: The AGPL does not — and legally cannot — forbid commercial or paid use;
> its condition is source disclosure, not payment. The commercial license
> exists so that organisations who cannot or prefer not to meet that
> source-disclosure obligation have a lawful alternative.

## 3. Third-party components

This repository incorporates third-party code that is **not** covered by either
option above. Such components remain under **their own original licenses and
belong to their original authors**, and nothing in this project's licensing
grants or restricts rights in them beyond what those licenses provide. Notable
examples include:

- **Tcl runtime library** (`runtime/rust/vendor/tcl_library/`) — licensed under
  the standard Tcl/BSD-style license; see
  [`runtime/rust/vendor/tcl_library/license.terms`](runtime/rust/vendor/tcl_library/license.terms).
- **Henry Spencer's Advanced Regular Expression test corpus**
  (`rust/tcl-regex/tests/data/reg.test`) — Copyright © 1998, 1999 Henry
  Spencer; retained verbatim under Spencer's permissive regex license.
- **F5 TMOS default-profile base** (`scripts/registry-audit/data/profile_base.conf`)
  — a verbatim `profile_base.conf` taken from the
  [f5-corkscrew](https://github.com/f5devcentral/f5-corkscrew) project's test
  fixtures, distributed by F5 DevCentral under the Apache License 2.0. Retained
  solely as input to `scripts/registry-audit/gen_profile_defaults.py`, which
  derives the registry's profile-default table
  (`rust/registry/tcl-registry/src/profile_defaults/generated.rs`).

The `tcl-regex` engine itself is an independent, idiomatic Rust reimplementation
of the ARE semantics (not a transliteration of Spencer's C); its source is
original work under the licensing options above. Spencer's permissive license
allows reuse for any purpose, including relicensing derived work, provided the
original notice is retained and the origin and nature of modifications are
indicated — which this project does here and in the engine's documentation.

When a third-party component's license imposes obligations (such as retaining a
copyright notice), those obligations continue to apply regardless of which of
the two options above you choose.
