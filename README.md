# Key Checker

A fast, optimized Rust CLI tool to check the validity of Google API keys by testing them against the Google Maps Geocoding API.

## Usage

```bash
cargo run -- --keys "AIzaSyDUSHGDs3JbC_w7oeExYkzVMA6OpJnUjbM AIzaSyBNMVbj1kCx2JkSQolohVUyLdGOiU3O7Is"
```

## Installation

1. Ensure Rust is installed.
2. Clone or download the project.
3. Run `cargo build --release` to build the optimized binary.

## Features

- Asynchronous checks for speed.
- Parallel processing of multiple keys.
- Robust error handling.
- Simple CLI interface.

## Troubleshooting

- If keys are invalid, ensure they are correct Google API keys with Maps API enabled.
- Check internet connection for API requests.
- For rate limits, add delays if needed (not implemented).

# Key Checker - Mining Mode

This tool continuously mines for valid Google API keys and saves them to `output.txt`.

## Features

- **Mining Mode**: Continuously checks keys in a loop.
- **Output File**: Saves valid keys to `output.txt`.
- **Customizable Delay**: Set delay between checks using `--delay`.
- **Error Handling**: Handles API errors gracefully.

## Usage

```bash
cargo run -- --keys "key1 key2 key3" --delay 2
```

- Replace `key1 key2 key3` with your API keys.
- Use `--delay` to set the delay between checks (default: 1 second).

## Output

- Valid keys are appended to `output.txt` in the project directory.

## Installation

1. Ensure Rust is installed.
2. Clone or download the project.
3. Run `cargo build --release` to build the optimized binary.

## Troubleshooting

- Ensure keys are valid Google API keys with Maps API enabled.
- Check internet connection for API requests.
- For rate limits, increase the delay using `--delay`.

Happy mining!