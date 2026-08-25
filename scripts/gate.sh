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
bash -n scripts/build-docs.sh
cargo build --workspace --all-targets --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked

cargo metadata --no-deps --locked --format-version 1 | python3 -c '
import json, sys
packages = json.load(sys.stdin)["packages"]
assert len(packages) == 1, packages
p = packages[0]
assert p["name"] == "openbim-dt", p["name"]
assert p["version"] == "0.1.1", p["version"]
assert p["rust_version"] == "1.85", p["rust_version"]
assert p["repository"] == "https://github.com/openbimrs/dt", p["repository"]
assert p["homepage"] == "https://openbimrs.github.io/dt/", p["homepage"]
assert p["documentation"] == "https://docs.rs/openbim-dt", p["documentation"]
deps = p["dependencies"]
assert len(deps) == 1 and deps[0]["name"] == "openbim-core", deps
assert deps[0]["req"] == "^0.1.0", deps[0]["req"]
assert deps[0].get("path") is None, deps[0].get("path")
'

package_files=$(cargo package -p openbim-dt --locked --allow-dirty --list)
for required in LICENSE README.md src/lib.rs; do
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
        .cargo_vcs_info.json | Cargo.lock | Cargo.toml | Cargo.toml.orig | LICENSE | README.md | src/*.rs) ;;
        *)
            printf 'package contains an undeclared file: %s\n' "$package_file" >&2
            exit 1
            ;;
    esac
done <<<"$package_files"
while IFS= read -r package_file; do
    lower=$(printf '%s' "$package_file" | tr '[:upper:]' '[:lower:]')
    case "$lower" in
        references/* | *.pdf | *.xsd | *.xsd.xml | *.xml | *.xlsx | *.xls)
            printf 'package contains forbidden standards or fixture material: %s\n' "$package_file" >&2
            exit 1
            ;;
    esac
done <<<"$package_files"

cargo package -p openbim-dt --locked --allow-dirty
