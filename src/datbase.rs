use rusqlite::{Connection, Result};
use rusqlite::NO_PARAMS;

use rusqlite::{params, Connection, Result};

fn main() -> Result<()> {
    let conn = Connection::open("debian_packages.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS packages (
            id TEXT PRIMARY KEY,
            package_name TEXT NOT NULL,
            version TEXT NOT NULL,
            architecture TEXT NOT NULL,
            maintainer TEXT,
            description TEXT,
            depends TEXT,
            recommends TEXT,
            suggests TEXT,
            enhances TEXT,
            pre_depends TEXT,
            breaks TEXT,
            conflicts TEXT,
            provides TEXT,
            replaces TEXT,
            installed_size INTEGER,
            homepage TEXT,
            source TEXT,
            section TEXT,
            priority TEXT,
            multi_arch TEXT,
            essential TEXT
        ) WITHOUT ROWID", [],
    )?;

    conn.execute(
        "INSERT INTO packages (package_name, version, architecture, maintainer, description, depends, recommends, suggests, enhances, pre_depends, breaks, conflicts, provides, replaces, installed_size, homepage, source, section, priority, multi_arch, essential) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            "nano",
            "2.9.3",
            "amd64",
            "Example Maintainer",
            "Nano text editor",
            "libc6 (>= 2.15), libncursesw5 (>= 6)",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            2048,
            "https://example.com",
            "nano",
            "editors",
            "optional",
            "no",
            "no"
        ],
    )?;

    Ok(())
}



// Use md5 for calculating file hash and add it to the key like this: package_name + version +
// architecture + hash
// create table using a string for unique key and WITHOUT ROWID.
