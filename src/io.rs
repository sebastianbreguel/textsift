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

/// Iterate non-blank input lines with 1-based line numbers, preserving the
/// raw bytes so output can be byte-identical passthrough.
pub fn read_lines(reader: impl BufRead) -> impl Iterator<Item = (usize, io::Result<String>)> {
    reader.lines().enumerate().filter_map(|(i, line)| {
        let line_num = i + 1;
        match line {
            Ok(l) if l.trim().is_empty() => None,
            other => Some((line_num, other)),
        }
    })
}

/// Write a raw line followed by a newline — no re-serialization.
pub fn write_line(writer: &mut dyn Write, line: &str) -> Result<()> {
    writeln!(writer, "{line}")?;
    Ok(())
}

pub fn write_jsonl(writer: &mut dyn Write, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writeln!(writer)?;
    Ok(())
}

pub fn extract_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(|v| v.as_str())
}
