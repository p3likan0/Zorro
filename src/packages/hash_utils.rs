use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;
use digest::Digest;

pub fn calculate_hash<D: Digest>(file_path: &Path) -> io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = D::new();
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
    Ok(hex::encode(result))
}
