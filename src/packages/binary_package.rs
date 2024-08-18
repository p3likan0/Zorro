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
use crate::database;

#[derive(Debug, Serialize, Deserialize)]
pub struct DebianBinaryPackage {
    pub filename: String,
    pub size: u64,
    pub md5sum: String,
    pub sha1: String,
    pub sha256: String,
    pub description_md5: Option<String>,
    pub control: DebianBinaryControl,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebianBinaryControl {
    pub package: String,
    pub source: Option<String>,
    pub version: String,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub architecture: String,
    pub essential: Option<String>,
    pub depends: Option<String>,
    pub recommends: Option<String>,
    pub suggests: Option<String>,
    pub enhances: Option<String>,
    pub pre_depends: Option<String>,
    pub breaks: Option<String>,
    pub conflicts: Option<String>,
    pub provides: Option<String>,
    pub replaces: Option<String>,
    pub installed_size: Option<String>,
    pub maintainer: String,
    pub description: String,
    pub homepage: Option<String>,
    pub built_using: Option<String>,
}

// The debian format is indenting all but the first line.
// Ex:
// Description: bla ble blu
//  bla ble blo
macro_rules! write_with_debian_format {
    ($output:expr, $format:expr, $text:expr) => {{
        use std::fmt::Write;

        let lines = $text.split('\n').enumerate();
        for (index, line) in lines {
            if index == 0 {
                writeln!($output, $format, line)?;
                continue;
            } else {
                writeln!($output, " {}", line)?;
            }
        }
        Ok(())
    }};
}
    
impl DebianBinaryPackage {
    pub fn generate_package_index(self) -> Result<String, fmt::Error> {
        let mut output = String::new();

        write_with_debian_format!(output, "Package: {}", self.control.package)?;
        if let Some(ref source) = self.control.source {
            write_with_debian_format!(output, "Source: {}", source)?;
        }
        write_with_debian_format!(output, "Version: {}", self.control.version)?;
        if let Some(ref section) = self.control.section {
            write_with_debian_format!(output, "Section: {}", section)?;
        }
        if let Some(ref priority) = self.control.priority {
            write_with_debian_format!(output, "Priority: {}", priority)?;
        }
        write_with_debian_format!(output, "Architecture: {}", self.control.architecture)?;
        if let Some(ref essential) = self.control.essential {
            write_with_debian_format!(output, "Essential: {}", essential)?;
        }
        write_with_debian_format!(output, "Filename: {}", self.filename)?;
        writeln!(output, "Size: {}", self.size)?;
        write_with_debian_format!(output, "MD5sum: {}", self.md5sum)?;
        write_with_debian_format!(output, "SHA1: {}", self.sha1)?;
        write_with_debian_format!(output, "SHA256: {}", self.sha256)?;
        if let Some(ref md5) = self.description_md5 {
            write_with_debian_format!(output, "Description-md5: {}", md5)?;
        }
        write_with_debian_format!(output, "Maintainer: {}", self.control.maintainer)?;
        write_with_debian_format!(output, "Description: {}", &self.control.description)?;
        if let Some(ref homepage) = self.control.homepage {
            write_with_debian_format!(output, "Homepage: {}", homepage)?;
        }
        // Optional fields should be handled carefully
        if let Some(ref field) = self.control.depends { write_with_debian_format!(output, "Depends: {}", field)?; }
        if let Some(ref field) = self.control.recommends { write_with_debian_format!(output, "Recommends: {}", field)?; }
        if let Some(ref field) = self.control.suggests { write_with_debian_format!(output, "Suggests: {}", field)?; }
        if let Some(ref field) = self.control.enhances { write_with_debian_format!(output, "Enhances: {}", field)?; }
        if let Some(ref field) = self.control.pre_depends { write_with_debian_format!(output, "Pre-Depends: {}", field)?; }
        if let Some(ref field) = self.control.breaks { write_with_debian_format!(output, "Breaks: {}", field)?; }
        if let Some(ref field) = self.control.conflicts { write_with_debian_format!(output, "Conflicts: {}", field)?; }
        if let Some(ref field) = self.control.provides { write_with_debian_format!(output, "Provides: {}", field)?; }
        if let Some(ref field) = self.control.replaces { write_with_debian_format!(output, "Replaces: {}", field)?; }
        if let Some(ref field) = self.control.installed_size { write_with_debian_format!(output, "Installed-Size: {}", field)?; }
        if let Some(ref field) = self.control.built_using { write_with_debian_format!(output, "Built-Using: {}", field)?; }

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
        //let key = format!("{} {} {} {}", control.name(), control.version(), arch, md5);
        //let key = format!("{} {} {}", control.name(), control.version(), arch);
        Ok(DebianBinaryPackage{
            filename: filename.to_string(),
            size: size,
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

    pub fn process(uploaded_path: &path::Path, pool_dir: &str, db_conn: &database::Pool) -> io::Result<()>{
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
        database::insert_debian_binary_package(&db_conn, &package)?;
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
