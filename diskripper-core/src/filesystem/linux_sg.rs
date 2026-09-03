//! Linux raw disc access using SG_IO (SCSI Generic I/O).
//!
//! Provides direct SCSI command pass-through for optical drives:
//! - READ CD (0xBE) for raw sector reads
//! - READ TOC (0x43) for track listing
//! - READ CAPACITY (0x25) for disc size
//! - TEST UNIT READY (0x00) for drive status

#[cfg(target_os = "linux")]
pub mod linux {
    use std::fs::File;
    use std::io;
    use std::os::unix::io::AsRawFd;
    use std::path::Path;

    // SCSI CDB opcodes
    const SCSIOP_READ_CD: u8 = 0xBE;
    const SCSIOP_READ_TOC: u8 = 0x43;
    const SCSIOP_READ_CAPACITY: u8 = 0x25;
    const SCSIOP_TEST_UNIT_READY: u8 = 0x00;

    // SG_IO constants
    const SG_DXFER_DEV_TO_HOST: u8 = 0x01; // SG_DXFER_TO_DEV
    const SG_DXFER_HOST_TO_DEV: u8 = 0x00; // SG_DXFER_FROM_DEV
    const SG_IO_TIMEOUT: u32 = 30000; // 30 seconds

    /// SCSI pass-through structure for Linux
    #[repr(C)]
    struct SgIoHdr {
        interface_id: i32,    // 'S' for SCSI generic
        dxfer_direction: i32,
        cmd_len: u8,
        mx_sb_len: u8,
        iovec_count: u16,
        dxfer_len: u32,
        dxferp: *mut u8,
        cmdp: *mut u8,
        sbp: *mut u8,
        timeout: u32,
        flags: u32,
        pack_id: i32,
        usr_ptr: *mut u8,
        status: u8,
        masked_status: u8,
        msg_status: u8,
        sb_len_wr: u8,
        host_status: u16,
        driver_status: u16,
        resid: i32,
        duration: u32,
        info: u32,
    }

    impl SgIoHdr {
        fn new(cmd: &[u8], data: &mut [u8], direction: i32) -> Self {
            let sense_buffer = [0u8; 32];
            Self {
                interface_id: 'S' as i32,
                dxfer_direction: direction,
                cmd_len: cmd.len() as u8,
                mx_sb_len: 32,
                iovec_count: 0,
                dxfer_len: data.len() as u32,
                dxferp: data.as_mut_ptr(),
                cmdp: cmd.as_ptr() as *mut u8,
                sbp: sense_buffer.as_ptr() as *mut u8,
                timeout: SG_IO_TIMEOUT,
                flags: 0,
                pack_id: 0,
                usr_ptr: std::ptr::null_mut(),
                status: 0,
                masked_status: 0,
                msg_status: 0,
                sb_len_wr: 0,
                host_status: 0,
                driver_status: 0,
                resid: 0,
                duration: 0,
                info: 0,
            }
        }
    }

