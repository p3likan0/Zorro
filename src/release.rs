use std::collections::HashMap;
use std::fs;
use std::io::{Write, Result as IoResult};

#[derive(Debug, Clone)]
pub struct DebianRelease {
    pub suite: String, 
    pub components: Vec<String>,
    pub version: String,
    pub origin: String,
    pub label: String,
    pub architectures: Vec<String>,
    pub description: String,
    pub codename: String,
    pub checksums_md5: HashMap<String, String>,
    pub checksums_sha1: HashMap<String, String>,
    pub checksums_sha256: HashMap<String, String>,
}

impl DebianRelease {
    pub fn new(suite: String, components: Vec<String>, version: String, origin: String, label: String, 
               architectures: Vec<String>, description: String, codename: String) -> Self {
        DebianRelease {
            suite,
            components,
            version,
            origin,
            label,
            architectures,
            description,
            codename,
            checksums_md5: HashMap::new(),
            checksums_sha1: HashMap::new(),
            checksums_sha256: HashMap::new(),
        }
    }
    
    pub fn add_checksum_md5(&mut self, file: String, checksum: String) {
        self.checksums_md5.insert(file, checksum);
    }

    pub fn add_checksum_sha1(&mut self, file: String, checksum: String) {
        self.checksums_sha1.insert(file, checksum);
    }

    pub fn add_checksum_sha256(&mut self, file: String, checksum: String) {
        self.checksums_sha256.insert(file, checksum);
    }

    pub fn generate_release_file_contents(&self) -> String {
        let mut contents = format!(
            "Origin: {}\nLabel: {}\nSuite: {}\nVersion: {}\nCodename: {}\nDate: {}\nArchitectures: {}\nComponents: {}\nDescription: {}\n",
            self.origin,
            self.label,
            self.suite,
            self.version,
            self.codename,
            chrono::Utc::now().to_rfc2822(),
            self.architectures.join(" "),
            self.components.join(" "),
            self.description,
        );

        contents.push_str("MD5Sum:\n");
        for (file, checksum) in &self.checksums_md5 {
            contents.push_str(&format!(" {} {}\n", checksum, file));
        }

        contents.push_str("SHA256:\n");
        for (file, checksum) in &self.checksums_sha256 {
            contents.push_str(&format!(" {} {}\n", checksum, file));
        }

        contents
    }
    pub fn save_to_file(&self, path: &str) -> IoResult<()> {
        let full_path = format!("{}/{}",path,self.suite).to_string();
        fs::create_dir_all(&full_path)?;
        let mut file = fs::File::create(format!("{}/Release",full_path))?;
        file.write_all(self.generate_release_file_contents().as_bytes())
    }
}
