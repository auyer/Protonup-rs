use std::io::Error;

use clap::CommandFactory;
use clap_complete::{generate_to, Shell};

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let outdir = "completions";
    let mut cmd = Opt::command();

    generate_to(Shell::Bash, &mut cmd, "protonup-rs", outdir)?;
    generate_to(Shell::Fish, &mut cmd, "protonup-rs", outdir)?;
    generate_to(Shell::Zsh, &mut cmd, "protonup-rs", outdir)?;

    Ok(())
}
