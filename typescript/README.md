# @cooklang/cooklang-ts

A TypeScript/JavaScript library for parsing CookLang recipes, built with Rust and WebAssembly.

## Prerequisites

This project uses Rust compiled to WebAssembly, so you'll need to set up the Rust toolchain properly before development.

### Required Tools

1. **Rust via rustup** (recommended over system package managers)
2. **wasm-pack** 
3. **Node.js**

### Setup Instructions

#### 1. Install Rust (via rustup)

**Important:** Use rustup rather than system package managers (Homebrew on macOS, apt on Ubuntu, etc.) for better toolchain management.

If you have Rust installed via a package manager, consider uninstalling it first:
```bash
# macOS (Homebrew)
brew uninstall rust

# Ubuntu/Debian
sudo apt remove rustc

# Arch Linux
sudo pacman -R rust
```

Install Rust using rustup. Platform specific instructions at [rustup.rs](https://rustup.rs/).

After installation, reload your shell environment:
```bash
# Unix-like systems (macOS, Linux)
source ~/.cargo/env

# Windows (restart your terminal or run)
# The installer usually handles PATH automatically
```

#### 2. Add WebAssembly Target

```bash
rustup target add wasm32-unknown-unknown
```

#### 3. Install wasm-pack

```bash
cargo install wasm-pack
```

#### 4. Verify Installation

```bash
# Check Rust
rustc --version
rustup target list --installed | grep wasm32

# Check wasm-pack
wasm-pack --version

# Verify WASM target is available
rustc --print target-list | grep wasm32-unknown-unknown
```

## Development

### Install Dependencies

From the project root:
```bash
npm install
```

### Playground Development

From the project root:
```bash
npm run playground
```

### Project Structure

- `src/` - Rust source code (extensions to cooklang-rs)
- `pkg/` - Generated WebAssembly files (created by wasm-pack)
- `index.ts` - TypeScript entry point
- `Cargo.toml` - Rust dependencies and configuration
