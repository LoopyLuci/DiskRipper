use std::io::{Read, Seek, SeekFrom};

fn main() {
    let raw_path = r"\\.\D:";
    println!("Trying to open {}...", raw_path);
    
    let mut file = match std::fs::OpenOptions::new()
        .read(true)
        .open(raw_path) 
    {
        Ok(f) => {
            println!("Successfully opened raw device");
            f
        }
        Err(e) => {
            println!("Failed to open: {}", e);
            return;
        }
    };
    
    // Try to seek to start
    match file.seek(SeekFrom::Start(0)) {
        Ok(pos) => println!("Seeked to position: {}", pos),
        Err(e) => println!("Seek failed: {}", e),
    }
    
    // Try to read first sector
    let mut buf = [0u8; 2048];
    match file.read(&mut buf) {
        Ok(n) => {
            println!("Read {} bytes", n);
            println!("First 32 bytes: {:?}", &buf[..32]);
        }
        Err(e) => println!("Read failed: {}", e),
    }
    
    // Try seeking to DVD capacity
    let dvd_offset = 4_700_000_000u64;
    match file.seek(SeekFrom::Start(dvd_offset)) {
        Ok(pos) => {
            println!("Seeked to DVD offset: {}", pos);
            let mut buf = [0u8; 2048];
            match file.read(&mut buf) {
                Ok(n) => println!("Read {} bytes at DVD offset", n),
                Err(e) => println!("Read at DVD offset failed: {}", e),
            }
        }
        Err(e) => println!("Seek to DVD offset failed: {}", e),
    }
    
    // Try seeking to Blu-ray capacity  
    let bd_offset = 25_000_000_000u64;
    match file.seek(SeekFrom::Start(bd_offset)) {
        Ok(pos) => {
            println!("Seeked to BD offset: {}", pos);
            let mut buf = [0u8; 2048];
            match file.read(&mut buf) {
                Ok(n) => println!("Read {} bytes at BD offset", n),
                Err(e) => println!("Read at BD offset failed: {}", e),
            }
        }
        Err(e) => println!("Seek to BD offset failed: {}", e),
    }
}
