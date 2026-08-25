#!/usr/bin/env bash
# Prove that security and lossless-round-trip tests reject key mutations.
set -euo pipefail

cd "$(dirname "$0")/.."
parser=openbim-dt/src/parser.rs
writer=openbim-dt/src/document.rs
model=openbim-dt/src/model.rs
value=openbim-dt/src/value.rs
domain=openbim-dt/src/domain.rs
cli=openbim-dt/src/main.rs
backup=$(mktemp -d)
cp "$parser" "$backup/parser.rs"
cp "$writer" "$backup/document.rs"
cp "$model" "$backup/model.rs"
cp "$value" "$backup/value.rs"
cp "$domain" "$backup/domain.rs"
cp "$cli" "$backup/main.rs"

restore() {
    cp "$backup/parser.rs" "$parser"
    cp "$backup/document.rs" "$writer"
    cp "$backup/model.rs" "$model"
    cp "$backup/value.rs" "$value"
    cp "$backup/domain.rs" "$domain"
    cp "$backup/main.rs" "$cli"
    rm -rf "$backup"
}
trap restore EXIT INT TERM

replace_exact() {
    python3 - "$1" "$2" "$3" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
old, new = sys.argv[2], sys.argv[3]
text = path.read_text(encoding="utf-8")
assert text.count(old) == 1, (path, text.count(old))
path.write_text(text.replace(old, new), encoding="utf-8")
PY
}

expect_killed() {
    name=$1
    shift
    log="$backup/$name.log"
    if "$@" >"$log" 2>&1; then
        printf 'mutation survived: %s\n' "$name" >&2
        cat "$log" >&2
        exit 1
    fi
    printf 'mutation killed: %s\n' "$name"
}

replace_exact "$parser" \
'    strict_well_formedness_check(xml, options)?;' \
'    // mutation: strict XML preflight disabled'
expect_killed strict-xml cargo test -p openbim-dt --test well_formedness parser_rejects_namespace_and_character_well_formedness_violations
cp "$backup/parser.rs" "$parser"

replace_exact "$writer" \
'        Node::Element(element) => write_element(writer, element),' \
'        Node::Element(_) => Ok(()),'
expect_killed unknown-elements cargo test -p openbim-dt --test document document_round_trip_preserves_all_xml_semantics
cp "$backup/document.rs" "$writer"

replace_exact "$model" \
'        ElementKind::from_element(self.root()).filter(|kind| kind.is_global_root())' \
'        ElementKind::from_element(self.root())'
expect_killed global-roots cargo test -p openbim-dt --test well_formedness root_and_context_sensitive_types_match_annex_e_declarations
cp "$backup/model.rs" "$model"

replace_exact "$model" \
'        && reference_allowed(parent_local, element.local_name())' \
''
expect_killed reference-context cargo test -p openbim-dt --test well_formedness reference_and_multilingual_diagnostics_follow_parent_declarations
cp "$backup/model.rs" "$model"

replace_exact "$model" \
'        && multilingual_allowed(parent_local, element.local_name())' \
''
expect_killed multilingual-context cargo test -p openbim-dt --test well_formedness reference_and_multilingual_diagnostics_follow_parent_declarations
cp "$backup/model.rs" "$model"

replace_exact "$model" \
'        "Name" | "Definition" | "Description" | "Example" | "Symbol" | "ValueList"' \
'        "Name" | "Definition" | "Description" | "Example" | "Symbol"'
expect_killed value-list-language cargo test -p openbim-dt --test well_formedness value_list_requires_language_and_contains_repeating_ordered_values
cp "$backup/model.rs" "$model"

replace_exact "$value" \
"        if matches!(character, ' ' | '\\t' | '\\r' | '\\n') {" \
"        if character == '\\0' {"
expect_killed whitespace-collapse cargo test -p openbim-dt --test value_types decimal_and_language_contracts_match_xml_schema_lexical_spaces
cp "$backup/value.rs" "$value"

replace_exact "$value" \
'            .all(is_xml_10_character)' \
'            .all(|_| true)'
expect_killed any-uri-validation cargo test -p openbim-dt --test value_types multilingual_text_and_references_are_dt_owned_contracts
cp "$backup/value.rs" "$value"

