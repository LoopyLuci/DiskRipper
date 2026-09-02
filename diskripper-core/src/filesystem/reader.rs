use std::io;
use std::path::Path;

#[cfg(target_os = "windows")]
pub fn read_raw_sectors<P: AsRef<Path>>(
    drive_path: P,
    start_sector: u64,
    num_sectors: u32,
    sector_size: u32,
) -> io::Result<Vec<u8>> {
    use std::process::Command;

    let path = drive_path.as_ref();
    let path_str = path.to_string_lossy();
    let drive_letter = path_str.trim_end_matches('\\').trim_end_matches(':');
    let total_bytes = (num_sectors * sector_size) as u64;
    let offset = start_sector * sector_size as u64;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "C:/Projects/DiskRipper/scripts/raw.ps1",
            "read",
            &drive_letter,
            &offset.to_string(),
            &total_bytes.to_string(),
        ])
        .output()
        .map_err(|e| io::Error::other(format!("Failed to run PowerShell: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!("Raw read failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(&stdout)
        .map_err(|e| io::Error::other(format!("Base64 decode failed: {}", e)))
}

#[cfg(target_os = "windows")]
pub fn read_raw_cdda<P: AsRef<Path>>(
    drive_path: P,
    start_sector: u64,
    num_sectors: u32,
) -> io::Result<Vec<u8>> {
    use std::process::Command;

    let path = drive_path.as_ref();
    let path_str = path.to_string_lossy();
    let drive_letter = path_str.trim_end_matches('\\').trim_end_matches(':');
    let offset = start_sector * 2352;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "C:/Projects/DiskRipper/scripts/raw.ps1",
            "readcdda",
            &drive_letter,
            &offset.to_string(),
            &num_sectors.to_string(),
        ])
        .output()
        .map_err(|e| io::Error::other(format!("Failed to run PowerShell: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!("CDDA read failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(&stdout)
        .map_err(|e| io::Error::other(format!("Base64 decode failed: {}", e)))
}

#[cfg(target_os = "linux")]
pub fn read_raw_sectors<P: AsRef<Path>>(
    drive_path: P,
    start_sector: u64,
    num_sectors: u32,
    sector_size: u32,
) -> io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_RDONLY)
        .open(drive_path)?;
    let mut file = std::io::BufReader::new(file);
    file.seek(SeekFrom::Start(start_sector * sector_size as u64))?;
    let mut buf = vec![0u8; (num_sectors * sector_size) as usize];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(target_os = "macos")]
pub fn read_raw_sectors<P: AsRef<Path>>(
    drive_path: P,
    start_sector: u64,
    num_sectors: u32,
    sector_size: u32,
) -> io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    let file = File::open(drive_path)?;
    let mut file = std::io::BufReader::new(file);
    file.seek(SeekFrom::Start(start_sector * sector_size as u64))?;
    let mut buf = vec![0u8; (num_sectors * sector_size) as usize];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(target_os = "windows")]
pub fn get_disc_size<P: AsRef<Path>>(drive_path: P) -> io::Result<u64> {
    use std::process::Command;
    let raw_path = format!(
        "\\\\.\\{}:",
        drive_path.as_ref().to_string_lossy().trim_end_matches('\\')
    );
    let letter = raw_path.trim_start_matches(r"\\.\").trim_end_matches(':');
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "C:/Projects/DiskRipper/scripts/raw.ps1",
            "size",
            letter,
        ])
        .output()
        .map_err(|e| io::Error::other(format!("PowerShell failed: {}", e)))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "Size failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|e| io::Error::other(format!("Parse failed: {}", e)))
}

#[cfg(target_os = "linux")]
pub fn get_disc_size<P: AsRef<Path>>(drive_path: P) -> io::Result<u64> {
    use std::io::Seek;
    let f = std::fs::File::open(drive_path)?;
    let mut f = std::io::BufReader::new(f);
    f.seek(std::io::SeekFrom::End(0))
}

#[cfg(target_os = "macos")]
pub fn get_disc_size<P: AsRef<Path>>(drive_path: P) -> io::Result<u64> {
    use std::io::Seek;
    let f = std::fs::File::open(drive_path)?;
    let mut f = std::io::BufReader::new(f);
    f.seek(std::io::SeekFrom::End(0))
}
