use rusqlite::params;

use crate::distribution::{DistributionKey, PublishedDistribution};
use crate::packages::binary_package::{DebianBinaryControl, DebianBinaryPackage};
use crate::packages::PackageKey;
use crate::repository::Distribution;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;

pub type Pool = r2d2::Pool<SqliteConnectionManager>;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("Could not create connection manager, r2d2 error:{0}")]
    CouldNotCreateConnectionManager(r2d2::Error),

    #[error("Could not aquire pool lock, r2d2 error:{0}")]
    CouldNotAquirePoolLock(r2d2::Error),

    #[error("Could not perform execute operation, rusqlite error: {0}")]
    CouldNotExecute(rusqlite::Error),

    #[error("Error while running query to get distribution:{0}, rusqlite error: {1}")]
    QueryGetDistributionID(DistributionKey, rusqlite::Error),

    #[error("Error while running query to get package:{0}, rusqlite error: {1}")]
    QueryGetPackageID(PackageKey, rusqlite::Error),

    #[error(
        "Error while running query to get package_id:{0}, distribution_id:{1} rusqlite error: {2}"
    )]
    InsertPackageIDToDistributionID(i64, i64, rusqlite::Error),

    #[error("Could not prepare query to get published distributions, rusqlite error: {0}")]
    CouldNotPrepareQueryGetPublishedDistributions(rusqlite::Error),

    #[error("Could not to get published distributions, rusqlite error: {0}")]
    CouldNotGetPublishedDistributions(rusqlite::Error),

    #[error("Could not to map published distribution, rusqlite error: {0}")]
    CouldNotMapPublishedDistribution(rusqlite::Error),

    #[error("Could not prepare query to get debian binary package:{0}, rusqlite error: {1}")]
    CouldNotPrepareQueryGetDebianBinaryPackage(PackageKey, rusqlite::Error),

    #[error("Could not to get debian binary package: {0}, rusqlite error: {1}")]
    CouldNotGetDebianBinaryPackage(PackageKey, rusqlite::Error),

    #[error("Could not to insert distribution: {0},{1}, rusqlite error: {2}")]
    CouldNotInsertDistribution(String, Distribution, rusqlite::Error),

    #[error("Could not to insert debian binary package: {0},rusqlite error: {1}")]
    CouldNotInsertDebianBinaryPackage(DebianBinaryPackage, rusqlite::Error),
}

use DatabaseError::*;

pub fn init_db_pool_connection(db_path: &str) -> Result<Pool, DatabaseError> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = r2d2::Pool::new(manager).map_err(CouldNotCreateConnectionManager)?;
    Ok(pool)
}

pub fn create_tables(db_pool: &Pool) -> Result<(), DatabaseError> {
    let table_creations = vec![
        "CREATE TABLE IF NOT EXISTS distribution_packages (
            distribution_id INTEGER NOT NULL,
            package_id INTEGER NOT NULL,
            PRIMARY KEY (distribution_id, package_id),
            FOREIGN KEY (distribution_id) REFERENCES distributions(id) ON DELETE CASCADE,
            FOREIGN KEY (package_id) REFERENCES debian_binary_package(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS distributions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            origin TEXT NOT NULL,
            label TEXT NOT NULL,
            version TEXT NOT NULL,
            codename TEXT NOT NULL,
            description TEXT NOT NULL,
            component TEXT NOT NULL,
            architecture TEXT NOT NULL,
            UNIQUE(name, component, architecture)
        )",
        "CREATE TABLE IF NOT EXISTS debian_binary_package (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            built_using TEXT,
            UNIQUE (package, version, architecture)
        )",
    ];

    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    for sql in table_creations {
        conn.execute(sql, []).map_err(CouldNotExecute)?;
    }
    Ok(())
}

pub fn insert_package_to_distribution(
    db_pool: &Pool,
    package: &PackageKey,
    dist: &DistributionKey,
) -> Result<(), DatabaseError> {
    let dist_id = get_distribution_id(&db_pool, dist)?;
    let pkg_id = get_package_id(&db_pool, package)?;
    insert_package_id_to_distribution_id(&db_pool, dist_id, pkg_id)
}

fn get_distribution_id(db_pool: &Pool, dist: &DistributionKey) -> Result<i64, DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    conn.query_row(
        "SELECT id FROM distributions WHERE name = ? AND component = ? AND architecture = ?",
        params![dist.name, dist.component, dist.architecture],
        |row| row.get(0),
    )
    .map_err(|err| QueryGetDistributionID(dist.clone(), err))
}

fn get_package_id(db_pool: &Pool, package: &PackageKey) -> Result<i64, DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    conn.query_row(
        "SELECT id FROM debian_binary_package WHERE package = ? AND version = ? AND architecture = ?",
        params![package.name, package.version, package.architecture],
        |row| row.get(0),
    ).map_err(|err|{QueryGetPackageID(package.clone(), err)})
}

