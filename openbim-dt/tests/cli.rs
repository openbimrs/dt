use std::{fs, process::Command};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/synthetic-library.xml"
);

#[test]
fn inspect_reports_dt_root_and_diagnostics() {
    let output = Command::new(env!("CARGO_BIN_EXE_openbim-dt"))
        .args(["inspect", FIXTURE])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("root=Library"));
    assert!(stdout.contains("errors=0"));
}

#[test]
fn validate_uses_a_distinct_exit_code_for_semantic_errors() {
    let malformed =
        std::env::temp_dir().join(format!("openbim-dt-invalid-{}.xml", std::process::id()));
    fs::write(
        &malformed,
        r#"<dt:Property xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="bad"/>"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_openbim-dt"))
        .arg("validate")
        .arg(&malformed)
        .output()
        .unwrap();
    let _ = fs::remove_file(&malformed);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("InvalidGuid"));
}

#[test]
fn rewrite_emits_a_reparseable_document() {
    let output_path =
        std::env::temp_dir().join(format!("openbim-dt-rewrite-{}.xml", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_openbim-dt"))
        .args(["rewrite", FIXTURE, output_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let xml = fs::read_to_string(&output_path).unwrap();
    let _ = fs::remove_file(&output_path);
    openbim_dt::Document::parse(&xml).unwrap();
}

#[cfg(unix)]
#[test]
fn rewrite_does_not_follow_a_precreated_temporary_symlink() {
    let directory = std::env::temp_dir().join(format!("openbim-dt-symlink-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let victim = directory.join("victim");
    fs::write(&victim, "DO-NOT-OVERWRITE\n").unwrap();

    let script = r#"set -eu
printf '%s' "$$" > "$1/child-pid"
ln -s "$1/victim" "$1/.output.xml.$$.tmp"
exec "$2" rewrite "$3" "$1/output.xml"
"#;
    let output = Command::new("bash")
        .args(["-c", script, "sh"])
        .arg(&directory)
        .arg(env!("CARGO_BIN_EXE_openbim-dt"))
        .arg(FIXTURE)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&victim).unwrap(), "DO-NOT-OVERWRITE\n");
    let output_type = fs::symlink_metadata(directory.join("output.xml"))
        .unwrap()
        .file_type();
    assert!(output_type.is_file() && !output_type.is_symlink());

    let child_pid = fs::read_to_string(directory.join("child-pid")).unwrap();

    for path in [
        directory.join("output.xml"),
        directory.join(format!(".output.xml.{child_pid}.tmp")),
        directory.join("child-pid"),
        victim,
    ] {
        let _ = fs::remove_file(path);
    }
    let _ = fs::remove_dir(directory);
}
