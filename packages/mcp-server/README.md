# DiskRipper MCP Server

Model Context Protocol server that enables AI agents (including Hermes) to automatically backup optical media (CDs, DVDs, Blu-rays).

## Quick Start

```bash
cd packages/mcp-server
npm install
npm run build

# Test it
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | node dist/cli.js
```

## Tools

| Tool | Description |
|------|-------------|
| `list_drives` | List all optical drives |
| `drive_info` | Get detailed drive/disc information |
| `rip_disc` | Rip disc to ISO/BIN/IMG image |
| `extract_files` | Extract files from disc to directory |
| `rip_audio_cd` | Rip audio CD to WAV/FLAC |
| `verify_image` | Verify image against original disc |
| `list_jobs` | List all backup jobs |
| `job_status` | Get specific job status |
| `cancel_job` | Cancel running job |

## Configuration

Add to `~/.hermes/config.yaml`:

```yaml
mcp_servers:
  diskripper:
    command: node
    args:
      - C:/Projects/DiskRipper/packages/mcp-server/dist/cli.js
    timeout: 300
    connect_timeout: 120
```

Set the environment variable to point to the diskripper binary:

```bash
# Windows (PowerShell)
$env:DISKRIPPER_BIN = "C:\Projects\DiskRipper\target\debug\diskripper.exe"

# Linux/macOS
export DISKRIPPER_BIN="/path/to/diskripper"
```

## Building the Rust CLI

```bash
cargo build -p diskripper-core --bin diskripper
# Output: target/debug/diskripper.exe
```

## Architecture

```
Hermes Agent → MCP (stdio) → diskripper-mcp → CLI → diskripper-core
```

The MCP server communicates with the Rust CLI binary via subprocess calls. The CLI does the actual optical media work (imaging, extraction, verification).
