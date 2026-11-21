use anyhow::{anyhow, bail, Context, Result};
use futures_util::{stream, StreamExt};
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tracing::{debug, info, warn};
use url::Url;

const MAX_CONNECTIONS: usize = 32;
const MIN_CHUNK_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url: String,
    pub output: PathBuf,
    pub connections: usize,
    pub chunk_size: u64,
    pub retries: usize,
    pub expected_sha256: Option<String>,
    pub request_timeout: Duration,
}

impl DownloadOptions {
    pub fn validate(&self) -> Result<()> {
        let parsed = Url::parse(&self.url).context("invalid download URL")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("only HTTP and HTTPS URLs are supported");
        }
        if self.connections == 0 || self.connections > MAX_CONNECTIONS {
            bail!("connections must be between 1 and {MAX_CONNECTIONS}");
        }
        if self.chunk_size < MIN_CHUNK_BYTES {
            bail!("chunk size must be at least 1 MiB");
        }
        if self.retries > 20 {
            bail!("retries must not exceed 20");
        }
        if let Some(hash) = &self.expected_sha256 {
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                bail!("expected SHA-256 must be exactly 64 hexadecimal characters");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    url: String,
    size: u64,
    chunk_size: u64,
    chunks_complete: Vec<bool>,
}

#[derive(Debug, Clone)]
struct ResourceInfo {
    size: u64,
    supports_ranges: bool,
}

#[derive(Clone)]
struct Paths {
    work_dir: PathBuf,
    manifest: PathBuf,
    chunks_dir: PathBuf,
}

pub async fn download(options: DownloadOptions) -> Result<()> {
    options.validate()?;
    if let Some(parent) = options.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).await?;
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(options.request_timeout)
        .user_agent(concat!("HyperLoad/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let info = probe_resource(&client, &options.url).await?;
    info!("resource size={} bytes ranges={}", info.size, info.supports_ranges);

    if info.size == 0 {
        bail!("server reported an empty resource or did not provide a usable content length");
    }

    let paths = make_paths(&options)?;
    fs::create_dir_all(&paths.chunks_dir).await?;

    if !info.supports_ranges || options.connections == 1 {
        download_sequential(&client, &options, &paths, info.size).await?;
    } else {
        download_parallel(&client, &options, &paths, info.size).await?;
    }

    assemble(&options, &paths, info.size, info.supports_ranges && options.connections > 1).await?;

    if let Some(expected) = &options.expected_sha256 {
        let actual = sha256_file(&options.output).await?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&options.output).await;
            bail!("SHA-256 mismatch: expected {expected}, got {actual}");
        }
        info!("SHA-256 verified: {actual}");
    }

    let _ = fs::remove_dir_all(&paths.work_dir).await;
    info!("download complete: {}", options.output.display());
    Ok(())
}

async fn probe_resource(client: &Client, url: &str) -> Result<ResourceInfo> {
    let head = client.head(url).send().await;
    let mut size = None;
    if let Ok(response) = head {
        if response.status().is_success() {
            size = response.content_length();
        }
    }

    // A one-byte range probe is more reliable than relying only on Accept-Ranges.
    let probe = client
        .get(url)
        .header(header::RANGE, "bytes=0-0")
        .send()
        .await
        .context("resource probe failed")?;

    let supports_ranges = probe.status() == StatusCode::PARTIAL_CONTENT;
    if supports_ranges {
        let range = probe
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !range.starts_with("bytes 0-0/") {
            bail!("server returned an invalid Content-Range for the probe");
        }
        let total = range.split('/').nth(1).and_then(|s| s.parse::<u64>().ok());
        size = total.or(size);
    } else if size.is_none() {
        size = probe.content_length();
    }

    let size = size.ok_or_else(|| anyhow!("server did not provide a usable resource size"))?;
    Ok(ResourceInfo { size, supports_ranges })
}

fn make_paths(options: &DownloadOptions) -> Result<Paths> {
    let parent = options.output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = options.output.file_name().and_then(|n| n.to_str()).unwrap_or("download");
    let mut hasher = Sha256::new();
    hasher.update(options.url.as_bytes());
    hasher.update(options.output.to_string_lossy().as_bytes());
    let id = hex::encode(&hasher.finalize()[..12]);
    let work_dir = parent.join(".hyperload").join(format!("{name}-{id}"));
    Ok(Paths {
        manifest: work_dir.join("manifest.json"),
        chunks_dir: work_dir.join("chunks"),
        work_dir,
    })
}

