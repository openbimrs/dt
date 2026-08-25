#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    ffi::OsString,
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::{self, Write as _},
    path::Path,
    process::ExitCode,
};

use openbim_dt::{Document, LibraryItem, Severity};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("openbim-dt: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<ExitCode, Box<dyn Error>> {
    match arguments.as_slice() {
        [command, input] if command == "inspect" => inspect(Path::new(input)),
        [command, input] if command == "validate" => validate(Path::new(input)),
        [command, input, output] if command == "rewrite" => {
            rewrite(Path::new(input), Path::new(output))
        }
        _ => {
            eprintln!(
                "usage:\n  openbim-dt inspect <input.xml>\n  openbim-dt validate <input.xml>\n  openbim-dt rewrite <input.xml> <output.xml>"
            );
            Ok(ExitCode::from(64))
        }
    }
}

fn load(path: &Path) -> Result<Document, Box<dyn Error>> {
    let xml = fs::read_to_string(path)?;
    Ok(Document::parse(&xml)?)
}

fn inspect(path: &Path) -> Result<ExitCode, Box<dyn Error>> {
    let document = load(path)?;
    println!(
        "root={}",
        document
            .root_kind()
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "Unknown".to_owned())
    );
    if let Some(library) = document.library() {
        let items = library.items().count();
        let extensions = library
            .items()
            .filter(|item| matches!(item, LibraryItem::Extension(_)))
            .count();
        println!("items={items}");
        println!("extensions={extensions}");
    }
    let (errors, warnings) = diagnostic_counts(&document);
    println!("errors={errors}");
    println!("warnings={warnings}");
    Ok(ExitCode::SUCCESS)
}

fn validate(path: &Path) -> Result<ExitCode, Box<dyn Error>> {
    let document = load(path)?;
    let diagnostics = document.validate();
    for diagnostic in &diagnostics {
        println!(
            "{:?} {:?} {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.path, diagnostic.message
        );
    }
    let (errors, warnings) = diagnostic_counts(&document);
    println!("errors={errors}");
    println!("warnings={warnings}");
    Ok(if errors == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

fn rewrite(input: &Path, output: &Path) -> Result<ExitCode, Box<dyn Error>> {
    let document = load(input)?;
    let xml = document.to_xml_string()?;
    atomic_replace(output, xml.as_bytes())?;
    Ok(ExitCode::SUCCESS)
}

fn atomic_replace(output: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let file_name = output.file_name().ok_or("output path has no file name")?;
    for _ in 0..32 {
        let mut random = [0_u8; 16];
        getrandom::getrandom(&mut random)
            .map_err(|error| io::Error::other(format!("OS random source failed: {error}")))?;
        let mut token = String::with_capacity(32);
        for byte in random {
            write!(&mut token, "{byte:02x}").expect("writing to String cannot fail");
        }
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".{token}.tmp"));
        let temporary = output.with_file_name(temporary_name);

        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        };

        let result = (|| -> io::Result<()> {
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, output)
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a unique temporary output file",
    )
    .into())
}

fn diagnostic_counts(document: &Document) -> (usize, usize) {
    document
        .validate()
        .into_iter()
        .fold((0, 0), |(errors, warnings), diagnostic| {
            match diagnostic.severity {
                Severity::Error => (errors + 1, warnings),
                Severity::Warning => (errors, warnings + 1),
            }
        })
}
