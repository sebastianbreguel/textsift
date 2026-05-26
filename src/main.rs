use anyhow::Result;
use clap::Parser;
use textsift::cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    textsift::pipeline::run(&args)
}
