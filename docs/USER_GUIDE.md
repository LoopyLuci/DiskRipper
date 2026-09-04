# DiskRipper User Guide

DiskRipper is a cross-platform desktop application for backing up optical media (CDs, DVDs, Blu-rays) with AI-powered content identification and smart organization.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Features](#features)
4. [Using the GUI](#using-the-gui)
5. [Using the CLI](#using-the-cli)
6. [Using the MCP Server](#using-the-mcp-server)
7. [ML Content Identification](#ml-content-identification)
8. [Settings](#settings)
9. [Keyboard Shortcuts](#keyboard-shortcuts)
10. [Troubleshooting](#troubleshooting)
11. [FAQ](#faq)

---

## Installation

### Windows

1. Download the latest `.msi` installer from [GitHub Releases](https://github.com/LoopyLuci/DiskRipper/releases)
2. Run the installer and follow the prompts
3. Launch DiskRipper from the Start menu

**Note:** Windows SmartScreen may warn about an unsigned application. Click "More info" → "Run anyway" until we obtain a code signing certificate.

### macOS

1. Download the latest `.dmg` from [GitHub Releases](https://github.com/LoopyLuci/DiskRipper/releases)
2. Open the DMG and drag DiskRipper to Applications
3. Launch from Applications

### Linux

1. Download the latest `.AppImage` or `.deb` from [GitHub Releases](https://github.com/LoopyLuci/DiskRipper/releases)
2. For AppImage: `chmod +x DiskRipper.AppImage && ./DiskRipper.AppImage`
3. For DEB: `sudo dpkg -i diskripper.deb`

---

## Quick Start

1. **Insert a disc** into your optical drive
2. **Open DiskRipper** — the app automatically detects drives
3. **Select your drive** from the Drives panel
4. **Choose output location** in Settings or use the default
5. **Click "Rip"** to create an ISO image, or **"Extract"** to copy files
6. **Wait for completion** — progress shows speed, ETA, and bytes processed

---

## Features

### Disc Support

| Media Type | Read | Verify | Extract |
|------------|------|--------|---------|
| CD-DA (Audio CD) | ✅ | ✅ | ✅ |
| CD-ROM (Data) | ✅ | ✅ | ✅ |
| CD-R / CD-RW | ✅ | ✅ | ✅ |
| DVD-ROM | ✅ | ✅ | ✅ |
| DVD±R / DVD±RW | ✅ | ✅ | ✅ |
| DVD-DL (Dual Layer) | ✅ | ✅ | ✅ |
| Blu-ray | ✅ | ✅ | ✅ |
| Blu-ray DL | ✅ | ✅ | ✅ |
| Mixed Mode CD | ✅ | ✅ | ✅ |
| Enhanced CD | ✅ | ✅ | ✅ |
| CD-i | ✅ | ✅ | ✅ |

### Filesystems Supported

- **ISO 9660** (with Joliet, Rock Ridge extensions)
- **UDF** (1.02 through 2.60)
- **HFS / HFS+** (Macintosh)
- **Hybrid** (ISO + HFS)
- **DVD-Video** (VOB/IFO parser)
- **Blu-ray** (BDMV parser)

### Audio Formats

- **WAV** (44.1kHz/16-bit/stereo, CD-quality)
- **FLAC** (lossless compression)
- **CUE sheets** (with track timestamps)

---

## Using the GUI

### Drives Panel

The Drives panel shows all detected optical drives and their status.

- **Drive icon** — optical drive detected
- **Disc indicator** — disc is loaded
- **Drive type** — CD, DVD, or Blu-ray
- **Disc info** — type, size, filesystem, tracks

**Actions:**
- Click a drive to select it
- Click "Refresh" to rescan drives
- Click "Analyze" to get detailed disc information

### Jobs Panel

The Jobs panel shows all active and completed jobs.

**Job statuses:**
- 🟡 **Waiting** — queued to start
- 🔵 **Running** — in progress
- 🟢 **Completed** — finished successfully
- 🔴 **Failed** — error occurred
- ⚪ **Cancelled** — user cancelled

**Actions:**
- Filter jobs by status
- Cancel running jobs
- Remove completed jobs from list
- View error details for failed jobs

### Audio CD Panel

Extract individual tracks from audio CDs.

1. Select a drive with an audio CD
2. Click "Load Tracks" to read the TOC
3. Select a track
4. Choose output format (WAV or FLAC)
5. Click "Extract" to save the track

**Features:**
- Automatic track detection
- CD-Text reading (artist, album, track names)
- CUE sheet generation
- Jitter correction for accurate rips

### Verify Panel

Verify ripped images against the original disc.

1. Select a drive with the original disc
2. Browse for the image file to verify
3. Click "Verify" to start comparison

**Verification methods:**
- **CRC32 checksum** — fast, detects data corruption
- **SHA-256 checksum** — cryptographic, detects any changes
- **AccurateRip** — compares against database of known good rips

### ML Identify Panel

Identify content using machine learning.

1. Select an audio file (WAV format)
2. Click "Identify Content"
3. View results: title, artist, album, genre, confidence
4. Provide feedback to improve ML accuracy

**How it works:**
- Custom audio fingerprinting identifies music from actual audio
- Hybrid ML combines fingerprinting + classification + metadata
- Self-learning pipeline improves from your feedback
- All processing is local — no external API dependencies

### Settings Panel

Configure DiskRipper behavior.

**Output Settings:**
- Default output directory
- Auto-organize ripped files

**Reading Settings:**
- Read speed (1x to max)
- Retries on error (0-10)
- Buffer size (1-64 MB)

**Options:**
- Verify checksums after rip
- Eject disc after completion
- Enable audio CD extraction
- Jitter correction (audio CDs)

**Logging:**
- Log level (Trace, Debug, Info, Warn, Error)

**Theme:**
- Dark or Light appearance

---

## Using the CLI

DiskRipper includes a command-line interface for automation and scripting.

### Commands

```bash
# List all optical drives
diskripper list-drives

# Get drive information
diskripper drive-info <drive_id>

# Rip disc to ISO image
diskripper rip --drive D: --output disc.iso

# Rip with auto-organization
diskripper rip --drive D: --output disc.iso --auto-organize

# Extract files from disc
diskripper extract --drive D: --output ./extracted/

# Verify image against disc
diskripper verify --drive D: --image disc.iso

# List all jobs
diskripper jobs

# Get job status
diskripper job-status <job_id>

# Cancel a job
diskripper cancel-job <job_id>

# Batch rip multiple drives
diskripper batch-rip --drives D:,E: --output-dir ./rips/

# Output as JSON (for automation)
diskripper --json list-drives
```

### Examples

```bash
# Rip a DVD and auto-organize
diskripper rip --drive E: --output "C:\Rips\movie.iso" --auto-organize

# Extract audio CD to WAV
diskripper extract --drive D: --output "C:\Music\Album/"

# Verify a rip
diskripper verify --drive D: --image "C:\Rips\disc.iso"

# Script: rip all discs
for drive in $(diskripper --json list-drives | jq -r '.[].id'); do
    diskripper rip --drive "$drive" --output "C:\Rips\${drive}.iso"
done
```

---

## Using the MCP Server

DiskRipper includes an MCP (Model Context Protocol) server for AI agent integration.

### Setup

The MCP server is automatically registered with Hermes Agent. For other agents, add to your MCP config:

```json
{
  "mcpServers": {
    "diskripper": {
      "command": "node",
      "args": ["C:/Projects/DiskRipper/packages/mcp-server/dist/cli.js"]
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `list_drives` | List all optical drives |
| `drive_info` | Get detailed drive information |
| `rip_disc` | Rip a disc to ISO image |
| `extract_files` | Extract files from disc |
| `rip_audio_cd` | Rip audio CD to WAV |
| `verify_image` | Verify image against disc |
| `list_jobs` | List all jobs |
| `job_status` | Get job status |
| `job_history` | Get job history |

### Example Agent Usage

```
User: "Rip the disc in drive D:"
Agent: [Calls rip_disc tool]
Agent: "Disc ripped successfully. Job ID: abc-123"

User: "What's the status?"
Agent: [Calls job_status tool]
Agent: "Job abc-123 is 45% complete, running at 12 MB/s"
```

---

## ML Content Identification

### How It Works

1. **Audio Fingerprinting** — Generates compact fingerprints from audio using spectral peak analysis
2. **Hybrid Identification** — Combines fingerprinting + content classification + metadata lookup
3. **Confidence Scoring** — Each identification includes a confidence percentage
4. **Self-Learning** — User feedback improves model accuracy over time

### Supported Content Types

| Type | Identification | Organization |
|------|---------------|--------------|
| Music CD | ✅ Artist, Album, Track | `Music/Artist/Album/Track.ext` |
| Movie DVD | ✅ Title, Year | `Movies/Title (Year)/Title.ext` |
| TV DVD | ✅ Title, Season | `TV Shows/Title/Title.ext` |
| Software | ✅ Title | `Software/Title/Title.ext` |
| Game | ✅ Title | `Games/Title/Title.ext` |

### Providing Feedback

When ML identification is incorrect:

1. Open the ML Identify panel
2. View the identification result
3. Enter the correct information
4. Click "Submit Feedback"

Your feedback is used to retrain models, improving accuracy for future identifications.

---

## Settings

### Output Settings

| Setting | Default | Description |
|---------|---------|-------------|
| Default output directory | `C:\DiskRipper` | Where ripped files are saved |
| Auto-organize | Enabled | Automatically sort ripped files |

### Reading Settings

| Setting | Default | Description |
|---------|---------|-------------|
| Read speed | Maximum | Disc read speed (1x to max) |
| Retries on error | 3 | Number of retry attempts |
| Buffer size | 8 MB | Read buffer size |

### Options

| Setting | Default | Description |
|---------|---------|-------------|
| Verify checksums | Enabled | Verify after rip |
| Eject after rip | Disabled | Auto-eject on completion |
| Audio CD extraction | Enabled | Enable audio CD features |
| Jitter correction | Enabled | Correct audio CD read errors |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+R` | Refresh drives |
| `Ctrl+J` | Go to Jobs panel |
| `Ctrl+,` | Go to Settings |
| `Ctrl+Shift+F` | Open feedback dialog |
| `Escape` | Close dialog |

---

## Troubleshooting

### No Drives Detected

1. Ensure the drive is connected and powered on
2. Try refreshing drives (Ctrl+R)
3. Check Device Manager for driver issues
3. On Linux, ensure the user is in the `cdrom` group

### Rip Fails with Read Errors

1. Clean the disc surface
2. Reduce read speed in Settings
3. Increase retries in Settings
4. Enable jitter correction for audio CDs

### Verification Fails

1. The disc may be scratched or damaged
2. Try reducing read speed
3. Clean the disc and retry
4. Some copy-protected discs may not verify correctly

### ML Identification is Inaccurate

1. Provide feedback on incorrect identifications
2. More feedback = better accuracy over time
3. Short clips (< 5 seconds) may be harder to identify

### Application Won't Start

1. Ensure you have the latest version
2. Check the log file: `%APPDATA%\DiskRipper\logs\`
3. Try running from CLI to see error output

---

## FAQ

**Q: Is DiskRipper free?**
A: Yes, DiskRipper is open source under the MIT license.

**Q: Can I rip copy-protected discs?**
A: DiskRipper can read the data sectors but cannot bypass copy protection. Some discs may not rip correctly.

**Q: Where are ripped files saved?**
A: By default to `C:\DiskRipper`. You can change this in Settings.

**Q: How do I rip an audio CD to MP3?**
A: Rip to WAV first, then use a separate encoder (like LAME) to convert to MP3. FLAC output is built-in.

**Q: Can I rip a Blu-ray?**
A: Yes, DiskRipper supports Blu-ray reading. Note that commercial Blu-rays have copy protection.

**Q: How does ML identification work offline?**
A: All ML models run locally on your machine. No data is sent to external servers.

**Q: Where is my feedback stored?**
A: Feedback is stored locally at `%APPDATA%\DiskRipper\feedback\`. It is used to retrain models on your machine.

**Q: Can I use DiskRipper from a script?**
A: Yes, use the CLI (`diskripper.exe`) with the `--json` flag for machine-readable output.

---

## Support

- **GitHub Issues:** https://github.com/LoopyLuci/DiskRipper/issues
- **Documentation:** https://github.com/LoopyLuci/DiskRipper/wiki
- **Discord:** [Join our community](https://discord.gg/diskripper)

---

## License

DiskRipper is licensed under the MIT License. See [LICENSE](../LICENSE) for details.

---

*Last updated: September 2026*
