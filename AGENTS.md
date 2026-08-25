# Data-template repository instructions

This repository owns the OpenBIM.rs ISO 23387 data-template family. The
published crate is currently a namespace scaffold; do not describe parsing,
writing, complete modeling, validation, ISO 23386 governance, or ISO 12006-3
mapping as implemented without executable conformance evidence.

## Map

- `openbim-dt/` — canonical published crate; read its `AGENTS.md` before editing
- `docs/` — architecture and maintained documentation
- `ROADMAP.md` — canonical public capability roadmap
- `mkdocs.yml` — GitHub Pages navigation and theme configuration
- `references/` — ignored local standards material; never publish it
- `tests/fixtures/` — redistributable or original synthetic examples only
- `scripts/gate.sh` — complete local/CI verification gate
- `scripts/build-docs.sh` — assembles MkDocs prose and generated rustdoc API
- `CHANGELOG.md` — user-visible changes using Keep a Changelog

## Commands

```bash
./scripts/gate.sh
cargo test --workspace
cargo package -p openbim-dt
python -m pip install -r docs/requirements.txt
./scripts/build-docs.sh
```

Trust command exit codes. Never pipe Cargo output through a summarizer that
hides the Cargo process status.

## Boundaries

- Data-template contracts may depend on released shared core contracts.
- LOIN, bSDD tooling, and other consumers may depend on data templates.
- Data templates must never depend on LOIN.
- Release-critical metadata is explicit in the crate manifest; do not replace
  it with parent-workspace inheritance.
- Restricted standards material remains local under ignored `references/`.

## Documentation discipline

Update README capability status, rustdoc, `ROADMAP.md`, and `CHANGELOG.md`
together for user-visible changes. Pages copies those canonical files; do not
create parallel changelog or roadmap copies under `docs/`.
