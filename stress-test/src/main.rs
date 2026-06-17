use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Parser, Debug, Clone)]
#[command(about = "cocompute single-host stress test: concurrency sweep across input-token tiers")]
struct Args {
    /// Proxy base URL, e.g. https://cocompute.ai
    #[arg(long, env = "COCOMPUTE_URL", default_value = "http://localhost:3000")]
    base_url: String,

    /// API key (Bearer token)
    #[arg(long, env = "COCOMPUTE_API_KEY")]
    api_key: String,

    /// Model identifier
    #[arg(long, default_value = "gemma4:31b")]
    model: String,

    /// Output directory for runs.csv and summary.md
    #[arg(long, default_value = "bench/stress-3090")]
    output_dir: PathBuf,

    /// Input-token tiers to sweep (comma-separated)
    #[arg(long, value_delimiter = ',', default_values_t = vec![10_000usize, 50_000])]
    input_tokens: Vec<usize>,

    /// Concurrency levels to sweep (comma-separated)
    #[arg(long, value_delimiter = ',', default_values_t = vec![5usize, 10, 15, 20, 25])]
    concurrency: Vec<usize>,

    /// Target output tokens per request
    #[arg(long, default_value_t = 1000)]
    max_output_tokens: u32,

    /// Stop a cell after this many completed requests OR --max-duration-secs, whichever first.
    /// Default: 3 * concurrency.
    #[arg(long)]
    completions_per_cell: Option<usize>,

    /// Per-cell timeout (seconds)
    #[arg(long, default_value_t = 600)]
    max_duration_secs: u64,

    /// Cool-down between cells (seconds)
    #[arg(long, default_value_t = 60)]
    cool_down_secs: u64,

    /// Per-request HTTP timeout (seconds)
    #[arg(long, default_value_t = 1800)]
    request_timeout_secs: u64,

    /// Use SSE streaming (stream: true). Captures TTFT and avoids long-idle
    /// HTTP connections that ingress proxies may cancel.
    #[arg(long)]
    stream: bool,

    /// Don't actually call the API — print the plan and exit.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize, Default, Clone)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: Option<StreamDelta>,
}

#[derive(Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Serialize)]
struct Row {
    run_id: String,
    cell: String,
    input_tokens_target: usize,
    concurrency: usize,
    request_index: usize,
    started_at: String,
    latency_ms: u128,
    ttft_ms: Option<u128>,
    http_status: u16,
    error_class: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let run_id = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let out_dir = args.output_dir.join(&run_id);

    println!("run_id            = {}", run_id);
    println!("output_dir        = {}", out_dir.display());
    println!("base_url          = {}", args.base_url);
    println!("model             = {}", args.model);
    println!("input_tokens      = {:?}", args.input_tokens);
    println!("concurrency       = {:?}", args.concurrency);
    println!("max_output_tokens = {}", args.max_output_tokens);
    println!("streaming         = {}", args.stream);
    println!(
        "cells total       = {}",
        args.input_tokens.len() * args.concurrency.len()
    );