replace_exact "$value" \
'        is_xs_datetime(&value)' \
'        true'
expect_killed datetime-validation cargo test -p openbim-dt --test value_types concept_contract_is_reusable_by_standards_that_extend_concept_type
cp "$backup/value.rs" "$value"

replace_exact "$value" \
"        || (year.len() > 4 && year.starts_with('0'))" \
''
expect_killed datetime-leading-zero cargo test -p openbim-dt --test value_types concept_contract_is_reusable_by_standards_that_extend_concept_type
cp "$backup/value.rs" "$value"

replace_exact "$value" \
"                && fraction.is_none_or(|value| value.bytes().all(|byte| byte == b'0'))))" \
'                && fraction.is_none_or(|_| true)))'
expect_killed datetime-end-of-day cargo test -p openbim-dt --test value_types concept_contract_is_reusable_by_standards_that_extend_concept_type
cp "$backup/value.rs" "$value"

replace_exact "$parser" \
'    let namespace_uri = scope.get(prefix.as_deref().unwrap_or_default()).cloned();' \
'    let namespace_uri = scope
        .get(prefix.as_deref().unwrap_or_default())
        .map(|uri| std::sync::Arc::<str>::from(uri.as_ref()));'
expect_killed namespace-storage-sharing cargo test -p openbim-dt --test well_formedness inherited_namespace_storage_is_shared_and_scopes_restore_without_cloning
cp "$backup/parser.rs" "$parser"

replace_exact "$value" \
'            && digits.bytes().any(|byte| byte != b'"'"'0'"'"');' \
'            && true;'
expect_killed positive-integer cargo test -p openbim-dt --test value_types decimal_and_language_contracts_match_xml_schema_lexical_spaces
cp "$backup/value.rs" "$value"

replace_exact "$value" \
'            names: vec![first_name],' \
'            names: Vec::new(),'
expect_killed concept-required-name cargo test -p openbim-dt --test value_types concept_contract_is_reusable_by_standards_that_extend_concept_type
cp "$backup/value.rs" "$value"

replace_exact "$domain" \
'            values: vec![first_value],' \
'            values: Vec::new(),'
expect_killed value-list-required-value cargo test -p openbim-dt --test domain_types owned_type_contracts_enforce_annex_e_required_content
cp "$backup/domain.rs" "$domain"

replace_exact "$domain" \
'    order: Option<i32>,' \
'    order: Option<i64>,'
expect_killed value-list-order-bound cargo test -p openbim-dt --test domain_types owned_type_contracts_enforce_annex_e_required_content
cp "$backup/domain.rs" "$domain"

replace_exact "$domain" \
'    MinInclusive(String),' \
'    Length(String),'
expect_killed datatype-constraint-shape cargo test -p openbim-dt --test domain_types owned_type_contracts_enforce_annex_e_required_content
cp "$backup/domain.rs" "$domain"

replace_exact "$domain" \
'    data_formats: Vec<String>,' \
'    data_formats: Vec<DataTypeName>,'
expect_killed data-format-pattern cargo test -p openbim-dt --test domain_types owned_type_contracts_enforce_annex_e_required_content
cp "$backup/domain.rs" "$domain"

replace_exact "$writer" \
'            '\''\n'\'' => output.push_str("&#xA;"),' \
'            '\''\n'\'' => output.push(character),'
expect_killed attribute-whitespace-roundtrip cargo test -p openbim-dt --test document control_character_references_survive_repeated_round_trips
cp "$backup/document.rs" "$writer"

replace_exact "$writer" \
'            '\''\r'\'' => output.push_str("&#13;"),' \
'            '\''\r'\'' => output.push(character),'
expect_killed text-carriage-return-roundtrip cargo test -p openbim-dt --test document control_character_references_survive_repeated_round_trips
cp "$backup/document.rs" "$writer"

replace_exact "$cli" \
'    atomic_replace(output, xml.as_bytes())?;' \
'    let file_name = output.file_name().and_then(|name| name.to_str()).unwrap();
    let temporary = output.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    std::fs::write(&temporary, xml)?;
    std::fs::rename(&temporary, output)?;'
expect_killed temporary-symlink cargo test -p openbim-dt --test cli rewrite_does_not_follow_a_precreated_temporary_symlink
cp "$backup/main.rs" "$cli"

cargo fmt --all -- --check
