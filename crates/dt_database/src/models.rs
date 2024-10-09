use rusqlite::{params, Connection, Row};

pub trait Model {
    fn table() -> String;
}

#[derive(Debug)]
pub struct Project {
    pub id: usize,
    pub path: String,
}

impl Model for Project {
    fn table() -> String {
        "
        project (
            id   INTEGER PRIMARY KEY,
            path TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Project {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            path: row.get(1)?,
        })
    }

    pub fn create(conn: &Connection, path: &str) -> anyhow::Result<Self> {
        conn.execute("INSERT INTO project (path) VALUES (?1)", params![path])?;
        let id = conn.query_row("SELECT last_insert_rowid()", (), |row| {
            Ok(row.get::<_, usize>(0)?)
        })?;
        let project = conn.query_row(
            "SELECT * FROM project where id=?1",
            params![id],
            Self::from_row,
        )?;
        Ok(project)
    }

    pub fn add_module(&self, conn: &Connection, path: &str) -> anyhow::Result<Module> {
        Module::create(conn, self, path)
    }
}

#[derive(Debug)]
pub struct Module {
    pub id: usize,
    pub project_id: usize,
    pub path: String,
}

impl Model for Module {
    fn table() -> String {
        "
        module (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER REFERENCES project(id) ON DELETE CASCADE,
            path       TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Module {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            project_id: row.get(1)?,
            path: row.get(2)?,
        })
    }

    pub fn create(conn: &Connection, project: &Project, path: &str) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO module (project_id, path) VALUES (?1, ?2)",
            params![project.id, path],
        )?;
        let id = conn.query_row("SELECT last_insert_rowid()", (), |row| {
            Ok(row.get::<_, usize>(0)?)
        })?;
        let module = conn.query_row(
            "SELECT * FROM module where id=?1",
            params![id],
            Self::from_row,
        )?;
        Ok(module)
    }
}

#[derive(Debug)]
pub enum SymbolVariant {
    LocalVariable,
    NamedExport,
    DefaultExport,
}

impl SymbolVariant {
    pub fn from(n: usize) -> Self {
        match n {
            0 => Self::LocalVariable,
            1 => Self::NamedExport,
            2 => Self::DefaultExport,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Symbol {
    pub id: usize,
    pub module_id: usize,
    pub variant: SymbolVariant,
    pub name: String,
}

impl Model for Symbol {
    fn table() -> String {
        "
        symbol (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            module_id INTEGER REFERENCES module(id) ON DELETE CASCADE,
            variant   INTEGER NOT NULL,
            name      TEXT    NOT NULL
        )
        "
        .to_string()
    }
}

impl Symbol {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            module_id: row.get(1)?,
            variant: SymbolVariant::from(row.get::<_, usize>(2)?),
            name: row.get(3)?,
        })
    }
}

// Join Table
#[derive(Debug)]
pub struct SymbolDependency {
    pub id: usize,
    pub symbol_id: usize,
    pub depend_on_symbol_id: usize,
}

impl Model for SymbolDependency {
    fn table() -> String {
        "
        symbol_dependency (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol_id           INTEGER REFERENCES symbol(id) ON DELETE CASCADE,
            depend_on_symbol_id INTEGER REFERENCES symbol(id) ON DELETE CASCADE
        )
        "
        .to_string()
    }
}

impl SymbolDependency {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            symbol_id: row.get(1)?,
            depend_on_symbol_id: row.get(2)?,
        })
    }
}

#[derive(Debug)]
pub struct Translation {
    pub id: usize,
    pub key: String,
    pub value: String,
}

impl Model for Translation {
    fn table() -> String {
        "
        translation (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            key   TEXT NOT NULL,
            value TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Translation {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
        })
    }
}

// Join Table
#[derive(Debug)]
pub struct TranslationUsage {
    pub id: usize,
    pub translation_id: usize,
    pub symbol_id: usize,
}

impl Model for TranslationUsage {
    fn table() -> String {
        "
        translation_usage (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            translation_id INTEGER REFERENCES translation(id) ON DELETE CASCADE,
            symbol_id      INTEGER REFERENCES symbol(id) ON DELETE CASCADE
        )
        "
        .to_string()
    }
}

impl TranslationUsage {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            translation_id: row.get(1)?,
            symbol_id: row.get(2)?,
        })
    }
}

#[derive(Debug)]
pub struct Route {
    pub id: usize,
    pub path: String,
}

impl Model for Route {
    fn table() -> String {
        "
        route (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Route {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            path: row.get(1)?,
        })
    }
}

// Join Table
#[derive(Debug)]
pub struct RouteUsage {
    pub id: usize,
    pub route_id: usize,
    pub symbol_id: usize,
}

impl Model for RouteUsage {
    fn table() -> String {
        "
        route_usage (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            translation_id INTEGER REFERENCES route(id) ON DELETE CASCADE,
            symbol_id      INTEGER REFERENCES symbol(id) ON DELETE CASCADE
        )
        "
        .to_string()
    }
}

impl RouteUsage {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            route_id: row.get(1)?,
            symbol_id: row.get(2)?,
        })
    }
}
