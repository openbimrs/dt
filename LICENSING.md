# Licensing

## Current license

Unless a file or package states otherwise, repository-authored software and
documentation are licensed under the **MIT License** (`MIT`). See
[`LICENSE`](LICENSE).

The MIT License permits use, modification, distribution, sublicensing, and
sale, including in proprietary and networked products, provided the copyright
notice and permission notice are retained.

## Version history

This repository has carried three licensing states. Each grant remains valid
for the versions in which it was made; a later change does not revoke an
earlier one.

| Versions | License |
|---|---|
| Up to and including `0.2.0` (published) | MIT |
| Unpublished AGPL interval (`15e38f9`..`be96c5b`) | AGPL-3.0-or-later |
| `0.2.1` onwards | MIT |

No release was ever published under `AGPL-3.0-or-later`: the relicense landed
after `0.2.0` and is reverted here before any release carried it. Every
published version of `openbim-dt` is MIT.

This also keeps `openbim-dt` compatible with its MIT dependents: `openbim-loin`
is MIT and depends on this crate, which an AGPL release would have broken.

Verified against the crates.io versions API: `0.1.0`, `0.1.1` and `0.2.0` are
all MIT.

## Third-party material

Dependencies, standards, schemas, catalogs, fixtures, generated material, and
other third-party content retain their own copyright and license terms. Their
notices control where they differ from this repository's license. The
OpenBIM.rs license grant does not cover material its contributors do not have
the right to license.

ISO 23387 schema and example artifacts are ISO copyright. They are not tracked
or packaged by this repository; local copies belong under the ignored
`references/` directory.

## Contributions

Contributions are accepted under `MIT` unless an explicitly signed agreement
says otherwise. See [`CONTRIBUTING.md`](CONTRIBUTING.md).
