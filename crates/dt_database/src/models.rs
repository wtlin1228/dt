use rusqlite::{params, Connection, Row, ToSql};

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

    /// single thread only: last_insert_rowid()
    pub fn create(conn: &Connection, path: &str) -> anyhow::Result<Self> {
        conn.execute("INSERT INTO project (path) VALUES (?1)", params![path])?;
        let project = conn.query_row(
            "SELECT * FROM project WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(project)
    }

    pub fn add_module(&self, conn: &Connection, path: &str) -> anyhow::Result<Module> {
        Module::create(conn, self, path)
    }

    pub fn get_module(&self, conn: &Connection, path: &str) -> anyhow::Result<Module> {
        Module::retrieve(conn, self, path)
    }

    /// single thread only: retrieve then create
    pub fn get_or_create_module(&self, conn: &Connection, path: &str) -> anyhow::Result<Module> {
        match Module::retrieve(conn, self, path) {
            Ok(module) => Ok(module),
            Err(_) => self.add_module(conn, path),
        }
    }

    pub fn add_translation(
        &self,
        conn: &Connection,
        key: &str,
        value: &str,
    ) -> anyhow::Result<Translation> {
        Translation::create(conn, self, key, value)
    }

    pub fn get_translation(&self, conn: &Connection, key: &str) -> anyhow::Result<Translation> {
        Translation::retrieve(conn, self, key)
    }

    pub fn add_route(&self, conn: &Connection, path: &str) -> anyhow::Result<Route> {
        Route::create(conn, self, path)
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

    /// single thread only: last_insert_rowid()
    pub fn create(conn: &Connection, project: &Project, path: &str) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO module (project_id, path) VALUES (?1, ?2)",
            params![project.id, path],
        )?;
        let module = conn.query_row(
            "SELECT * FROM module WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(module)
    }

    pub fn retrieve(conn: &Connection, project: &Project, path: &str) -> anyhow::Result<Self> {
        let module = conn.query_row(
            "SELECT * FROM module WHERE (project_id, path) = (?1, ?2)",
            params![project.id, path],
            Self::from_row,
        )?;
        Ok(module)
    }

    pub fn add_symbol(
        &self,
        conn: &Connection,
        variant: SymbolVariant,
        name: &str,
    ) -> anyhow::Result<Symbol> {
        Symbol::create(conn, self, variant, name)
    }

    pub fn get_symbol(
        &self,
        conn: &Connection,
        variant: SymbolVariant,
        name: &str,
    ) -> anyhow::Result<Symbol> {
        Symbol::retrieve(conn, self, variant, name)
    }

    /// single thread only: retrieve then create
    pub fn get_or_create_symbol(
        &self,
        conn: &Connection,
        variant: SymbolVariant,
        name: &str,
    ) -> anyhow::Result<Symbol> {
        match Symbol::retrieve(conn, self, variant, name) {
            Ok(symbol) => Ok(symbol),
            Err(_) => self.add_symbol(conn, variant, name),
        }
    }

    pub fn get_named_export_symbols(&self, conn: &Connection) -> anyhow::Result<Vec<Symbol>> {
        let named_export_symbols: Vec<Symbol> = conn
            .prepare("SELECT * FROM symbol WHERE (module_id, variant) = (?1, ?2)")?
            .query_map(
                params![self.id, SymbolVariant::NamedExport],
                Symbol::from_row,
            )?
            .map(|s| s.unwrap())
            .collect();
        Ok(named_export_symbols)
    }
}

#[derive(Debug, Clone, Copy)]
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

impl ToSql for SymbolVariant {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SymbolVariant::LocalVariable => 0.to_sql(),
            SymbolVariant::NamedExport => 1.to_sql(),
            SymbolVariant::DefaultExport => 2.to_sql(),
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