    if args.dry_run {
        println!("\n[dry-run] exiting without sending requests");
        return Ok(());
    }

    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let csv_path = out_dir.join("runs.csv");
    let summary_path = out_dir.join("summary.md");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(args.request_timeout_secs))
        .build()
        .context("building reqwest client")?;

    let (tx, mut rx) = mpsc::channel::<Row>(1024);

    let csv_writer_handle = tokio::spawn(async move {
        let mut w = csv::Writer::from_path(&csv_path)
            .with_context(|| format!("creating {}", csv_path.display()))?;
        while let Some(row) = rx.recv().await {
            w.serialize(&row).context("writing csv row")?;
            w.flush().context("flushing csv")?;
        }
        Ok::<_, anyhow::Error>(())
    });

    let mut summary_rows: Vec<CellSummary> = Vec::new();

    for &input_target in &args.input_tokens {
        let prompt = build_prompt(input_target, args.max_output_tokens);

        for &concurrency in &args.concurrency {
            let cell = format!("in{}_c{}", input_target, concurrency);
            let target_completions = args
                .completions_per_cell
                .unwrap_or(concurrency.saturating_mul(3).max(1));

            println!(
                "\n[cell {}] target_completions={} max_duration={}s",
                cell, target_completions, args.max_duration_secs
            );

            let cell_summary = run_cell(
                &client,
                &args,
                &cell,
                &run_id,
                &prompt,
                input_target,
                concurrency,
                target_completions,
                tx.clone(),
            )
            .await?;

            println!(
                "[cell {}] done: completed={} failed={} p50={}ms p95={}ms p99={}ms ttft_p50={}ms ttft_p95={}ms out_tok_total={} out_tok_mean={:.0} out_tok/s={:.1}",
                cell,
                cell_summary.completed,
                cell_summary.failed,
                cell_summary.p50_ms,
                cell_summary.p95_ms,
                cell_summary.p99_ms,
                cell_summary.ttft_p50_ms,
                cell_summary.ttft_p95_ms,
                cell_summary.total_output_tokens,
                cell_summary.mean_output_tokens,
                cell_summary.output_tokens_per_sec,
            );

            summary_rows.push(cell_summary);

            let is_last = input_target == *args.input_tokens.last().unwrap()
                && concurrency == *args.concurrency.last().unwrap();
            if !is_last && args.cool_down_secs > 0 {
                println!("cool-down {}s …", args.cool_down_secs);
                tokio::time::sleep(Duration::from_secs(args.cool_down_secs)).await;
            }
        }
    }

    drop(tx);
    csv_writer_handle.await.context("csv writer join")??;

    write_summary(&summary_path, &args, &run_id, &summary_rows)?;
    println!("\nsummary: {}", summary_path.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_cell(
    client: &reqwest::Client,
    args: &Args,
    cell: &str,
    run_id: &str,
    prompt: &str,
    input_target: usize,
    concurrency: usize,
    target_completions: usize,
    tx: mpsc::Sender<Row>,
) -> Result<CellSummary> {
    let cell_start = Instant::now();
    let deadline = cell_start + Duration::from_secs(args.max_duration_secs);
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::<u128>::new()));
    let ttfts = Arc::new(tokio::sync::Mutex::new(Vec::<u128>::new()));
    let failures = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let completion_tokens_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let streaming = args.stream;

    let mut workers = JoinSet::new();
    let prompt = Arc::new(prompt.to_string());

    for _ in 0..concurrency {
        let client = client.clone();
        let url = format!(
            "{}/v1/chat/completions",
            args.base_url.trim_end_matches('/')
        );
        let api_key = args.api_key.clone();
        let model = args.model.clone();
        let max_output = args.max_output_tokens;
        let prompt = prompt.clone();
        let counter = counter.clone();
        let latencies = latencies.clone();
        let ttfts = ttfts.clone();
        let failures = failures.clone();
        let completion_tokens_total = completion_tokens_total.clone();
        let tx = tx.clone();
        let cell = cell.to_string();
        let run_id = run_id.to_string();

        workers.spawn(async move {
            loop {
                if Instant::now() >= deadline {
                    return;
                }
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if idx >= target_completions {
                    return;
                }

                let body = ChatRequest {
                    model: &model,
                    messages: vec![ChatMessage {
                        role: "user",
                        content: &prompt,
                    }],
                    stream: streaming,
                    max_tokens: max_output,
                    stream_options: if streaming {
                        Some(StreamOptions { include_usage: true })
                    } else {
                        None
                    },
                };

                let started_at = chrono::Utc::now().to_rfc3339();
                let start = Instant::now();
                let resp = client
                    .post(&url)
                    .bearer_auth(&api_key)
                    .json(&body)
                    .send()
                    .await;

                let row = match resp {
                    Ok(r) => {
                        let status = r.status();
                        let (ttft_ms, usage, error_class) = if streaming && status.is_success() {
                            read_stream(&start, r).await
                        } else {
                            let body_text = r.text().await.unwrap_or_default();
                            let parsed: Option<ChatResponse> =
                                serde_json::from_str(&body_text).ok();
                            let usage = parsed
                                .as_ref()
                                .and_then(|p| p.usage.clone())
                                .unwrap_or_default();
                            let err = if status.is_success() && parsed.is_some() {
                                String::new()
                            } else if !status.is_success() {
                                format!("http_{}: {}", status.as_u16(), truncate(&body_text, 160))
                            } else {
                                format!("parse_error: {}", truncate(&body_text, 160))
                            };
                            (None, usage, err)
                        };
                        let latency_ms = start.elapsed().as_millis();
                        if error_class.is_empty() {
                            latencies.lock().await.push(latency_ms);
                            if let Some(t) = ttft_ms {
                                ttfts.lock().await.push(t);
                            }
                            completion_tokens_total.fetch_add(
                                usage.completion_tokens as u64,
                                std::sync::atomic::Ordering::SeqCst,
                            );
                        } else {
                            failures.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                        Row {
                            run_id: run_id.clone(),
                            cell: cell.clone(),
                            input_tokens_target: input_target,
                            concurrency,
                            request_index: idx,
                            started_at,
                            latency_ms,
                            ttft_ms,
                            http_status: status.as_u16(),
                            error_class,
                            prompt_tokens: usage.prompt_tokens,
                            completion_tokens: usage.completion_tokens,
                            total_tokens: usage.total_tokens,
                        }
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis();
                        failures.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        Row {
                            run_id: run_id.clone(),
                            cell: cell.clone(),
                            input_tokens_target: input_target,
                            concurrency,
                            request_index: idx,
                            started_at,
                            latency_ms,
                            ttft_ms: None,
                            http_status: 0,
                            error_class: classify_send_error(&e),
                            prompt_tokens: 0,
                            completion_tokens: 0,
                            total_tokens: 0,
                        }
                    }
                };

                if tx.send(row).await.is_err() {
                    return;
                }
            }
        });
    }

    while workers.join_next().await.is_some() {}

    let mut lat = latencies.lock().await.clone();
    lat.sort_unstable();
    let mut ttft_vec = ttfts.lock().await.clone();
    ttft_vec.sort_unstable();
    let completed = lat.len();
    let failed = failures.load(std::sync::atomic::Ordering::SeqCst);
    let total_output_tokens =
        completion_tokens_total.load(std::sync::atomic::Ordering::SeqCst);
    let mean_output_tokens = if completed > 0 {
        total_output_tokens as f64 / completed as f64
    } else {
        0.0
    };
    let elapsed_secs = cell_start.elapsed().as_secs_f64().max(1e-9);
    let output_tokens_per_sec = total_output_tokens as f64 / elapsed_secs;
    Ok(CellSummary {
        input_tokens_target: input_target,
        concurrency,
        completed,
        failed,
        p50_ms: percentile(&lat, 50.0),
        p95_ms: percentile(&lat, 95.0),
        p99_ms: percentile(&lat, 99.0),
        ttft_p50_ms: percentile(&ttft_vec, 50.0),
        ttft_p95_ms: percentile(&ttft_vec, 95.0),
        total_output_tokens,
        mean_output_tokens,
        output_tokens_per_sec,
    })
}

