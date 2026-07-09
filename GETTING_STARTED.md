# Getting Started with LogLens

This guide walks you through setting up LogLens from scratch, even if you have never used Rust, Node.js or a terminal before. LogLens runs on Windows, Linux and macOS. It has two parts: the **desktop app** (a native window built with Tauri) and the **CLI** (a command-line tool, `loglens`) that works the same way on all three platforms. Pick whichever fits your workflow, or use both.

---

## Windows

### 1. Open a terminal

Right-click the Start button and choose **"Terminal"** (or "Windows PowerShell" on older Windows versions).

### 2. Check prerequisites

Run these commands one by one:

```powershell
rustc --version
cargo --version
node --version
cargo tauri --version
```

If any command prints something like `'rustc' is not recognized as an internal or external command`, that tool is not installed (or not on your PATH):

- **Rust & Cargo missing:** install from [https://rustup.rs](https://rustup.rs) and restart your terminal afterwards.
- **Node.js missing:** install from [https://nodejs.org](https://nodejs.org) (LTS version) and restart your terminal.
- **`cargo tauri` missing:** once Rust/Cargo is installed, run `cargo install tauri-cli`.

### 3. Get the code

**Easiest way (no git required):**
1. Go to [https://github.com/9t29zhmwdh-coder/LogLens](https://github.com/9t29zhmwdh-coder/LogLens)
2. Click the green **"Code"** button → **"Download ZIP"**
3. Extract the ZIP file somewhere convenient (e.g. `Documents\LogLens`)
4. In your terminal, navigate into the extracted folder, e.g. `cd Documents\LogLens`

**If you already use git:**
```powershell
git clone https://github.com/9t29zhmwdh-coder/LogLens.git
cd LogLens
```

### 4. Build & run

**Desktop app:**
```powershell
cargo tauri dev
```
This compiles the Rust backend and the React frontend, then opens the LogLens window automatically. The first run takes a while (Rust and npm dependencies are downloaded and compiled); subsequent runs are much faster.

<!-- TODO: Screenshot -->

**CLI:**
```powershell
cargo build --release -p ll-cli
```
This produces a `loglens` executable under `target\release\loglens.exe`. From there you can run commands like:
```powershell
target\release\loglens.exe watch C:\path\to\app.log --level warn
target\release\loglens.exe search "connection refused"
target\release\loglens.exe clusters --top 20
```

### What you should see

For the desktop app, a native LogLens window opens after the build finishes. For the CLI, commands print their output directly in the terminal (e.g. matched log lines, cluster summaries).

---

## Linux

### 1. Open a terminal

This depends on your desktop environment. Common shortcuts: **Ctrl+Alt+T** (GNOME, many distros), or look for "Terminal" in your application menu (Files/Activities/App Grid depending on your desktop).

### 2. Check prerequisites

```bash
rustc --version
cargo --version
node --version
cargo tauri --version
```

If you get a `command not found` error:

- **Rust & Cargo missing:** install via [https://rustup.rs](https://rustup.rs) (run the one-line install script it gives you), then restart your terminal or run `source $HOME/.cargo/env`.
- **Node.js missing:** install from [https://nodejs.org](https://nodejs.org) or via your distro's package manager.
- **`cargo tauri` missing:** run `cargo install tauri-cli` once Rust is set up.

Tauri apps on Linux also need WebKitGTK and a few system libraries installed (see Troubleshooting below if the build fails with missing `.pc` files).

### 3. Get the code

**Easiest way (no git required):**
1. Go to [https://github.com/9t29zhmwdh-coder/LogLens](https://github.com/9t29zhmwdh-coder/LogLens)
2. Click the green **"Code"** button → **"Download ZIP"**
3. Extract it, e.g. `unzip LogLens-main.zip`
4. `cd` into the extracted folder

**If you already use git:**
```bash
git clone https://github.com/9t29zhmwdh-coder/LogLens.git
cd LogLens
```

### 4. Build & run

**Desktop app:**
```bash
cargo tauri dev
```

**CLI:**
```bash
cargo build --release -p ll-cli
```
The binary is at `target/release/loglens`. Example usage:
```bash
target/release/loglens watch /var/log/app.log --level warn
target/release/loglens watch docker://my-api --ai
target/release/loglens search "connection refused"
```

### What you should see

The desktop app opens as a native window once compilation finishes. The CLI prints log entries, search results or cluster summaries directly to your terminal, depending on the subcommand.

---

## macOS

### 1. Open a terminal

Press **Cmd+Space** to open Spotlight, type **"Terminal"**, and press Enter.

### 2. Check prerequisites

```bash
rustc --version
cargo --version
node --version
cargo tauri --version
```

If you see `command not found`:

- **Rust & Cargo missing:** install via [https://rustup.rs](https://rustup.rs), then restart Terminal or run `source $HOME/.cargo/env`.
- **Node.js missing:** install from [https://nodejs.org](https://nodejs.org) (or via Homebrew: `brew install node`).
- **`cargo tauri` missing:** run `cargo install tauri-cli`.

### 3. Get the code

**Easiest way (no git required):**
1. Go to [https://github.com/9t29zhmwdh-coder/LogLens](https://github.com/9t29zhmwdh-coder/LogLens)
2. Click the green **"Code"** button → **"Download ZIP"**
3. Double-click the downloaded ZIP to extract it
4. Open Terminal and `cd` into the extracted folder

**If you already use git:**
```bash
git clone https://github.com/9t29zhmwdh-coder/LogLens.git
cd LogLens
```

### 4. Build & run

**Desktop app:**
```bash
cargo tauri dev
```

**CLI:**
```bash
cargo build --release -p ll-cli
```
The binary is at `target/release/loglens`. Example usage:
```bash
target/release/loglens search "connection refused"
target/release/loglens clusters --top 20
target/release/loglens analyze <cluster-id>
```

### What you should see

The desktop app opens as its own window (LogLens has no menu-bar icon or background process; it only runs while the window is open). The CLI prints results straight to your terminal.

<!-- TODO: Screenshot -->

---

### Troubleshooting

| Issue | Cause | Fix |
|---|---|---|
| `'cargo' is not recognized` / `cargo: command not found` | Rust is not installed or not on your PATH | Install via [rustup.rs](https://rustup.rs), then restart your terminal |
| `'node' is not recognized` / `node: command not found` | Node.js is not installed or not on your PATH | Install from [nodejs.org](https://nodejs.org), then restart your terminal |
| PowerShell blocks a `.ps1` script with "cannot be loaded because running scripts is disabled" | Windows execution policy restricts script execution | Run `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` in PowerShell, or run the equivalent `cargo`/`npm` command directly instead of a script |
| Build fails on Windows with linker errors or messages about `link.exe` / MSVC | Missing C++ build tools required by some Rust crates | Install "Desktop development with C++" via [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) |
| `cargo tauri dev` fails on Linux with errors about `webkit2gtk` / `javascriptcoregtk` not found | Missing WebKitGTK system libraries required by Tauri | Install the WebKitGTK dev package for your distro, e.g. `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` on Debian/Ubuntu |
