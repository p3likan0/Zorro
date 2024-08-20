use std::collections::HashMap;
use std::fs;
use std::io::{Result as IoResult, Write};

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

// Date time trait created for mocking during tests
trait DateTimeProvider {
    fn current_time_rfc2822(&self) -> String;
}

// Date time production implementation
struct RealDateTimeProvider;
impl DateTimeProvider for RealDateTimeProvider {
    fn current_time_rfc2822(&self) -> String {
        chrono::Utc::now().to_rfc2822()
    }
}

impl DebianRelease {
    pub fn new(
        suite: String,
        components: Vec<String>,
        version: String,
        origin: String,
        label: String,
        architectures: Vec<String>,
        description: String,
        codename: String,
    ) -> Self {
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

    fn generate_release_file_contents<T: DateTimeProvider>(&self, time_provider: &T) -> String {
        let mut contents = format!(
            "Origin: {}\nLabel: {}\nSuite: {}\nVersion: {}\nCodename: {}\nDate: {}\nArchitectures: {}\nComponents: {}\nDescription: {}\n",
            self.origin,
            self.label,
            self.suite,
            self.version,
            self.codename,
            time_provider.current_time_rfc2822(),
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
        let time_provider = RealDateTimeProvider;
        self.save_release_to_file(path, &time_provider)
    }

    fn save_release_to_file<T: DateTimeProvider>(
        &self,
        path: &str,
        time_provider: &T,
    ) -> IoResult<()> {
        let full_path = format!("{}/{}", path, self.suite).to_string();
        fs::create_dir_all(&full_path)?;
        let mut file = fs::File::create(format!("{}/Release", full_path))?;
        file.write_all(
            self.generate_release_file_contents(time_provider)
                .as_bytes(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    // Date time testing implementation
    struct MockDateTimeProvider;
    impl DateTimeProvider for MockDateTimeProvider {
        fn current_time_rfc2822(&self) -> String {
            String::from("Thu, 01 Jan 1970 00:00:00 +0000")
        }
    }

    #[test]
    fn create_release_file() {
        let release = DebianRelease::new(
            "experimental".to_string(),
            vec!["main".to_string(), "contrib".to_string(), "ble".to_string()],
            "1.2".to_string(),
            "YourCoolCompany".to_string(),
            "YourLabel".to_string(),
            vec!["arm64".to_string(), "riscv".to_string()],
            "This is a very cool repository".to_string(),
            "buster".to_string(),
        );
        let tmp_dir = TempDir::new("example").expect("cannot create tempdir");
        let time_provider = MockDateTimeProvider;
        assert!(release
            .save_release_to_file(
                tmp_dir.path().to_str().expect("Failed to convert to &str"),
                &time_provider
            )
            .is_ok());
        let expected_release_path = tmp_dir.path().join("experimental/Release");
        assert!(expected_release_path.exists());

        let file_contents =
            fs::read_to_string(&expected_release_path).expect("Could not read file");
        let expected_contents = r#"Origin: YourCoolCompany
Label: YourLabel
Suite: experimental
Version: 1.2
Codename: buster
Date: Thu, 01 Jan 1970 00:00:00 +0000
Architectures: arm64 riscv
Components: main contrib ble
Description: This is a very cool repository
MD5Sum:
SHA256:
"#;
        assert_eq!(expected_contents, file_contents);
    }
}