struct CellSummary {
    input_tokens_target: usize,
    concurrency: usize,
    completed: usize,
    failed: usize,
    p50_ms: u128,
    p95_ms: u128,
    p99_ms: u128,
    ttft_p50_ms: u128,
    ttft_p95_ms: u128,
    total_output_tokens: u64,
    mean_output_tokens: f64,
    output_tokens_per_sec: f64,
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn build_prompt(input_target_tokens: usize, output_target_tokens: u32) -> String {
    // Heuristic: ~4 chars/token for English text on sentencepiece tokenizers.
    // The server reports the actual prompt_tokens — recorded per-request in the CSV.
    let chunk =
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. ";
    let target_chars = input_target_tokens * 4;
    let mut s = String::with_capacity(target_chars + 512);
    let target_words = (output_target_tokens as f32 * 0.75) as u32;
    s.push_str(&format!(
        "You are continuing a long passage. Continue the narrative below with original prose for at least {target_words} words. Do not summarize. Do not stop early. Do not add commentary. Continue writing only the story.\n\nPassage:\n"
    ));
    while s.len() < target_chars {
        s.push_str(chunk);
    }
    s.push_str("\n\nContinuation:\n");
    s
}

async fn read_stream(
    start: &Instant,
    resp: reqwest::Response,
) -> (Option<u128>, Usage, String) {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut ttft: Option<u128> = None;
    let mut chunk_content_count: u32 = 0;
    let mut usage_from_stream: Option<Usage> = None;
    let mut err: Option<String> = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => buf.extend_from_slice(&bytes),
            Err(e) => {
                err = Some(format!("stream_io: {}", truncate(&e.to_string(), 160)));
                break;
            }
        }
        loop {
            let Some(pos) = find_double_newline(&buf) else { break };
            let event = String::from_utf8_lossy(&buf[..pos]).into_owned();
            buf.drain(..pos + 2);
            for line in event.lines() {
                let Some(data) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) else { continue };
                for choice in &chunk.choices {
                    if let Some(content) = choice.delta.as_ref().and_then(|d| d.content.as_ref()) {
                        if !content.is_empty() {
                            if ttft.is_none() {
                                ttft = Some(start.elapsed().as_millis());
                            }
                            chunk_content_count += 1;
                        }
                    }
                }
                if let Some(u) = chunk.usage {
                    usage_from_stream = Some(u);
                }
            }
        }
    }

    let mut usage = usage_from_stream.unwrap_or_default();
    if usage.completion_tokens == 0 && chunk_content_count > 0 {
        usage.completion_tokens = chunk_content_count;
    }
    (ttft, usage, err.unwrap_or_default())
}