// The idea here is to add the functions as private to this mod and abstract the user from doing
// the queries to get the required ids here.
fn insert_package_id_to_distribution_id(
    db_pool: &Pool,
    distribution_id: i64,
    package_id: i64,
) -> Result<(), DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    conn.execute(
        "INSERT INTO distribution_packages (distribution_id, package_id) VALUES (?1, ?2)",
        params![distribution_id, package_id],
    )
    .map_err(|err| InsertPackageIDToDistributionID(distribution_id, package_id, err))?;
    Ok(())
}

pub fn insert_distributions(
    db_pool: &Pool,
    dists: &HashMap<String, Distribution>,
) -> Result<(), DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    for (name, dist) in dists {
        for component in &dist.components {
            for architecture in &dist.architectures {
                conn.execute(
                    "INSERT OR REPLACE INTO distributions (name, origin, label, version, codename, description, component, architecture)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        name,
                        dist.origin,
                        dist.label,
                        dist.version,
                        dist.codename,
                        dist.description,
                        component,
                        architecture,
                    ],
                ).map_err(|err|{CouldNotInsertDistribution(name.clone(), dist.clone(), err)})?;
            }
        }
    }
    Ok(())
}

pub fn get_published_distributions(
    db_pool: &Pool,
) -> Result<Vec<PublishedDistribution>, DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    let mut stmt = conn.prepare(
        "SELECT name, origin, label, version, codename, description, component, architecture FROM distributions"
    ).map_err(CouldNotPrepareQueryGetPublishedDistributions)?;
    let distribution_iter = stmt
        .query_map([], |row| {
            Ok(PublishedDistribution {
                name: row.get(0)?,
                origin: row.get(1)?,
                label: row.get(2)?,
                version: row.get(3)?,
                codename: row.get(4)?,
                description: row.get(5)?,
                component: row.get(6)?,
                architecture: row.get(7)?,
            })
        })
        .map_err(CouldNotGetPublishedDistributions)?;
    let mut distributions = Vec::new();
    for dist in distribution_iter {
        distributions.push(dist.map_err(|err| CouldNotMapPublishedDistribution(err))?);
    }

    Ok(distributions)
}
pub fn insert_debian_binary_package(
    db_pool: &Pool,
    pkg: &DebianBinaryPackage,
) -> Result<(), DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    conn.execute(
        "INSERT INTO debian_binary_package (filename, size, md5sum, sha1, sha256, description_md5, package, source, version, section, priority, architecture, essential, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, maintainer, description, homepage, built_using)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
        params![
            pkg.filename, pkg.size, pkg.md5sum, pkg.sha1, pkg.sha256, pkg.description_md5,
            pkg.control.package, pkg.control.source, pkg.control.version, pkg.control.section,
            pkg.control.priority, pkg.control.architecture, pkg.control.essential, pkg.control.depends,
            pkg.control.recommends, pkg.control.suggests, pkg.control.enhances, pkg.control.pre_depends,
            pkg.control.breaks, pkg.control.conflicts, pkg.control.provides, pkg.control.replaces,
            pkg.control.installed_size, pkg.control.maintainer, pkg.control.description,
            pkg.control.homepage, pkg.control.built_using
        ],
    ).map_err(|err|{CouldNotInsertDebianBinaryPackage(pkg.clone(), err)})?;
    Ok(())
}

pub fn get_debian_binary_package(
    db_pool: &Pool,
    package: &PackageKey,
) -> Result<DebianBinaryPackage, DatabaseError> {
    let conn = db_pool.get().map_err(CouldNotAquirePoolLock)?;
    let mut stmt = conn.prepare(
        "SELECT filename, size, md5sum, sha1, sha256, description_md5, package, source, version, section, priority, architecture, essential, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, maintainer, description, homepage, built_using
        FROM debian_binary_package
        WHERE package = ?1 AND version = ?2 AND architecture = ?3",
    ).map_err(|err|{CouldNotPrepareQueryGetDebianBinaryPackage(package.clone(), err)})?;
    let pkg = stmt
        .query_row(
            params![package.name, package.version, package.architecture],
            |row| {
                Ok(DebianBinaryPackage {
                    filename: row.get(0)?,
                    size: row.get(1)?,
                    md5sum: row.get(2)?,
                    sha1: row.get(3)?,
                    sha256: row.get(4)?,
                    description_md5: row.get(5)?,
                    control: DebianBinaryControl {
                        package: row.get(6)?,
                        source: row.get(7)?,
                        version: row.get(8)?,
                        section: row.get(9)?,
                        priority: row.get(10)?,
                        architecture: row.get(11)?,
                        essential: row.get(12)?,
                        depends: row.get(13)?,
                        recommends: row.get(14)?,
                        suggests: row.get(15)?,
                        enhances: row.get(16)?,
                        pre_depends: row.get(17)?,
                        breaks: row.get(18)?,
                        conflicts: row.get(19)?,
                        provides: row.get(20)?,
                        replaces: row.get(21)?,
                        installed_size: row.get(22)?,
                        maintainer: row.get(23)?,
                        description: row.get(24)?,
                        homepage: row.get(25)?,
                        built_using: row.get(26)?,
                    },
                })
            },
        )
        .map_err(|err| CouldNotGetDebianBinaryPackage(package.clone(), err))?;
    Ok(pkg)
}
