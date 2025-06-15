# T-CLI

High-performance CLI for code-based i18n management. Built with Rust for maximum speed and reliability.

## Features

- 🚀 Lightning-fast Rust implementation
- 🔍 Batch code scanning and key collection
- 🛠️ Automatic generation of translation files
- ⚡ Modern SWC-based parsing, async processing

---

## Installation

### Build from Source

```bash
git clone https://github.com/NuclEnergy/t-cli.git
cd t-cli
cargo install --path .
```

Or for local development:

```bash
cargo build --release
```

---

## Usage

### 1. Init

Generate an initial `t.config.ts` file:

```bash
t-cli init
```

---

### 2. Collect

Scan your codebase and collect all `t` function keys:

```bash
t-cli collect
```

---

### 3. Generate

Generate translation files and TypeScript index from collected keys:

```bash
t-cli generate
```

---

> **Tip:**  
> Run `t-cli --help` for all available options and flags.

---

## Development

### Requirements

- Rust 1.80+
- Cargo

### Build

```bash
cargo build --release
```

---

## FAQ

- **Which languages and frameworks are supported?**  
  Any language defined in config; best used with React/Next.js/TypeScript projects.

- **Is performance an issue on large codebases?**  
  t-cli is optimized for speed with async/multithreaded processing. File issues if you hit a bottleneck.

---

## License

MIT. See [LICENSE](LICENSE) for details.
