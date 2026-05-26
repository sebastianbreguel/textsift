use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub fn open_reader(path: &Path) -> Result<Box<dyn BufRead>> {
    if path.to_str() == Some("-") {
        Ok(Box::new(BufReader::new(io::stdin())))
    } else {
        let file = File::open(path).with_context(|| format!("cannot open '{}'", path.display()))?;
        Ok(Box::new(BufReader::new(file)))
    }
}

pub fn open_writer(path: Option<&Path>) -> Result<Box<dyn Write>> {
    match path {
        Some(p) => {
            let file =
                File::create(p).with_context(|| format!("cannot create '{}'", p.display()))?;
            Ok(Box::new(BufWriter::new(file)))
        }
        None => Ok(Box::new(BufWriter::new(io::stdout().lock()))),
    }
}

pub fn read_jsonl(reader: impl BufRead) -> impl Iterator<Item = (usize, Result<Value>)> {
    reader.lines().enumerate().filter_map(|(i, line)| {
        let line_num = i + 1;
        match line {
            Ok(l) if l.trim().is_empty() => None,
            Ok(l) => {
                let parsed = serde_json::from_str(&l)
                    .with_context(|| format!("invalid JSON on line {line_num}"));
                Some((line_num, parsed))
            }
            Err(e) => Some((line_num, Err(e.into()))),
        }
    })
}

pub fn write_jsonl(writer: &mut dyn Write, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writeln!(writer)?;
    Ok(())
}

pub fn extract_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(|v| v.as_str())
}
