#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/dt.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! cmp -s LICENSE openbim-dt/LICENSE; then
    printf 'crate and repository license files differ\n' >&2
    exit 1
fi

cargo fmt --all -- --check
python3 -m py_compile scripts/check-docs-site.py
python3 -m py_compile scripts/generate-schema-tables.py
bash -n scripts/build-docs.sh
bash -n scripts/test-capability-guard.sh
python3 - <<'PY'
from pathlib import Path

workflow = Path(".github/workflows/pages.yml").read_text(encoding="utf-8")
upload = "uses: actions/upload-pages-artifact@"
assert workflow.count(upload) == 1, "Pages workflow must have one upload step"
upload_block = workflow.split(upload, 1)[1].split("\n      - ", 1)[0]
assert "include-hidden-files: true" in upload_block, (
    "Pages upload must include validated hidden files such as .nojekyll"
)
PY
cargo build --workspace --all-targets --locked
cargo test --workspace --all-features --locked
./scripts/test-capability-guard.sh
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked

cargo metadata --no-deps --locked --format-version 1 | python3 -c '
import json, sys
packages = json.load(sys.stdin)["packages"]
assert len(packages) == 1, packages
p = packages[0]
assert p["name"] == "openbim-dt", p["name"]
assert p["version"] == "0.2.0", p["version"]
assert p["rust_version"] == "1.85", p["rust_version"]
assert p["repository"] == "https://github.com/openbimrs/dt", p["repository"]
assert p["homepage"] == "https://openbimrs.github.io/dt/", p["homepage"]
assert p["documentation"] == "https://docs.rs/openbim-dt", p["documentation"]
deps = p["dependencies"]
deps = {dep["name"]: dep for dep in deps}
assert set(deps) == {"getrandom", "quick-xml", "roxmltree"}, deps
assert deps["getrandom"]["req"] == "^0.2.16", deps["getrandom"]["req"]
assert deps["quick-xml"]["req"] == "^0.41.0", deps["quick-xml"]["req"]
assert not deps["quick-xml"]["uses_default_features"], deps["quick-xml"]
assert deps["roxmltree"]["req"] == "^0.21.1", deps["roxmltree"]["req"]
assert set(deps["roxmltree"]["features"]) == {"std", "positions"}, deps["roxmltree"]
assert all(dep.get("path") is None for dep in deps.values()), deps
targets = {(target["name"], tuple(target["kind"])) for target in p["targets"]}
assert ("openbim_dt", ("lib",)) in targets, targets
assert ("openbim-dt", ("bin",)) in targets, targets
'

package_files=$(cargo package -p openbim-dt --locked --allow-dirty --list)
for required in LICENSE README.md src/lib.rs src/main.rs src/parser.rs src/document.rs \
    tests/fixtures/README.md tests/fixtures/synthetic-library.xml; do
    case "$package_files" in
        *"$required"*) ;;
        *)
            printf 'package is missing required file: %s\n' "$required" >&2
            exit 1
            ;;
    esac
done
while IFS= read -r package_file; do
    case "$package_file" in
        .cargo_vcs_info.json | Cargo.lock | Cargo.toml | Cargo.toml.orig | LICENSE | README.md | src/*.rs | tests/*.rs | tests/fixtures/README.md | tests/fixtures/synthetic-library.xml) ;;
        *)
            printf 'package contains an undeclared file: %s\n' "$package_file" >&2
            exit 1
            ;;
    esac
done <<<"$package_files"
while IFS= read -r package_file; do
    lower=$(printf '%s' "$package_file" | tr '[:upper:]' '[:lower:]')
    case "$lower" in
        tests/fixtures/synthetic-library.xml) ;;
        references/* | *.pdf | *.xsd | *.xsd.xml | *.xml | *.xlsx | *.xls)
            printf 'package contains forbidden standards or fixture material: %s\n' "$package_file" >&2
            exit 1
            ;;
    esac
done <<<"$package_files"

cargo run --locked -q -p openbim-dt --bin openbim-dt -- inspect \
    openbim-dt/tests/fixtures/synthetic-library.xml
cargo run --locked -q -p openbim-dt --bin openbim-dt -- validate \
    openbim-dt/tests/fixtures/synthetic-library.xml

cargo package -p openbim-dt --locked --allow-dirty
