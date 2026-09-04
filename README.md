# DiskRipper

<div align="center">

**Next generation media backup software with AI-powered content identification**

[![CI](https://github.com/LoopyLuci/DiskRipper/actions/workflows/ci.yml/badge.svg)](https://github.com/LoopyLuci/DiskRipper/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE
[![Release](https://img.shields.io/github/v/release/LoopyLuci/DiskRipper)](https://github.com/LoopyLuci/DiskRipper/releases

[Features](#features) • [Installation](#installation) • [Quick Start](#quick-start) • [Documentation](docs/USER_GUIDE.md) • [MCP Server](packages/mcp-server/README.md)

</div>

---

## Features

### Optical Media Backup
- **CDs** — Audio, data, mixed-mode, Enhanced CD
- **DVDs** — Single/dual layer, DVD-Video
- **Blu-ray** — Single/dual layer, BDMV

### AI-Powered Identification
- **Custom audio fingerprinting** — Identifies music from actual audio (AcoustID replacement)
- **Hybrid ML pipeline** — Combines fingerprinting + classification + metadata
- **Self-learning** — Improves from user feedback
- **Smart organization** — Auto-sorts ripped content into folders

### Cross-Platform
- **Windows** (MSI installer)
- **macOS** (DMG)
- **Linux** (AppImage, DEB)

### Interfaces
- **GUI** — Modern React/TypeScript desktop app (Tauri 2)
- **CLI** — Full command-line automation with JSON output
- **MCP Server** — AI agent integration (Hermes, etc.)

---

## Installation

### Windows
Download the latest `.msi` from [Releases](https://github.com/LoopyLuci/DiskRipper/releases).

### macOS
Download the latest `.dmg` from [Releases](https://github.com/LoopyLuci/DiskRipper/releases).

### Linux
Download the latest `.AppImage` or `.deb` from [Releases](https://github.com/LoopyLuci/DiskRipper/releases).

---

## Quick Start

### GUI

1. Insert a disc
2. Open DiskRipper
3. Select your drive
4. Click "Rip"

### CLI

```bash
# List drives
diskripper list-drives

# Rip a disc
diskripper rip --drive D: --output disc.iso

# Extract audio CD
diskripper extract --drive D: --output ./music/

# Batch rip
diskripper batch-rip --drives D:,E: --output-dir ./rips/
```

### MCP Server

```json
{
  "mcpServers": {
    "diskripper": {
      "command": "node",
      "args": ["path/to/packages/mcp-server/dist/cli.js"]
    }
  }
}
```

---

## ML Identification

Identify content from actual audio — no metadata needed.

1. Open the **ML Identify** panel
2. Select a WAV file
3. Click **Identify Content**
4. View results with confidence scores
5. Provide feedback to improve accuracy

**All processing is local. No external API dependencies.**

---

## Building from Source

### Prerequisites
- Rust 1.70+
- Node.js 20+
- PowerShell 7+ (Windows)

### Build

```bash
# Clone
git clone https://github.com/LoopyLuci/DiskRipper.git
cd DiskRipper

# Install dependencies
cd frontend && npm install && cd ..

# Build release
cd diskripper-tauri && cargo build --release

# Build frontend
cd ../frontend && npm run build
```

### Development

```bash
# Run with hot reload
cd frontend && npm run tauri dev
```

---

## Project Structure

```
DiskRipper/
├── diskripper-core/          # Rust backend (50+ modules)
│   ├── src/
│   │   ├── ml/               # ML system (14 modules)
│   │   ├── filesystem/       # Parser modules (ISO 9660, UDF, etc.)
│   │   ├── audio_cd.rs       # Audio CD ripping
│   │   ├── drive.rs          # Drive detection
│   │   ├── rip.rs            # Rip engine
│   │   └── ...
│   └── Cargo.toml
├── diskripper-tauri/         # Tauri 2 shell
│   ├── src/                  # Tauri commands
│   └── tauri.conf.json
├── frontend/                 # React/TypeScript GUI
│   └── src/
│       ├── components/       # 12+ UI components
│       └── store.ts          # Zustand store
├── packages/mcp-server/      # MCP server
│   └── src/                  # 9 tools
├── scripts/                  # Build/packaging
├── docs/                     # Documentation
└── .github/workflows/        # CI/CD
```

---

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   GUI (React)   │     │   CLI (clap)    │
└────────┬────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     │
         ┌───────────▼───────────┐
         │   Tauri Commands      │
         └───────────┬───────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
┌───▼───┐      ┌────▼────┐     ┌────▼────┐
│  Rip  │      │   ML    │     │  Drive  │
│Engine │      │Pipeline │     │ Scanner │
└───┬───┘      └────┬────┘     └────┬────┘
    │               │               │
    └───────────────┼───────────────┘
                    │
         ┌──────────▼──────────┐
         │  diskripper-core    │
         │  (50+ modules)      │
         └─────────────────────┘
```

---

## License

MIT License. See [LICENSE](LICENSE).

---

## Support

- **Issues:** https://github.com/LoopyLuci/DiskRipper/issues
- **Wiki:** https://github.com/LoopyLuci/DiskRipper/wiki
- **Documentation:** [docs/USER_GUIDE.md](docs/USER_GUIDE.md)
