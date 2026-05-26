mod cli;
mod exact;
mod io;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use exact::ExactDedup;

fn main() -> Result<()> {
    let args = Cli::parse();
    let reader = io::open_reader(&args.input)?;
    let mut writer = io::open_writer(args.output.as_deref())?;

    let mut dedup = ExactDedup::new();
    let mut total: u64 = 0;
    let mut exact_dupes: u64 = 0;

    for (line_num, result) in io::read_jsonl(reader) {
        match result {
            Ok(value) => match io::extract_field(&value, &args.field) {
                Some(text) => {
                    total += 1;
                    if dedup.is_duplicate(text) {
                        exact_dupes += 1;
                    } else {
                        io::write_jsonl(&mut writer, &value)?;
                    }
                }
                None => {
                    eprintln!("warning: line {line_num} missing field '{}'", args.field);
                }
            },
            Err(e) => {
                eprintln!("warning: {e}");
            }
        }
    }

    if args.stats {
        eprintln!("total docs: {total}");
        eprintln!("exact duplicates: {exact_dupes}");
        eprintln!(
            "unique docs: {}",
            total.saturating_sub(exact_dupes)
        );
    }

    Ok(())
}
