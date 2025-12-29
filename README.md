# Stabilized 10k+ RPS Google API Key Miner

This is a high-performance, asynchronous tool written in Rust for discovering valid Google API keys. It is designed to be highly efficient, leveraging several advanced techniques to achieve a high rate of key validation checks.

## Features

*   **High Concurrency:** Utilizes `tokio` to perform thousands of parallel requests.
*   **Optimized Performance:**
    *   Uses `mimalloc` as the global allocator for improved memory management speed.
    *   Implements a sharded client strategy, distributing requests across multiple pinned Google IP addresses to bypass rate limits and improve throughput.
    *   Zero-allocation URL construction in the hot path.
*   **Resumable Sessions:** Automatically saves progress to `checkpoint.json`, allowing you to stop and resume the mining process without losing your place.
*   **Efficient Key Generation:** Generates unique, deterministic key candidates based on a permutation algorithm.
*   **Real-time Statistics:** Displays the current search index, number of valid and invalid keys found, and the real-time check speed (keys/second).
*   **Organized Output:** Saves successfully validated keys to `success_keys.txt`.

## How It Works

1.  **Initialization:** The tool sets a high file descriptor limit and initializes a pool of `reqwest` clients, each pinned to a specific Google IP address. This "sharding" helps distribute the load.
2.  **State Restoration:** It loads the last checked index and the generation seed from `checkpoint.json`. If the file doesn't exist, it starts a new session with a random seed.
3.  **Key Generation:** A deterministic permutation algorithm generates a stream of potential 39-character API key candidates starting with `AIzaSy`.
4.  **Asynchronous Validation:** The stream of keys is processed by a buffered, unordered pool of asynchronous tasks. Each task sends a request to the Google API using one of the sharded clients.
5.  **Result Handling:**
    *   If a key results in a `200 OK`, `403 Forbidden`, or `429 Too Many Requests` response, it is considered potentially valid and is saved.
    *   Valid keys are written to `success_keys.txt`.
    *   Statistics are continuously updated in the console.
6.  **Checkpointing:** The current index is periodically saved to `checkpoint.json` to ensure the session can be resumed later.

## Requirements

*   [Rust](https://www.rust-lang.org/tools/install) programming language and Cargo package manager.

## Usage

1.  **Build the project in release mode for maximum performance:**
    ```bash
    cargo build --release
    ```

2.  **Run the tool:**
    ```bash
    ./target/release/key_checker
    ```

3.  **Specify the level of concurrency (default is 5000):**
    ```bash
    ./target/release/key_checker --parallel 10000
    ```

## Disclaimer

This tool is intended for educational and research purposes only. The use of this tool to find and use Google API keys may violate Google's terms of service. The author is not responsible for any misuse of this tool.
