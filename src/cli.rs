use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "textsift",
    version,
    about = "Fast text deduplication for ML datasets"
)]
pub struct Cli {
    /// Input file (JSONL). Use - for stdin.
    pub input: PathBuf,

    /// JSON field containing text to deduplicate (repeat for composite keys,
    /// e.g. --field instruction --field output)
    #[arg(short, long, required = true)]
    pub field: Vec<String>,

    /// Jaccard similarity threshold for MinHash
    #[arg(short, long, default_value = "0.8")]
    pub threshold: f64,

    /// Number of MinHash permutations
    #[arg(short, long, default_value = "128")]
    pub num_perm: usize,

    /// Word n-gram size for shingling
    #[arg(long, default_value = "5")]
    pub shingle_size: usize,

    /// Output duplicate clusters instead of deduplicated file
    #[arg(long)]
    pub clusters: bool,

    /// Skip MinHash, only do exact dedup
    #[arg(long)]
    pub exact_only: bool,

    /// Print dedup statistics to stderr
    #[arg(long)]
    pub stats: bool,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
