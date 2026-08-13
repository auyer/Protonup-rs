use std::io::Error;

use clap::CommandFactory;
use clap_complete::{Shell, generate_to};

include!("src/cli.rs");

#[derive(serde::Deserialize)]
struct SourceEntry {
    name: String,
}

fn completion_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '(' | ')'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

// this has to be done here otherwise we need to compile the lib just to run this build script
fn tool_completion_names() -> Vec<String> {
    let ron_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../libprotonup/src/sources.ron");
    let content = match std::fs::read_to_string(&ron_path) {
        Ok(c) => c,
        Err(e) => {
            println!(
                "cargo:warning=could not read {} ({e}); tool completions omitted",
                ron_path.display()
            );
            return Vec::new();
        }
    };
    match ron::from_str::<Vec<SourceEntry>>(&content) {
        Ok(entries) => entries
            .into_iter()
            .map(|e| completion_name(&e.name))
            .collect(),
        Err(e) => {
            println!("cargo:warning=failed to parse sources.ron: {e}; tool completions omitted");
            Vec::new()
        }
    }
}

fn main() -> Result<(), Error> {
    let outdir = "completions";

    let names = tool_completion_names();
    let mut cmd = Opt::command();
    if !names.is_empty() {
        cmd = cmd.mut_arg("tool", move |a| {
            a.value_parser(clap::builder::PossibleValuesParser::new(names))
        });
    }

    generate_to(Shell::Bash, &mut cmd, "protonup-rs", outdir)?;
    generate_to(Shell::Fish, &mut cmd, "protonup-rs", outdir)?;
    generate_to(Shell::Zsh, &mut cmd, "protonup-rs", outdir)?;

    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=../libprotonup/src/sources.ron");

    Ok(())
}
