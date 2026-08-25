#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
docs_source="$repo_root/target/docs-source"
site_dir="$repo_root/target/site"
rust_target="$repo_root/target/docs-rust"

for path in "$docs_source" "$site_dir" "$rust_target"; do
  case "$path" in
    "$repo_root"/target/*) ;;
    *)
      printf 'refusing to clean unexpected path: %s\n' "$path" >&2
      exit 2
      ;;
  esac
done

rm -rf "$docs_source" "$site_dir" "$rust_target"
mkdir -p "$docs_source/assets/stylesheets"

install -m 0644 "$repo_root/README.md" "$docs_source/index.md"
install -m 0644 "$repo_root/CHANGELOG.md" "$docs_source/changelog.md"
install -m 0644 "$repo_root/ROADMAP.md" "$docs_source/roadmap.md"
install -m 0644 "$repo_root/docs/architecture.md" "$docs_source/architecture.md"
install -m 0644 \
  "$repo_root/docs/assets/stylesheets/extra.css" \
  "$docs_source/assets/stylesheets/extra.css"

python3 -m mkdocs build \
  --config-file "$repo_root/mkdocs.yml" \
  --site-dir "$site_dir" \
  --strict

RUSTDOCFLAGS="-D warnings" cargo doc \
  --manifest-path "$repo_root/Cargo.toml" \
  --package openbim-dt \
  --no-deps \
  --locked \
  --target-dir "$rust_target"

mkdir -p "$site_dir/api"
cp -R "$rust_target/doc/." "$site_dir/api/"
touch "$site_dir/.nojekyll"

python3 "$repo_root/scripts/check-docs-site.py" "$site_dir"
printf 'documentation site built at %s\n' "$site_dir"
