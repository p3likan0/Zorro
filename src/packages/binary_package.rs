use serde::{Deserialize, Serialize};
use debpkg::Control;
use std::{path, path::PathBuf};
use std::{io, io::{Error, ErrorKind::{Other, InvalidData}}};
use sha2::{Sha256};
use sha1::Sha1;
use md5::Md5;
use std::{fmt::Write, fmt}; // For using the write! macro with Strings   
use std::os::linux::fs::MetadataExt;

use super::hash_utils::calculate_hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct DebianBinaryPackage {
    key: String,
    filename: String,
    size: u64,
    md5sum: String,
    sha1: String,
    sha256: String,
    description_md5: Option<String>,
    control: DebianBinaryControl,
}

#[derive(Debug, Serialize, Deserialize)]
struct DebianBinaryControl {
    package: String,
    source: Option<String>,
    version: String,
    section: Option<String>,
    priority: Option<String>,
    architecture: String,
    essential: Option<String>,
    depends: Option<String>,
    recommends: Option<String>,
    suggests: Option<String>,
    enhances: Option<String>,
    pre_depends: Option<String>,
    breaks: Option<String>,
    conflicts: Option<String>,
    provides: Option<String>,
    replaces: Option<String>,
    installed_size: Option<String>,
    maintainer: String,
    description: String,
    homepage: Option<String>,
    built_using: Option<String>,
}

impl DebianBinaryPackage {
    /// Formats the description for Debian control files with proper indentations for continuation lines.
    fn apply_debian_format(description: &str) -> Result<String, fmt::Error> {
        let mut output = String::new();
        let lines = description.split('\n').enumerate();
        for (index, line) in lines {
            if index == 0 {
                writeln!(output, "{}", line)?;
                continue;
            }
            writeln!(output, " {}", line)?;
        }
        Ok(output)
    }

    pub fn generate_package_index(self) -> Result<String, fmt::Error> {
        let mut output = String::new();

        writeln!(output, "Package: {}", self.control.package)?;
        if let Some(ref source) = self.control.source {
            writeln!(output, "Source: {}", source)?;
        }
        writeln!(output, "Version: {}", self.control.version)?;
        if let Some(ref section) = self.control.section {
            writeln!(output, "Section: {}", section)?;
        }
        if let Some(ref priority) = self.control.priority {
            writeln!(output, "Priority: {}", priority)?;
        }
        writeln!(output, "Architecture: {}", self.control.architecture)?;
        if let Some(ref essential) = self.control.essential {
            writeln!(output, "Essential: {}", essential)?;
        }
        writeln!(output, "Filename: {}", self.filename)?;
        writeln!(output, "Size: {}", self.size)?;
        writeln!(output, "MD5sum: {}", self.md5sum)?;
        writeln!(output, "SHA1: {}", self.sha1)?;
        writeln!(output, "SHA256: {}", self.sha256)?;
        if let Some(ref md5) = self.description_md5 {
            writeln!(output, "Description-md5: {}", md5)?;
        }
        writeln!(output, "Maintainer: {}", self.control.maintainer)?;
        write!(output, "Description: {}\n", DebianBinaryPackage::apply_debian_format(&self.control.description)?)?;
        if let Some(ref homepage) = self.control.homepage {
            writeln!(output, "Homepage: {}", homepage)?;
        }
        // Optional fields should be handled carefully
        if let Some(ref field) = self.control.depends { writeln!(output, "Depends: {}", field)?; }
        if let Some(ref field) = self.control.recommends { writeln!(output, "Recommends: {}", field)?; }
        if let Some(ref field) = self.control.suggests { writeln!(output, "Suggests: {}", field)?; }
        if let Some(ref field) = self.control.enhances { writeln!(output, "Enhances: {}", field)?; }
        if let Some(ref field) = self.control.pre_depends { writeln!(output, "Pre-Depends: {}", field)?; }
        if let Some(ref field) = self.control.breaks { writeln!(output, "Breaks: {}", field)?; }
        if let Some(ref field) = self.control.conflicts { writeln!(output, "Conflicts: {}", field)?; }
        if let Some(ref field) = self.control.provides { writeln!(output, "Provides: {}", field)?; }
        if let Some(ref field) = self.control.replaces { writeln!(output, "Replaces: {}", field)?; }
        if let Some(ref field) = self.control.installed_size { writeln!(output, "Installed-Size: {}", field)?; }
        if let Some(ref field) = self.control.built_using { writeln!(output, "Built-Using: {}", field)?; }

        // Separate packages with a blank line
        writeln!(output)?;

        Ok(output)
    }

    fn read_control(deb_path: &path::Path) -> io::Result<debpkg::Control> {
        // Open the Debian package file
        let deb_file = std::fs::File::open(deb_path)?;

        // Parse the Debian package
        let mut pkg = debpkg::DebPkg::parse(deb_file)
            .map_err(|err| {
                Error::new(InvalidData ,format!("Failed to parse package {}, error: {}", deb_path.display(), err))
            })?;

        // Extract and parse the control file
        let control_tar = pkg.control()
            .map_err(|err| {
                Error::new(InvalidData ,format!("Control for package {} not valid, error: {}", deb_path.display(), err))
            })?;

        let control = Control::extract(control_tar)
            .map_err(|err| {
                Error::new(InvalidData ,format!("Cannot extract control data from package {}, error {}", deb_path.display(), err))
            })?;
        Ok(control)
    }

