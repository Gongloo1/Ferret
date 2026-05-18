# FERRET

```text

███████╗███████╗██████╗ ██████╗ ███████╗████████╗
██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝╚══██╔══╝
█████╗  █████╗  ██████╔╝██████╔╝█████╗     ██║   
██╔══╝  ██╔══╝  ██╔══██╗██╔══██╗██╔══╝     ██║   
██║     ███████╗██║  ██║██║  ██║███████╗   ██║   
╚═╝     ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝   ╚═╝ 
```

**FERRET** — A blazing fast, multi-threaded command-line search utility built with Rust. It uses zero-copy memory mapping (`memmap2`) and data parallelism (`rayon`) to scan massive folders for matching text across all CPU cores instantly.

## Features
- **Ultra-Fast Processing:** Uses memory mapping to read files directly without expensive memory copying, processing files as raw byte slices (`&[u8]`).
- **Multi-Threaded:** Automatically scales across all available CPU threads to search directories concurrently.
- **Dual Modes:** Pass arguments directly via the CLI for scripting, or run it bare to open a clean interactive text menu.
- **Graceful Exit:** Safely intercepts Ctrl+C signals to stop multi-threaded execution cleanly without crashing.
- **Config Support:** Automatically reads defaults from a local `config.toml` if CLI arguments are missing.

## Getting Started
1. **Build from Source**
Make sure you have Rust installed, then clone and build the release binary:

```bash
git clone [https://github.com/Gongloo1/ferret.git](https://github.com/Gongloo1/ferret.git)
cd ferret
cargo build --release
```
2. **How to Run**
### Interactive Mode
Launch without parameters to open the menu UI:

```bash
cargo run
```
### Direct Mode
Search a folder instantly straight from the terminal:

```bash
cargo run -- "your_search_pattern" "./target_directory"
```
## License
This project is licensed under the MIT License.
