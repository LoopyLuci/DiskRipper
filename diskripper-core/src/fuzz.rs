//! Fuzz testing harness for parsers.

/// Fuzz test result
#[derive(Debug, Clone, serde::Serialize)]
pub struct FuzzResult {
    pub test_name: String,
    pub iterations: usize,
    pub failures: usize,
    pub findings: Vec<String>,
}

/// Run all fuzz tests
pub fn run_fuzz_tests(iterations: usize) -> Vec<FuzzResult> {
    let mut results = Vec::new();
    results.push(fuzz_iso9660(iterations));
    results.push(fuzz_dvd_ifo(iterations));
    results.push(fuzz_audio_cd_toc(iterations));
    results.push(fuzz_udf(iterations));
    results.push(fuzz_multisession(iterations));
    results
}

fn fuzz_iso9660(iterations: usize) -> FuzzResult {
    let mut failures = 0;
    let mut findings = Vec::new();

    for i in 0..iterations {
        let size = fastrand::usize(32768..262144);
        let mut data = vec![0u8; size];
        
        if size > 32777 {
            data[32768] = 0x01;
            data[32769] = b'C'; data[32770] = b'D'; data[32771] = b'0';
            data[32772] = b'0'; data[32773] = b'1'; data[32774] = 0x01;
        }
        
        for byte in &mut data[..32768] {
            *byte = fastrand::u8(..);
        }

        let result = std::panic::catch_unwind(|| {
            parse_iso9660_simple(&data);
        });

        if result.is_err() {
            failures += 1;
            if findings.len() < 10 {
                findings.push(format!("Iteration {}: panic", i));
            }
        }
    }

    FuzzResult { test_name: "iso9660_parser".to_string(), iterations, failures, findings }
}

fn fuzz_dvd_ifo(iterations: usize) -> FuzzResult {
    let mut failures = 0;
    let mut findings = Vec::new();

    for i in 0..iterations {
        let size = fastrand::usize(4096..65536);
        let mut data = vec![0u8; size];
        
        if size > 12 {
            data[0] = b'D'; data[1] = b'V'; data[2] = b'D'; data[3] = b'V';
            data[4] = b'I'; data[5] = b'D'; data[6] = b'E'; data[7] = b'O';
        }
        
        for byte in &mut data[8..] {
            *byte = fastrand::u8(..);
        }

        let result = std::panic::catch_unwind(|| {
            parse_ifo_simple(&data);
        });

        if result.is_err() {
            failures += 1;
            if findings.len() < 10 {
                findings.push(format!("Iteration {}: panic", i));
            }
        }
    }

    FuzzResult { test_name: "dvd_ifo_parser".to_string(), iterations, failures, findings }
}

fn fuzz_audio_cd_toc(iterations: usize) -> FuzzResult {
    let mut failures = 0;
    let mut findings = Vec::new();

    for i in 0..iterations {
        let size = fastrand::usize(4..1024);
        let data: Vec<u8> = (0..size).map(|_| fastrand::u8(..)).collect();

        let result = std::panic::catch_unwind(|| {
            parse_toc_simple(&data);
        });

        if result.is_err() {
            failures += 1;
            if findings.len() < 10 {
                findings.push(format!("Iteration {}: panic", i));
            }
        }
    }

    FuzzResult { test_name: "audio_cd_toc".to_string(), iterations, failures, findings }
}

fn fuzz_udf(iterations: usize) -> FuzzResult {
    let mut failures = 0;
    let mut findings = Vec::new();

    for i in 0..iterations {
        let size = fastrand::usize(65536..524288);
        let mut data = vec![0u8; size];
        
        if size > 20 {
            data[0] = 0x00; data[1] = b'B'; data[2] = b'E'; data[3] = b'A';
            data[4] = b'0'; data[5] = b'0'; data[6] = b'1';
        }
        
        for byte in &mut data[8..] {
            *byte = fastrand::u8(..);
        }

        let result = std::panic::catch_unwind(|| {
            parse_udf_simple(&data);
        });

        if result.is_err() {
            failures += 1;
            if findings.len() < 10 {
                findings.push(format!("Iteration {}: panic", i));
            }
        }
    }

    FuzzResult { test_name: "udf_parser".to_string(), iterations, failures, findings }
}

fn fuzz_multisession(iterations: usize) -> FuzzResult {
    let mut failures = 0;
    let mut findings = Vec::new();

    for i in 0..iterations {
        let size = fastrand::usize(1024..131072);
        let data: Vec<u8> = (0..size).map(|_| fastrand::u8(..)).collect();

        let result = std::panic::catch_unwind(|| {
            detect_sessions_simple(&data);
        });

        if result.is_err() {
            failures += 1;
            if findings.len() < 10 {
                findings.push(format!("Iteration {}: panic", i));
            }
        }
    }

    FuzzResult { test_name: "multisession".to_string(), iterations, failures, findings }
}

// === Simple parsers that should never panic ===

fn parse_iso9660_simple(data: &[u8]) {
    if data.len() < 32777 { return; }
    if data[32768] != 0x01 { return; }
    if data[32769] != b'C' || data[32770] != b'D' { return; }
    
    // Try to read volume descriptor fields
    let _sector_size = u16::from_le_bytes([data[32788], data[32789]]) as usize;
    let _num_sectors = u32::from_le_bytes([data[32800], data[32801], data[32802], data[32803]]);
    
    // Walk directory entries
    let mut offset = 32768 + 190; // Skip system area + PVD
    while offset + 33 < data.len() {
        let len = data[offset] as usize;
        if len == 0 { break; }
        if offset + len > data.len() { break; }
        offset += len;
    }
}

fn parse_ifo_simple(data: &[u8]) {
    if data.len() < 12 { return; }
    if data[0] != b'D' || data[1] != b'V' { return; }
    
    // Read VMG header
    let _num_sectors = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let _num_titles = u16::from_le_bytes([data[28], data[29]]);
    
    // Walk title table
    let mut offset = 64;
    while offset + 20 < data.len() {
        let _title_type = data[offset];
        offset += 20;
    }
}

fn parse_toc_simple(data: &[u8]) {
    if data.len() < 4 { return; }
    let _data_len = u16::from_le_bytes([data[0], data[1]]) as usize;
    let _first_track = data[2];
    let _last_track = data[3];
    
    // Walk track entries
    let num_tracks = (_last_track as usize).saturating_sub(_first_track as usize) + 1;
    let mut offset = 4;
    for _ in 0..num_tracks {
        if offset + 8 > data.len() { break; }
        let _track_num = data[offset];
        let _start_lba = u32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
        offset += 8;
    }
}

fn parse_udf_simple(data: &[u8]) {
    if data.len() < 20 { return; }
    if data[0] != 0x00 || data[1] != b'B' { return; }
    
    // Walk descriptor sequence
    let mut offset = 0;
    while offset + 16 < data.len() {
        let tag = u16::from_le_bytes([data[offset], data[offset+1]]);
        let _length = u16::from_le_bytes([data[offset+16], data[offset+17]]) as usize;
        
        match tag {
            1..=8 => { /* Valid descriptor */ }
            _ => break,
        }
        
        offset += _length;
        if _length == 0 { break; }
    }
}

fn detect_sessions_simple(data: &[u8]) {
    if data.len() < 2048 { return; }
    
    // Look for session boundaries
    let mut sessions = 0;
    let mut offset = 0;
    while offset + 2048 < data.len() {
        // Check for session start marker
        if data[offset] == 0x00 && data[offset+1] == 0xFF {
            sessions += 1;
        }
        offset += 2048;
    }
}
