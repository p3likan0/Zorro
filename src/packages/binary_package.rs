use serde::{Deserialize, Serialize};
use debpkg::Control;
use std::{path, path::PathBuf};
use std::{io, io::{Error, ErrorKind::{Other, InvalidData}}};

use super::hash_utils::{calculate_md5, calculate_sha1, calculate_sha256};

#[derive(Debug, Serialize, Deserialize)]
pub struct DebianBinaryPackage {
    key: String,
    filename: String,
    size: u64,
    MD5sum: String,
    SHA1: String,
    SHA256: String,
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
            MD5sum: md5.to_string(),
            SHA1: sha1.to_string(),
            SHA256: sha256.to_string(),
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
    fn calculate_hashes(file_path: &path::Path) {
        match calculate_md5(&file_path) {
            Ok(hash) => println!("MD5 hash: {}", hash),
            Err(e) => println!("Error calculating MD5: {}", e),
        }
    
        match calculate_sha1(&file_path) {
            Ok(hash) => println!("SHA1 hash: {}", hash),
            Err(e) => println!("Error calculating SHA1: {}", e),
        }
    
        match calculate_sha256(&file_path) {
            Ok(hash) => println!("SHA256 hash: {}", hash),
            Err(e) => println!("Error calculating SHA256: {}", e),
        }
    }

    pub fn process(uploaded_path: &path::Path, pool_dir: &str) -> io::Result<()>{
        // Read control to validate the package
        let control = DebianBinaryPackage::read_control(&uploaded_path)?;
        // Check if the pkg already exists in the db
        let deb_path = DebianBinaryPackage::move_package_to_pool(&uploaded_path, &pool_dir)?;
        DebianBinaryPackage::calculate_hashes(&deb_path);
        let deb_path = deb_path.to_str()
            .ok_or(Error::new(Other, format!("Could not get string from deb_path: {}", deb_path.display())))?;
        let package = DebianBinaryPackage::new_from_control(&control, "md5", "sha1", "sha256", deb_path, 123);
        println!("package: {:#?}", package);
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
