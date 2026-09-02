# DiskRipper

Next generation media backup software. Backup CDs, DVDs, Blu-ray disks with any media on them.

## Features

- **Disc Imaging** — Create ISO, BIN/CUE, IMG, DMG images of optical media
- **File Extraction** — Extract individual files and folders from discs
- **Multi-format Support** — CD, DVD, Blu-ray, HD DVD
- **Media Detection** — Automatically detects video, audio, images, programs, archives
- **Job Management** — Queue, monitor, and manage backup jobs
- **Cross-platform** — Windows, macOS, Linux

## Architecture

- `diskripper-core` — Pure Rust engine (drive detection, disc analysis, imaging, extraction)
- `diskripper-tauri` — Tauri 2 desktop app (Rust backend + React frontend)
- `frontend` — React + TypeScript UI

## Development

```bash
# Install dependencies
cd frontend && npm install

# Run dev server
npm run dev

# Build frontend
npm run build

# Run Tauri dev
cd ../diskripper-tauri && cargo tauri dev

# Build release
cargo tauri build
```

## License

MIT