async fn load_or_create_manifest(
    paths: &Paths,
    url: &str,
    size: u64,
    chunk_size: u64,
) -> Result<Manifest> {
    let count = size.div_ceil(chunk_size) as usize;
    if fs::try_exists(&paths.manifest).await.unwrap_or(false) {
        let raw = fs::read(&paths.manifest).await?;
        if let Ok(mut existing) = serde_json::from_slice::<Manifest>(&raw) {
            if existing.version == 1
                && existing.url == url
                && existing.size == size
                && existing.chunk_size == chunk_size
                && existing.chunks_complete.len() == count
            {
                for (index, done) in existing.chunks_complete.iter_mut().enumerate() {
                    if *done {
                        let expected = chunk_len(size, chunk_size, index);
                        let actual = fs::metadata(chunk_path(paths, index)).await
                            .map(|m| m.len()).unwrap_or(0);
                        if actual != expected {
                            *done = false;
                        }
                    }
                }
                return Ok(existing);
            }
        }
        warn!("resume manifest did not match current resource; starting a fresh transfer");
    }

    Ok(Manifest {
        version: 1,
        url: url.to_string(),
        size,
        chunk_size,
        chunks_complete: vec![false; count],
    })
}

async fn save_manifest(paths: &Paths, manifest: &Manifest) -> Result<()> {
    let temp = paths.manifest.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(manifest)?;
    fs::write(&temp, bytes).await?;
    fs::rename(&temp, &paths.manifest).await?;
    Ok(())
}

fn chunk_path(paths: &Paths, index: usize) -> PathBuf {
    paths.chunks_dir.join(format!("{index:08}.part"))
}

fn chunk_len(size: u64, chunk_size: u64, index: usize) -> u64 {
    let start = index as u64 * chunk_size;
    (size - start).min(chunk_size)
}

async fn download_parallel(
    client: &Client,
    options: &DownloadOptions,
    paths: &Paths,
    size: u64,
) -> Result<()> {
    let mut manifest = load_or_create_manifest(paths, &options.url, size, options.chunk_size).await?;
    save_manifest(paths, &manifest).await?;
    let state = Arc::new(Mutex::new(manifest.clone()));
    let jobs: Vec<usize> = manifest.chunks_complete.iter().enumerate()
        .filter_map(|(i, done)| if *done { None } else { Some(i) }).collect();

    let client = client.clone();
    let options = options.clone();
    let paths = paths.clone();
    let state_for_jobs = state.clone();

    let results = stream::iter(jobs.into_iter().map(|index| {
        let client = client.clone();
        let options = options.clone();
        let paths = paths.clone();
        let state = state_for_jobs.clone();
        async move {
            let start = index as u64 * options.chunk_size;
            let end = start + chunk_len(size, options.chunk_size, index) - 1;
            let destination = chunk_path(&paths, index);
            download_chunk(&client, &options.url, start, end, &destination, options.retries).await
                .with_context(|| format!("chunk {index} failed"))?;
            let mut guard = state.lock().await;
            guard.chunks_complete[index] = true;
            save_manifest(&paths, &guard).await?;
            debug!("completed chunk {index}");
            Ok::<(), anyhow::Error>(())
        }
    }))
    .buffer_unordered(options.connections)
    .collect::<Vec<_>>()
    .await;

    for result in results {
        result?;
    }
    manifest = state.lock().await.clone();
    if manifest.chunks_complete.iter().any(|done| !done) {
        bail!("one or more chunks are incomplete");
    }
    Ok(())
}

