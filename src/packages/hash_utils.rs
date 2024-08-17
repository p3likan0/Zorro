use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;
use sha1::Sha1;
use sha2::Sha256;
use digest::Digest;

pub fn calculate_md5(file_path: &Path) -> io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = md5::Context::new();
    let buffer_size = 10 * 10 * 1024;  // Use an 10MB buffer.
    let mut buffer = vec![0; buffer_size];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        // Update the hash with the bytes read
        hasher.consume(&buffer[..bytes_read]);
    }

    // Finalize the hash and convert it to a hexadecimal string
    let digest = hasher.compute();
    Ok(format!("{:x}", digest))
}

pub fn calculate_sha1(file_path: &Path) -> io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let buffer_size = 10 * 10 * 1024;  // Use an 10MB buffer.
    let mut buffer = vec![0; buffer_size];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        // Update the hash with the bytes read
        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize the hash and convert it to a hexadecimal string
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn calculate_sha256(file_path: &Path) -> io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let buffer_size = 8 * 1024;  // Use an 8KB buffer.
    let mut buffer = vec![0; buffer_size];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        // Update the hash with the bytes read
        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize the hash and convert it to a hexadecimal string
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
