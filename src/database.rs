use rusqlite::{params, Connection, Result};

use crate::packages::binary_package::{DebianBinaryPackage, DebianBinaryControl}; 

use r2d2_sqlite::SqliteConnectionManager;
//use rusqlite::Result;

use std::{io, io::Error, io::ErrorKind::{Other}};

pub type Pool = r2d2::Pool<SqliteConnectionManager>;

pub fn init_db_pool_connection(db_path: &str) -> io::Result<Pool> {
    let manager = SqliteConnectionManager::file(db_path);
    r2d2::Pool::new(manager).map_err(|err|{Error::new(Other,format!("Could not create connection manager, error: {}", err))})
}

pub fn create_debian_binary_package_table(db_pool: &Pool) -> io::Result<()> {
    let conn = db_pool.get().map_err(|err|{Error::new(Other, format!("Could not aquire db_pool, error: {}",err))})?; 
    conn.execute(
        "CREATE TABLE IF NOT EXISTS debian_binary_package (
            key TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            size INTEGER NOT NULL,
            md5sum TEXT NOT NULL,
            sha1 TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            description_md5 TEXT,
            package TEXT NOT NULL,
            source TEXT,
            version TEXT NOT NULL,
            section TEXT,
            priority TEXT,
            architecture TEXT NOT NULL,
            essential TEXT,
            depends TEXT,
            recommends TEXT,
            suggests TEXT,
            enhances TEXT,
            pre_depends TEXT,
            breaks TEXT,
            conflicts TEXT,
            provides TEXT,
            replaces TEXT,
            installed_size TEXT,
            maintainer TEXT NOT NULL,
            description TEXT NOT NULL,
            homepage TEXT,
            built_using TEXT
        ) WITHOUT ROWID", [],
    ).map_err(|err|{Error::new(Other, format!("Could not insert in db, error: {}",err))})?; 
    Ok(())
}

pub fn insert_debian_binary_package(db_pool: &Pool, pkg: &DebianBinaryPackage) -> io::Result<()> {
    let conn = db_pool.get().map_err(|err|{Error::new(Other, format!("Could not aquire db_pool, error: {}",err))})?; 
    conn.execute(
        "INSERT INTO debian_binary_package (key, filename, size, md5sum, sha1, sha256, description_md5, package, source, version, section, priority, architecture, essential, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, maintainer, description, homepage, built_using)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
        params![
            pkg.key, pkg.filename, pkg.size, pkg.md5sum, pkg.sha1, pkg.sha256, pkg.description_md5,
            pkg.control.package, pkg.control.source, pkg.control.version, pkg.control.section,
            pkg.control.priority, pkg.control.architecture, pkg.control.essential, pkg.control.depends,
            pkg.control.recommends, pkg.control.suggests, pkg.control.enhances, pkg.control.pre_depends,
            pkg.control.breaks, pkg.control.conflicts, pkg.control.provides, pkg.control.replaces,
            pkg.control.installed_size, pkg.control.maintainer, pkg.control.description,
            pkg.control.homepage, pkg.control.built_using
        ],
    ).map_err(|err|{Error::new(Other, format!("Could not insert in db, error: {}",err))})?; 
    Ok(())
}

pub fn get_debian_binary_package(conn: &Connection, key: &str) -> Result<DebianBinaryPackage> {
    let mut stmt = conn.prepare(
        "SELECT key, filename, size, md5sum, sha1, sha256, description_md5, package, source, version, section, priority, architecture, essential, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, maintainer, description, homepage, built_using
        FROM debian_binary_package
        WHERE key = ?1"
    )?;
    let pkg = stmt.query_row(params![key], |row| {
        Ok(DebianBinaryPackage {
            key: row.get(0)?,
            filename: row.get(1)?,
            size: row.get(2)?,
            md5sum: row.get(3)?,
            sha1: row.get(4)?,
            sha256: row.get(5)?,
            description_md5: row.get(6)?,
            control: DebianBinaryControl {
                package: row.get(7)?,
                source: row.get(8)?,
                version: row.get(9)?,
                section: row.get(10)?,
                priority: row.get(11)?,
                architecture: row.get(12)?,
                essential: row.get(13)?,
                depends: row.get(14)?,
                recommends: row.get(15)?,
                suggests: row.get(16)?,
                enhances: row.get(17)?,
                pre_depends: row.get(18)?,
                breaks: row.get(19)?,
                conflicts: row.get(20)?,
                provides: row.get(21)?,
                replaces: row.get(22)?,
                installed_size: row.get(23)?,
                maintainer: row.get(24)?,
                description: row.get(25)?,
                homepage: row.get(26)?,
                built_using: row.get(27)?
            }
        })
    })?;
    Ok(pkg)
}

