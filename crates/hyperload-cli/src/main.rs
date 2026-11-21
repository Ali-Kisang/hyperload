use anyhow::Result;
use clap::Parser;
use hyperload_core::{download, DownloadOptions};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Parser)]
#[command(name = "hyperload", version, about = "Resumable parallel HTTP/HTTPS downloader")]
struct Args {
    /// HTTP or HTTPS resource URL
    url: String,

    /// Destination file
    #[arg(short, long)]
    output: PathBuf,

    /// Maximum concurrent range requests (1-32)
    #[arg(long, default_value_t = 8)]
    connections: usize,

    /// Chunk size in MiB (minimum 1)
    #[arg(long, default_value_t = 16)]
    chunk_mib: u64,

    /// Retries per chunk
    #[arg(long, default_value_t = 4)]
    retries: usize,

    /// Expected SHA-256 checksum (64 hexadecimal characters)
    #[arg(long)]
    sha256: Option<String>,

    /// Total request timeout in seconds
    #[arg(long, default_value_t = 300)]
    timeout_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();
    let options = DownloadOptions {
        url: args.url,
        output: args.output,
        connections: args.connections,
        chunk_size: args.chunk_mib.saturating_mul(1024 * 1024),
        retries: args.retries,
        expected_sha256: args.sha256,
        request_timeout: Duration::from_secs(args.timeout_secs),
    };

    download(options).await
}