    fn new_from_control(control: &debpkg::Control, md5: &str, sha1: &str, sha256: &str, filename: &str, size: u64) -> io::Result<DebianBinaryPackage> { 
        let arch = match control.get("Architecture") {
            Some(arch) => arch,
            None => return Err(Error::new(InvalidData, format!("Cannot get the architecture from the package: {} {}", control.name(), control.version())))
        };

        let maintainer = match control.get("Maintainer") {
            Some(maintainer) => maintainer,
            None => return Err(Error::new(InvalidData, format!("Could not find Maintainer for package: {}", filename)))
        };
        let description = match control.long_description() {
            Some(description) => description,
            None => return Err(Error::new(InvalidData, format!("Could not find Description for package: {}", filename)))
        };
        let key = format!("{} {} {} {}", control.name(), control.version(), arch, md5);
        Ok(DebianBinaryPackage{
            filename: filename.to_string(),
            size: size,
            key: key, 
            md5sum: md5.to_string(),
            sha1: sha1.to_string(),
            sha256: sha256.to_string(),
            description_md5: None,
            control: DebianBinaryControl {
                package: control.name().to_string(),
                source: control.get("Source").map(|s| s.to_string()),
                version: control.version().to_string(),
                section: control.get("Section").map(|s| s.to_string()),
                priority: control.get("Priority").map(|s| s.to_string()),
                architecture: arch.to_string(),
                essential: control.get("Essential").map(|s| s.to_string()),
                depends: control.get("Depends").map(|s| s.to_string()),
                recommends: control.get("Recommends").map(|s| s.to_string()),
                suggests: control.get("Suggests").map(|s| s.to_string()),
                enhances: control.get("Enhances").map(|s| s.to_string()),
                pre_depends: control.get("Pre-Depends").map(|s| s.to_string()),
                breaks: control.get("Breaks").map(|s| s.to_string()),
                conflicts: control.get("Conflicts").map(|s| s.to_string()),
                provides: control.get("Provides").map(|s| s.to_string()),
                replaces: control.get("Replaces").map(|s| s.to_string()),
                installed_size: control.get("Installed-Size").map(|s| s.to_string()),
                maintainer: maintainer.to_string(),
                description: description.to_string(),
                homepage: control.get("Homepage").map(|s| s.to_string()),
                built_using: control.get("Built-Using").map(|s| s.to_string()),
            }
        })
    }
    fn calculate_hashes(file_path: &path::Path) -> io::Result<(String, String, String)> {
        let md5 = match calculate_hash::<Md5>(&file_path) {
            Ok(hash) => hash,
            Err(err) => return Err(Error::new(Other, format!("Error calculating md5: {}", err))),
        };
        let sha1 = match calculate_hash::<Sha1>(&file_path) {
            Ok(hash) => hash,
            Err(err) => return Err(Error::new(Other, format!("Error calculating sha1: {}", err))),
        };
    
        let sha256 = match calculate_hash::<Sha256>(&file_path) {
            Ok(hash) => hash,
            Err(err) => return Err(Error::new(Other, format!("Error calculating sha256: {}", err))),
        };
        Ok((md5, sha1, sha256))
    }

    pub fn process(uploaded_path: &path::Path, pool_dir: &str) -> io::Result<()>{
        // Read control to validate the package
        let control = DebianBinaryPackage::read_control(&uploaded_path)?;
        // Check if the pkg already exists in the db
        let deb_path = DebianBinaryPackage::move_package_to_pool(&uploaded_path, &pool_dir)?;
        let (md5, sha1, sha256) = DebianBinaryPackage::calculate_hashes(&deb_path)?;
        let file_metadata = std::fs::metadata(&deb_path)?;
        let package_size = file_metadata.st_size(); 
        let deb_path = deb_path.to_str()
            .ok_or(Error::new(Other, format!("Could not get string from deb_path: {}", deb_path.display())))?;
        let package = DebianBinaryPackage::new_from_control(&control, &md5, &sha1, &sha256, deb_path, package_size)?;
        let package_index = package.generate_package_index()
            .map_err(|err|{
                Error::new(InvalidData, format!("Could not generate package index package: {}, error {}", deb_path, err))
            })?;
        println!("package: {:#}", package_index);
        Ok(())
    }

    // We move the package using rename, which brings the limitation of the file needing to be in the
    // same filesystem but is extremely fast.
    fn move_package_to_pool(deb_path: &path::Path, pool_dir: &str) -> io::Result<PathBuf> {
        let dest_dir: PathBuf;
        let file_name_str = deb_path.file_name().expect("Could not decode package name")
            .to_str().expect("Could not decode package name to string");
        // Since there are going to be a lot of libx_1_2_3_arch.deb packages, we crate a subdirectory
        // for each one. Ex /lib/a/liba_1_2_3_amd64.deb, /lib/b/libb_1_2_3_amd64.deb
        if file_name_str.starts_with("lib"){
            let lib_fourth_char = file_name_str.chars().nth(3).expect("Library package does not contain a valid name");
            dest_dir = path::Path::new(&pool_dir).join("lib").join(lib_fourth_char.to_string());
        } else {
            let pkg_first_char = file_name_str.chars().nth(0).expect("DebianBinaryPackage does not contain a valid name");
            dest_dir = path::Path::new(&pool_dir).join(pkg_first_char.to_string());
        }
        std::fs::create_dir_all(&dest_dir).map_err(|err| {
            Error::new(Other, format!("Error creating dir: {}, error: {}", dest_dir.display(), err))
        })?;
        let new_deb_path = dest_dir.join(file_name_str);
        std::fs::rename(deb_path, &new_deb_path).map_err(|err| {
            Error::new(Other, format!("Error renaming deb from: {}, to: {}, error: {}", deb_path.display(), dest_dir.display(), err))
        })?;
        Ok(new_deb_path)
    }
}
