pub mod models;

use models::{
    Model, Module, Project, Route, RouteUsage, Symbol, SymbolDependency, Translation,
    TranslationUsage,
};
use rusqlite::Connection;
use std::path::Path;

pub trait Database {
    fn open(path: impl AsRef<Path>) -> anyhow::Result<Self>
    where
        Self: Sized;
    fn create_tables(&self) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct SqliteDb {
    pub conn: Connection,
}

impl SqliteDb {
    fn create_table_if_not_exists(&self, table: &str) -> anyhow::Result<()> {
        let sql = format!("CREATE TABLE if not exists {}", table);
        self.conn.execute(&sql, ())?;

        Ok(())
    }
}

impl Database for SqliteDb {
    fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self {
            conn: Connection::open(path)?,
        })
    }

    fn create_tables(&self) -> anyhow::Result<()> {
        self.create_table_if_not_exists(&Project::table())?;
        self.create_table_if_not_exists(&Module::table())?;
        self.create_table_if_not_exists(&Symbol::table())?;
        self.create_table_if_not_exists(&SymbolDependency::table())?;
        self.create_table_if_not_exists(&Translation::table())?;
        self.create_table_if_not_exists(&TranslationUsage::table())?;
        self.create_table_if_not_exists(&Route::table())?;
        self.create_table_if_not_exists(&RouteUsage::table())?;

        Ok(())
    }
}

#[test]
fn db_works() {
    let db =
        SqliteDb::open("/Users/linweitang/rust/js-symbol-dependency-tracker/database/test.db3")
            .unwrap();
    db.create_tables().unwrap();
    println!("{:#?}", db);

    for project in db
        .conn
        .prepare("SELECT * FROM project")
        .unwrap()
        .query_map([], Project::from_row)
        .unwrap()
    {
        println!("{:?}", project.unwrap());
    }

    for module in db
        .conn
        .prepare("SELECT * FROM module")
        .unwrap()
        .query_map([], Module::from_row)
        .unwrap()
    {
        println!("{:?}", module.unwrap());
    }

    for symbol in db
        .conn
        .prepare("SELECT * FROM symbol")
        .unwrap()
        .query_map([], Symbol::from_row)
        .unwrap()
    {
        println!("{:?}", symbol.unwrap());
    }
}