    /// Send a SCSI command via SG_IO
    fn send_scsi_command(
        fd: i32,
        cmd: &[u8],
        data: &mut [u8],
        direction: i32,
    ) -> io::Result<()> {
        let mut hdr = SgIoHdr::new(cmd, data, direction);
        
        let result = unsafe {
            libc::ioctl(fd, 0x2285, &mut hdr as *mut SgIoHdr) // SG_IO = 0x2285
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        if hdr.status != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("SCSI command failed: status={}", hdr.status),
            ));
        }

        Ok(())
    }

    /// Get disc size via READ CAPACITY
    pub fn get_disc_capacity<P: AsRef<Path>>(device_path: P) -> io::Result<(u32, u32)> {
        let file = File::open(device_path)?;
        let fd = file.as_raw_fd();

        // READ CAPACITY(10) command
        let mut cdb = [0u8; 10];
        cdb[0] = SCSIOP_READ_CAPACITY;

        let mut capacity_buffer = [0u8; 8];
        send_scsi_command(fd, &cdb, &mut capacity_buffer, SG_DXFER_DEV_TO_HOST)?;

        let last_lba = u32::from_be_bytes([
            capacity_buffer[0], capacity_buffer[1],
            capacity_buffer[2], capacity_buffer[3],
        ]);
        let block_size = u32::from_be_bytes([
            capacity_buffer[4], capacity_buffer[5],
            capacity_buffer[6], capacity_buffer[7],
        ]);

        Ok((last_lba, block_size))
    }

    /// Read CDDA sectors via READ CD (0xBE)
    pub fn read_cdda_sectors<P: AsRef<Path>>(
        device_path: P,
        start_sector: u64,
        num_sectors: u32,
    ) -> io::Result<Vec<u8>> {
        let file = File::open(device_path)?;
        let fd = file.as_raw_fd();

        let lba = start_sector as u32;
        let transfer_length = num_sectors.min(0xFFFF) as u16;

        // READ CD command (0xBE)
        let mut cdb = [u8; 12];
        cdb[0] = SCSIOP_READ_CD;
        cdb[1] = 0x00; // No special flags
        cdb[2] = ((lba >> 24) & 0xFF) as u8;
        cdb[3] = ((lba >> 16) & 0xFF) as u8;
        cdb[4] = ((lba >> 8) & 0xFF) as u8;
        cdb[5] = (lba & 0xFF) as u8;
        cdb[6] = ((transfer_length >> 8) & 0xFF) as u8;
        cdb[7] = (transfer_length & 0xFF) as u8;
        cdb[8] = 0x10; // 2048 bytes per sector (user data)
        cdb[9] = 0x00; // No sub-channel data

        let buffer_size = (num_sectors as usize) * 2048;
        let mut buffer = vec![0u8; buffer_size];
        send_scsi_command(fd, &cdb, &mut buffer, SG_DXFER_DEV_TO_HOST)?;

        Ok(buffer)
    }

    /// Read TOC via READ TOC (0x43)
    pub fn read_toc<P: AsRef<Path>>(device_path: P) -> io::Result<TocData> {
        let file = File::open(device_path)?;
        let fd = file.as_raw_fd();

        // READ TOC command (MSF format)
        let mut cdb = [u8; 10];
        cdb[0] = SCSIOP_READ_TOC;
        cdb[1] = 0x02; // MSF format
        cdb[2] = 0x00; // First track
        cdb[7] = 0xFF; // Allocation length (will be set by drive)
        cdb[8] = 0x00;

        let mut toc_buffer = [0u8; 4 + 100 * 8]; // Header + up to 100 track descriptors
        send_scsi_command(fd, &cdb, &mut toc_buffer, SG_DXFER_DEV_TO_HOST)?;

        let toc_data_len = u16::from_be_bytes([toc_buffer[0], toc_buffer[1]]) as usize;
        let first_track = toc_buffer[2];
        let last_track = toc_buffer[3];

        let mut tracks = Vec::new();
        for i in 0..toc_data_len.saturating_sub(2) / 8 {
            let offset = 4 + i * 8;
            if offset + 8 <= toc_buffer.len() {
                let track_number = toc_buffer[offset + 1];
                let control = toc_buffer[offset + 4];
                let lba = u32::from_be_bytes([
                    toc_buffer[offset + 5],
                    toc_buffer[offset + 6],
                    toc_buffer[offset + 7],
                    toc_buffer[offset + 8],
                ]);
                tracks.push(TocTrack {
                    track_number,
                    control,
                    start_lba: lba,
                });
            }
        }

        Ok(TocData {
            first_track,
            last_track,
            tracks,
        })
    }

    /// Check if drive is ready via TEST UNIT READY
    pub fn test_unit_ready<P: AsRef<Path>>(device_path: P) -> io::Result<bool> {
        let file = File::open(device_path)?;
        let fd = file.as_raw_fd();

        let mut cdb = [0u8; 6];
        cdb[0] = SCSIOP_TEST_UNIT_READY;

        let mut buffer = [0u8; 0];
        match send_scsi_command(fd, &cdb, &mut buffer, SG_DXFER_HOST_TO_DEV) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::*;

/// TOC track entry
#[derive(Debug, Clone)]
pub struct TocTrack {
    pub track_number: u8,
    pub control: u8,
    pub start_lba: u32,
}

/// TOC data
#[derive(Debug, Clone)]
pub struct TocData {
    pub first_track: u8,
    pub last_track: u8,
    pub tracks: Vec<TocTrack>,
}

/// Check if SG_IO is available on this system
#[cfg(target_os = "linux")]
pub fn is_sg_io_available() -> bool {
    std::path::Path::new("/dev/sg0").exists() || 
    std::path::Path::new("/dev/sr0").exists()
}

#[cfg(not(target_os = "linux"))]
pub fn is_sg_io_available() -> bool {
    false
}