fn find_double_newline(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

fn classify_send_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "timeout".into()
    } else if e.is_connect() {
        "connect".into()
    } else if e.is_request() {
        "request".into()
    } else {
        format!("send_error: {}", truncate(&e.to_string(), 120))
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

fn write_summary(
    path: &std::path::Path,
    args: &Args,
    run_id: &str,
    rows: &[CellSummary],
) -> Result<()> {
    use std::fmt::Write as _;
    let mut s = String::new();
    writeln!(s, "# stress-test summary — {}", run_id)?;
    writeln!(s)?;
    writeln!(s, "- base_url: `{}`", args.base_url)?;
    writeln!(s, "- model: `{}`", args.model)?;
    writeln!(s, "- max_output_tokens: {}", args.max_output_tokens)?;
    writeln!(s, "- streaming: {}", args.stream)?;
    writeln!(s, "- max_duration_secs: {}", args.max_duration_secs)?;
    writeln!(s, "- cool_down_secs: {}", args.cool_down_secs)?;
    writeln!(s)?;
    writeln!(
        s,
        "| input_tokens | concurrency | completed | failed | p50 ms | p95 ms | p99 ms | ttft p50 ms | ttft p95 ms | out_tok_total | out_tok_mean | out_tok/s |"
    )?;
    writeln!(
        s,
        "| ------------ | ----------- | --------- | ------ | ------ | ------ | ------ | ----------- | ----------- | ------------- | ------------ | --------- |"
    )?;
    for r in rows {
        writeln!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} | {:.1} |",
            r.input_tokens_target,
            r.concurrency,
            r.completed,
            r.failed,
            r.p50_ms,
            r.p95_ms,
            r.p99_ms,
            r.ttft_p50_ms,
            r.ttft_p95_ms,
            r.total_output_tokens,
            r.mean_output_tokens,
            r.output_tokens_per_sec,
        )?;
    }
    std::fs::write(path, s).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