    /// single thread only: last_insert_rowid()
    pub fn create(
        conn: &Connection,
        module: &Module,
        variant: SymbolVariant,
        name: &str,
    ) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO symbol (module_id, variant, name) VALUES (?1, ?2, ?3)",
            params![module.id, variant, name],
        )?;
        let symbol = conn.query_row(
            "SELECT * FROM symbol WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(symbol)
    }

    pub fn retrieve(
        conn: &Connection,
        module: &Module,
        variant: SymbolVariant,
        name: &str,
    ) -> anyhow::Result<Self> {
        let symbol = conn.query_row(
            "SELECT * FROM symbol WHERE (module_id, variant, name) = (?1, ?2, ?3)",
            params![module.id, variant, name],
            Self::from_row,
        )?;
        Ok(symbol)
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

    /// single thread only: last_insert_rowid()
    pub fn create(
        conn: &Connection,
        current_symbol: &Symbol,
        depend_on_symbol: &Symbol,
    ) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO symbol_dependency (symbol_id, depend_on_symbol_id) VALUES (?1, ?2)",
            params![current_symbol.id, depend_on_symbol.id],
        )?;
        let symbol_dependency = conn.query_row(
            "SELECT * FROM symbol_dependency WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(symbol_dependency)
    }
}

#[derive(Debug)]
pub struct Translation {
    pub id: usize,
    pub project_id: usize,
    pub key: String,
    pub value: String,
}

impl Model for Translation {
    fn table() -> String {
        "
        translation (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER REFERENCES project(id) ON DELETE CASCADE,
            key        TEXT NOT NULL,
            value      TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Translation {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            project_id: row.get(1)?,
            key: row.get(2)?,
            value: row.get(3)?,
        })
    }

    /// single thread only: last_insert_rowid()
    pub fn create(
        conn: &Connection,
        project: &Project,
        key: &str,
        value: &str,
    ) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO translation (project_id, key, value) VALUES (?1, ?2, ?3)",
            params![project.id, key, value],
        )?;
        let translation = conn.query_row(
            "SELECT * FROM translation WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(translation)
    }

    pub fn retrieve(conn: &Connection, project: &Project, key: &str) -> anyhow::Result<Self> {
        let translation = conn.query_row(
            "SELECT * FROM translation WHERE (project_id, key) = (?1, ?2)",
            params![project.id, key],
            Self::from_row,
        )?;
        Ok(translation)
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

    /// single thread only: last_insert_rowid()
    pub fn create(
        conn: &Connection,
        translation: &Translation,
        symbol: &Symbol,
    ) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO translation_usage (translation_id, symbol_id) VALUES (?1, ?2)",
            params![translation.id, symbol.id],
        )?;
        let translation = conn.query_row(
            "SELECT * FROM translation_usage WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(translation)
    }
}

#[derive(Debug)]
pub struct Route {
    pub id: usize,
    pub project_id: usize,
    pub path: String,
}

impl Model for Route {
    fn table() -> String {
        "
        route (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER REFERENCES project(id) ON DELETE CASCADE,
            path       TEXT NOT NULL
        )
        "
        .to_string()
    }
}

impl Route {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            project_id: row.get(1)?,
            path: row.get(2)?,
        })
    }

    /// single thread only: last_insert_rowid()
    pub fn create(conn: &Connection, project: &Project, path: &str) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO route (project_id, path) VALUES (?1, ?2)",
            params![project.id, path],
        )?;
        let route = conn.query_row(
            "SELECT * FROM route WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(route)
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
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            route_id  INTEGER REFERENCES route(id) ON DELETE CASCADE,
            symbol_id INTEGER REFERENCES symbol(id) ON DELETE CASCADE
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

    /// single thread only: last_insert_rowid()
    pub fn create(conn: &Connection, route: &Route, symbol: &Symbol) -> anyhow::Result<Self> {
        conn.execute(
            "INSERT INTO route_usage (route_id, symbol_id) VALUES (?1, ?2)",
            params![route.id, symbol.id],
        )?;
        let route = conn.query_row(
            "SELECT * FROM route_usage WHERE id=last_insert_rowid()",
            (),
            Self::from_row,
        )?;
        Ok(route)
    }
}
