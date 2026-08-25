# Contributing

Contributions are welcome, especially those that turn reserved ISO 23387
contracts into conformance-tested behavior.

## Before opening a pull request

1. Read `AGENTS.md` and the affected crate's nested instructions.
2. Keep data templates below LOIN and other consumers; do not introduce reverse
   dependencies.
3. Add tests before claiming modeling, parsing, writing, mapping, or validation.
4. Keep restricted standards material local under `references/`.
5. Commit fixtures under `tests/fixtures/` only when they are original synthetic
   examples or their redistribution rights are recorded and compatible.
6. Keep domain, wire, validation, and governance responsibilities explicit.
7. Run:

```bash
./scripts/gate.sh
python -m pip install -r docs/requirements.txt
./scripts/build-docs.sh
```

8. Update README capability status, rustdoc, `ROADMAP.md`, and `CHANGELOG.md` for
   user-visible behavior. Pages copies those canonical files; do not maintain
   parallel changelog or roadmap documents under `docs/`.

## Conformance work

Representative examples are not enough to claim format or validation support.
Record fixture provenance and supported editions; exercise required and optional
fields; and test unknown-content preservation before claiming lossless round
trips.

## Commits and releases

Use focused commits with imperative subjects. Cross-repository changes publish
lower-level child commits and crates first, then update the
`openbimrs/openbim` submodule pin.