async fn download_chunk(
    client: &Client,
    url: &str,
    start: u64,
    end: u64,
    destination: &Path,
    retries: usize,
) -> Result<()> {
    let expected_len = end - start + 1;
    for attempt in 0..=retries {
        let result = async {
            let mut response = client.get(url)
                .header(header::RANGE, format!("bytes={start}-{end}"))
                .send().await?;
            if response.status() != StatusCode::PARTIAL_CONTENT {
                bail!("expected HTTP 206 Partial Content, received {}", response.status());
            }
            let content_range = response.headers().get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok()).unwrap_or("");
            if !content_range.starts_with(&format!("bytes {start}-{end}/")) {
                bail!("unexpected Content-Range: {content_range}");
            }

            let mut file = File::create(destination).await?;
            let mut written = 0u64;
            while let Some(item) = response.chunk().await? {
                written += item.len() as u64;
                if written > expected_len {
                    bail!("server sent more bytes than requested");
                }
                file.write_all(&item).await?;
            }
            file.flush().await?;
            file.sync_all().await?;
            if written != expected_len {
                bail!("truncated chunk: expected {expected_len}, received {written}");
            }
            Ok::<(), anyhow::Error>(())
        }.await;

        match result {
            Ok(()) => return Ok(()),
            Err(error) => {
                let _ = fs::remove_file(destination).await;
                if attempt == retries {
                    return Err(error);
                }
                let delay = 2u64.saturating_pow(attempt.min(6) as u32);
                warn!("chunk retry {}/{} after error: {}", attempt + 1, retries, error);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
    unreachable!()
}

async fn download_sequential(
    client: &Client,
    options: &DownloadOptions,
    paths: &Paths,
    size: u64,
) -> Result<()> {
    let destination = chunk_path(paths, 0);
    if fs::metadata(&destination).await.map(|m| m.len()).unwrap_or(0) == size {
        return Ok(());
    }
    for attempt in 0..=options.retries {
        let result = async {
            let mut response = client.get(&options.url).send().await?;
            if !response.status().is_success() {
                bail!("download failed with HTTP {}", response.status());
            }
            let mut file = File::create(&destination).await?;
            let mut received = 0u64;
            while let Some(chunk) = response.chunk().await? {
                received += chunk.len() as u64;
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
            file.sync_all().await?;
            if received != size {
                bail!("truncated download: expected {size}, received {received}");
            }
            Ok::<(), anyhow::Error>(())
        }.await;
        match result {
            Ok(()) => return Ok(()),
            Err(error) if attempt < options.retries => {
                warn!("sequential retry {}: {}", attempt + 1, error);
                let _ = fs::remove_file(&destination).await;
                tokio::time::sleep(Duration::from_secs(2u64.saturating_pow(attempt.min(6) as u32))).await;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

async fn assemble(options: &DownloadOptions, paths: &Paths, size: u64, parallel: bool) -> Result<()> {
    let temp_output = options.output.with_extension("hyperload.tmp");
    let mut output = OpenOptions::new().create(true).truncate(true).write(true)
        .open(&temp_output).await?;
    let count = if parallel { size.div_ceil(options.chunk_size) as usize } else { 1 };

    for index in 0..count {
        let chunk = chunk_path(paths, index);
        let mut input = File::open(&chunk).await
            .with_context(|| format!("missing chunk file {}", chunk.display()))?;
        tokio::io::copy(&mut input, &mut output).await?;
    }
    output.flush().await?;
    output.sync_all().await?;
    drop(output);

    let assembled_size = fs::metadata(&temp_output).await?.len();
    if assembled_size != size {
        let _ = fs::remove_file(&temp_output).await;
        bail!("assembled file size mismatch: expected {size}, got {assembled_size}");
    }
    if fs::try_exists(&options.output).await? {
        fs::remove_file(&options.output).await?;
    }
    fs::rename(&temp_output, &options.output).await?;
    Ok(())
}

async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_http_scheme_and_limits() {
        let options = DownloadOptions {
            url: "ftp://example.com/file".into(),
            output: "file.bin".into(),
            connections: 8,
            chunk_size: 16 * 1024 * 1024,
            retries: 3,
            expected_sha256: None,
            request_timeout: Duration::from_secs(60),
        };
        assert!(options.validate().is_err());
    }

    #[test]
    fn computes_last_chunk_length() {
        assert_eq!(chunk_len(25, 10, 0), 10);
        assert_eq!(chunk_len(25, 10, 1), 10);
        assert_eq!(chunk_len(25, 10, 2), 5);
    }

    #[test]
    fn rejects_invalid_hash() {
        let mut options = DownloadOptions {
            url: "https://example.com/file".into(),
            output: "file.bin".into(),
            connections: 8,
            chunk_size: 1024 * 1024,
            retries: 3,
            expected_sha256: Some("bad".into()),
            request_timeout: Duration::from_secs(60),
        };
        assert!(options.validate().is_err());
        options.expected_sha256 = None;
        assert!(options.validate().is_ok());
    }
}
