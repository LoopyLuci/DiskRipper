# DiskRipper User Documentation

## Table of Contents

1. [Quick Start](#quick-start)
2. [User Guide](#user-guide)
3. [FAQ](#faq)
4. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Installation

#### Windows
1. Download `DiskRipper-0.1.0-x64.msi` from the [Releases](https://github.com/LoopyLuci/DiskRipper/releases) page
2. Run the installer and follow the prompts
3. Launch DiskRipper from the Start Menu

#### macOS
1. Download `DiskRipper-0.1.0.dmg` from the Releases page
2. Open the DMG and drag DiskRipper to Applications
3. Launch from Applications (may need to right-click → Open for first launch)

#### Linux
**AppImage:**
```bash
chmod +x DiskRipper-0.1.0.AppImage
./DiskRipper-0.1.0.AppImage
```

**Debian/Ubuntu:**
```bash
sudo dpkg -i diskripper_0.1.0_amd64.deb
sudo apt-get install -f
```

### First Launch

1. Insert an optical disc (CD, DVD, or Blu-ray)
2. Open DiskRipper
3. Click **Refresh** to detect your drive
4. Select the drive from the list
5. Choose your action:
   - **Create Image** — Save the entire disc as an ISO file
   - **Extract Files** — Copy individual files from the disc
   - **Audio CD** — Rip audio tracks to WAV files
6. Set the output path
7. Click the action button to start

---

## User Guide

### Creating a Disc Image

1. Select a drive with a disc inserted
2. Click **Create Image** in the Rip Configuration panel
3. Set the output path (e.g., `~/Documents/backup.iso`)
4. Click **Create Image** to start
5. Monitor progress in the **Jobs** tab

**Supported output formats:**
- ISO (default)
- BIN/CUE (for audio CDs)

### Extracting Files

1. Select a drive with a data disc
2. Click **Extract Files** in the Rip Configuration panel
3. Set the output directory
4. Click **Extract Files** to start
5. Files are copied preserving the directory structure

### Ripping Audio CDs

1. Select a drive with an audio CD
2. Click the **Audio CD** tab in the sidebar
3. Click **Load Tracks** to read the Table of Contents
4. Select a track (or leave empty for all tracks)
5. Set the output path
6. Click **Extract Track to WAV**

### Verifying a Rip

1. Create an image of a disc
2. Go to the **Verify** tab
3. Select the same drive
4. Enter the path to the image file
5. Click **Verify Image**
6. Review the results — green checkmarks indicate matching sectors

### Settings

Access settings from the **Settings** tab:

| Setting | Description | Default |
|---------|-------------|---------|
| Default Output Directory | Where ripped files are saved | `~/DiskRipper` |
| Read Speed | Drive read speed (None = Maximum) | Maximum |
| Verify Checksums | Verify after rip | Enabled |
| Eject After Rip | Auto-eject when done | Disabled |
| Read Retries | Number of retries on error | 3 |
| Buffer Size | Read buffer size in MB | 4 |
| Log Level | Verbosity of logging | Info |
| Enable Audio CD | Enable audio CD features | Enabled |
| Jitter Correction | Correct jitter in audio CDs | Enabled |

### Command-Line Interface

DiskRipper includes a CLI for automation:

```bash
# List drives
diskripper list-drives

# Rip a disc
diskripper rip --drive D: --output ./backup.iso --verify

# Extract files
diskripper extract --drive D: --output ./files/

# Rip audio CD
diskripper audio --drive D: --output ./music/ --track 1

# Verify image
diskripper verify --drive D: --image ./backup.iso

# Show disc info
diskripper info --drive D:

# List jobs
diskripper list-jobs

# Cancel job
diskripper cancel-job <job-id>
```

---

## FAQ

**Q: What disc types are supported?**
A: CD-ROM, CD-R, CD-RW, DVD-ROM, DVD-R, DVD-RW, DVD+R, DVD+RW, DVD-RAM, DVD+R DL, BD-ROM, BD-R, BD-RE, BD-R DL, and HD DVD.

**Q: Can I rip copy-protected discs?**
A: DiskRipper can read discs with structural protections (bad sectors, hidden tracks). CSS/AACS decryption requires system-installed libraries (libdvdcss, libaacs).

**Q: Why is my rip failing?**
A: Common causes:
- Scratched or damaged disc (increase retries in settings)
- Dirty lens (clean your drive)
- Incompatible drive (check compatibility list)
- Insufficient disk space

**Q: What is AccurateRip?**
A: AccurateRip is a database of checksums from other users' rips. If your rip matches, it's verified accurate. DiskRipper calculates AccurateRip-style checksums for verification.

**Q: How do I report a bug?**
A: Use the feedback form in the app (Help → Send Feedback) or open an issue on GitHub.

**Q: Is DiskRipper free?**
A: Yes, DiskRipper is open source under the MIT license.

---

## Troubleshooting

### Drive Not Detected
- Ensure the drive is properly connected
- Try refreshing the drive list
- Check if the drive appears in your OS's device manager
- Some USB drives require external power

### Read Errors
- Clean the disc with a soft cloth
- Try a slower read speed
- Increase the retry count in settings
- Try a different drive

### Slow Performance
- Enable DMA in your OS settings
- Close other disk-intensive applications
- Use a faster drive
- Reduce read speed if the drive is struggling

### Audio CD Issues
- Enable jitter correction in settings
- Use DAO (Disc-At-Once) mode if available
- Clean the disc and drive lens
- Try a different drive offset

### Log Files
Log files are stored at:
- **Windows:** `%APPDATA%\DiskRipper\logs\`
- **macOS:** `~/Library/Logs/DiskRipper/`
- **Linux:** `~/.local/share/DiskRipper/logs/`

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+R` | Refresh drives |
| `Ctrl+N` | New rip |
| `Ctrl+J` | Focus jobs |
| `Ctrl+,` | Open settings |
| `Ctrl+Q` | Quit |
| `F1` | Help |
| `F5` | Refresh |

---

## Supported Formats

### Input Formats (Read)
- CD: CD-DA, CD-ROM, CD-R, CD-RW
- DVD: DVD-ROM, DVD-R, DVD-RW, DVD+R, DVD+RW, DVD-RAM, DVD+R DL
- Blu-ray: BD-ROM, BD-R, BD-RE, BD-R DL
- HD DVD: HD DVD-ROM, HD DVD-R, HD DVD-RW

### Output Formats (Write)
- ISO 9660
- BIN/CUE
- WAV (audio)

---

## System Requirements

| Platform | Minimum Version | Architecture |
|----------|----------------|--------------|
| Windows | 10 (1903+) | x64 |
| macOS | 10.15 (Catalina) | x64, ARM64 |
| Linux | Ubuntu 20.04 / Fedora 34 | x64 |

**Hardware:**
- Optical drive (CD, DVD, or Blu-ray)
- 100 MB free disk space (for application)
- Additional space for ripped content
