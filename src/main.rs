mod cli;
mod io;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    let reader = io::open_reader(&args.input)?;
    let mut writer = io::open_writer(args.output.as_deref())?;

    for (line_num, result) in io::read_jsonl(reader) {
        match result {
            Ok(value) => {
                match io::extract_field(&value, &args.field) {
                    Some(_) => io::write_jsonl(&mut writer, &value)?,
                    None => {
                        eprintln!("warning: line {line_num} missing field '{}'", args.field);
                    }
                }
            }
            Err(e) => {
                eprintln!("warning: {e}");
            }
        }
    }

    Ok(())
}
