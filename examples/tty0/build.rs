use std::{
    env,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let records_dir = manifest_dir.join("data/records");
    let manifest_path = manifest_dir.join("data/archive_manifest.json");

    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed={}", records_dir.display());

    let mut record_paths = fs::read_dir(&records_dir)
        .expect("records directory must exist")
        .map(|entry| entry.expect("record entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    record_paths.sort();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let output_path = out_dir.join("embedded_archive.rs");
    let mut output = fs::File::create(&output_path).expect("create embedded archive module");

    writeln!(
        output,
        "pub const MANIFEST_JSON: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/archive_manifest.json\"));"
    )
    .expect("write manifest include");
    writeln!(output, "pub const RECORD_JSONS: &[(&str, &str)] = &[").expect("write header");

    for path in record_paths {
        let id = file_stem(&path);
        writeln!(
            output,
            "    ({id:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/records/{id}.json\"))),"
        )
        .expect("write record include");
    }

    writeln!(output, "];").expect("write footer");
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .expect("record path must have valid utf-8 stem")
        .to_string()
}
