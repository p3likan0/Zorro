use rusqlite::params;

use crate::distribution::{DistributionKey, PublishedDistribution};
use crate::packages::binary_package::{DebianBinaryControl, DebianBinaryPackage};
use crate::packages::PackageKey;
use crate::repository::Distribution;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;
//use rusqlite::Result;

use std::{io, io::Error, io::ErrorKind::Other};

pub type Pool = r2d2::Pool<SqliteConnectionManager>;

pub fn init_db_pool_connection(db_path: &str) -> io::Result<Pool> {
    let manager = SqliteConnectionManager::file(db_path);
    r2d2::Pool::new(manager).map_err(|err| {
        Error::new(
            Other,
            format!("Could not create connection manager, error: {}", err),
        )
    })
}

pub fn create_tables(db_pool: &Pool) -> io::Result<()> {
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

    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    for sql in table_creations {
        conn.execute(sql, []).map_err(|err| {
            Error::new(
                Other,
                format!("Could not insert initial tables in db, error: {}", err),
            )
        })?;
    }
    Ok(())
}

pub fn insert_package_to_distribution(
    db_pool: &Pool,
    package: &PackageKey,
    dist: &DistributionKey,
) -> io::Result<()> {
    let dist_id = get_distribution_id(&db_pool, dist)?;
    let pkg_id = get_package_id(&db_pool, package)?;
    insert_package_id_to_distribution_id(&db_pool, dist_id, pkg_id)
}

fn get_distribution_id(db_pool: &Pool, dist: &DistributionKey) -> io::Result<i64> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    conn.query_row(
        "SELECT id FROM distributions WHERE name = ? AND component = ? AND architecture = ?",
        params![dist.name, dist.component, dist.architecture],
        |row| row.get(0),
    )
    .map_err(|err| {
        Error::new(
            Other,
            format!(
                "Could not get distibution id, distribution: {:#?}, error: {}",
                dist, err
            ),
        )
    })
}

fn get_package_id(db_pool: &Pool, package: &PackageKey) -> io::Result<i64> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    conn.query_row(
        "SELECT id FROM debian_binary_package WHERE package = ? AND version = ? AND architecture = ?",
        params![package.name, package.version, package.architecture],
        |row| row.get(0),
    ).map_err(|err|{Error::new(Other, format!("Could not get package id, package: {:#?}, error: {}", package, err))})
}

// The idea here is to add the functions as private to this mod and abstract the user from doing
// the queries to get the required ids here.
fn insert_package_id_to_distribution_id(
    db_pool: &Pool,
    distribution_id: i64,
    package_id: i64,
) -> io::Result<()> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    conn.execute(
        "INSERT INTO distribution_packages (distribution_id, package_id) VALUES (?1, ?2)",
        params![distribution_id, package_id],
    )
    .map_err(|err| {
        Error::new(
            Other,
            format!(
                "Could not insert package:{} to Distribution{}:, error: {}",
                package_id, distribution_id, err
            ),
        )
    })?;
    Ok(())
}

pub fn insert_distributions(
    db_pool: &Pool,
    dists: &HashMap<String, Distribution>,
) -> io::Result<()> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
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
                ).map_err(|err|{Error::new(Other, format!("Could not insert dist in db, error: {}",err))})?;
            }
        }
    }
    Ok(())
}

pub fn get_published_distributions(db_pool: &Pool) -> io::Result<Vec<PublishedDistribution>> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    let mut stmt = conn.prepare(
        "SELECT name, origin, label, version, codename, description, component, architecture FROM distributions"
    ).map_err(|err|{Error::new(Other, format!("Could not prepare query for published distributions, error: {}",err))})?;
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
        .map_err(|err| {
            Error::new(
                Other,
                format!("Could not get published distributions, error: {}", err),
            )
        })?;
    let mut distributions = Vec::new();
    for dist in distribution_iter {
        distributions.push(dist.map_err(|err| {
            Error::new(
                Other,
                format!("Could not map published distribution, error: {}", err),
            )
        })?);
    }

    Ok(distributions)
}
pub fn insert_debian_binary_package(db_pool: &Pool, pkg: &DebianBinaryPackage) -> io::Result<()> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
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
    ).map_err(|err|{Error::new(Other, format!("Could not insert in db, error: {}",err))})?;
    Ok(())
}

pub fn get_debian_binary_package(
    db_pool: &Pool,
    package_name: &str,
    package_version: &str,
    package_arch: &str,
) -> io::Result<DebianBinaryPackage> {
    let conn = db_pool
        .get()
        .map_err(|err| Error::new(Other, format!("Could not aquire db_pool, error: {}", err)))?;
    let mut stmt = conn.prepare(
        "SELECT filename, size, md5sum, sha1, sha256, description_md5, package, source, version, section, priority, architecture, essential, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, maintainer, description, homepage, built_using
        FROM debian_binary_package
        WHERE package = ?1 AND version = ?2 AND architecture = ?3",
    ).map_err(|err|{Error::new(Other, format!("Could not prepare query, error: {}",err))})?;
    let pkg = stmt
        .query_row(
            params![package_name, package_version, package_arch],
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
        .map_err(|err| {
            Error::new(
                Other,
                format!(
                    "Could not get package with name: {}, version: {}, arch: {}, error: {}",
                    package_name, package_version, package_arch, err
                ),
            )
        })?;
    Ok(pkg)
}
