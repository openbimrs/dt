# HERMES.md — OpenBIM.rs Data Templates

Canonical repository: <https://github.com/openbimrs/dt>
Integration repository: <https://github.com/openbimrs/openbim>

Read `AGENTS.md` before changing the repository and the nested `AGENTS.md`
before editing the crate. Keep this repository independently buildable; its
OpenBIM.rs integration location is `packages/dt`, but the parent workspace is
not required for standalone development.

## Verification

Run `./scripts/gate.sh`. It is the authoritative local and CI gate and decides
success from command exit codes.

Run `./scripts/build-docs.sh` after installing `docs/requirements.txt` when
changing public documentation. It assembles MkDocs and rustdoc into
`target/site/`; GitHub Pages publishes that exact verified artifact.

## Project conventions

- Rust 2021, MSRV 1.85, MIT.
- Pure Rust; unsafe code is forbidden.
- Dependency direction is core → data templates → LOIN/consumers.
- Keep domain contracts, XML wire representation, validation policy, and
  ISO 23386 governance workflows as explicit layers.
- Do not vendor ISO/DIN/CEN documents, schemas, or annex examples without
  confirmed redistribution rights. Local material belongs under ignored
  `references/`.
- Commit fixtures under `tests/fixtures/` only with recorded, compatible
  redistribution terms or when they are original synthetic fixtures.
- Use Keep a Changelog and distinguish implemented, reserved, and
  conformance-tested capabilities.
