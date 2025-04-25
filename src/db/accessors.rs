// db/accessors.rs
// Database accessor implementations for different database systems.

use super::models::*;
use anyhow::{Result, Context};
use async_trait::async_trait;
use sqlx::{self, Row, postgres::PgPoolOptions, mysql::MySqlPoolOptions, sqlite::SqlitePoolOptions};
use std::collections::HashMap;

#[async_trait]
pub trait DatabaseAccessor {
    async fn extract_full_metadata(&mut self, schema_filter: Option<&str>) -> Result<DatabaseMetadata>;
}

// ------------------- PostgreSQL -------------------
pub struct PostgresAccessor {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl PostgresAccessor {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .connect(connection_string)
            .await
            .context("Failed to connect to PostgreSQL")?;
        Ok(Self { pool })
    }

    async fn get_tables(&self, schema: &str) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema = $1")
            .bind(schema)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>("table_name")).collect())
    }

    async fn get_columns_for_table(&self, schema: &str, table: &str) -> Result<Vec<ColumnMetadata>> {
        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable, character_maximum_length FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2"
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| ColumnMetadata {
            name: row.get("column_name"),
            data_type: row.get("data_type"),
            nullable: row.get::<String, _>("is_nullable") == "YES",
            primary_key: false, // set below
            field_length: row.try_get("character_maximum_length").ok(),
            unique: None,
            spec: None,
            is_checked: Some(true),
        }).collect())
    }

    async fn get_primary_keys_for_table(&self, schema: &str, table: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = $1::regclass AND i.indisprimary;"
        )
        .bind(format!("{}.{}", schema, table))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("attname")).collect())
    }

    async fn get_foreign_keys_for_table(&self, schema: &str, table: &str) -> Result<HashMap<String, String>> {
        let rows = sqlx::query(
            "SELECT kcu.column_name, ccu.table_schema, ccu.table_name, ccu.column_name AS foreign_column
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage ccu
               ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1 AND tc.table_name = $2"
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        let mut map = HashMap::new();
        for row in rows {
            let col = row.get::<String, _>("column_name");
            let ref_schema = row.get::<String, _>("table_schema");
            let ref_table = row.get::<String, _>("table_name");
            let ref_col = row.get::<String, _>("foreign_column");
            map.insert(col, format!("{}.{}.{}", ref_schema, ref_table, ref_col));
        }
        Ok(map)
    }
}

#[async_trait]
impl DatabaseAccessor for PostgresAccessor {
    async fn extract_full_metadata(&mut self, schema_filter: Option<&str>) -> Result<DatabaseMetadata> {
        let schema = schema_filter.unwrap_or("public");
        let tables = self.get_tables(schema).await?;
        let mut meta = DatabaseMetadata { tables: HashMap::new() };
        for table in tables {
            let mut columns = self.get_columns_for_table(schema, &table).await?;
            let primary_keys = self.get_primary_keys_for_table(schema, &table).await?;
            for col in columns.iter_mut() {
                col.primary_key = primary_keys.contains(&col.name);
            }
            let foreign_keys = self.get_foreign_keys_for_table(schema, &table).await?;
            meta.tables.insert(format!("{}.{}", schema, table), TableMetadata {
                columns,
                primary_keys,
                foreign_keys,
            });
        }
        Ok(meta)
    }
}

// ------------------- MySQL -------------------
pub struct MySqlAccessor {
    pool: sqlx::Pool<sqlx::MySql>,
}

impl MySqlAccessor {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = MySqlPoolOptions::new()
            .connect(connection_string)
            .await
            .context("Failed to connect to MySQL")?;
        Ok(Self { pool })
    }

    async fn get_tables(&self, db: &str) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema = ?")
            .bind(db)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>("table_name")).collect())
    }

    async fn get_columns_for_table(&self, db: &str, table: &str) -> Result<Vec<ColumnMetadata>> {
        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable, character_maximum_length FROM information_schema.columns WHERE table_schema = ? AND table_name = ?"
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| ColumnMetadata {
            name: row.get("column_name"),
            data_type: row.get("data_type"),
            nullable: row.get::<String, _>("is_nullable") == "YES",
            primary_key: false, // set below
            field_length: row.try_get("character_maximum_length").ok(),
            unique: None,
            spec: None,
            is_checked: Some(true),
        }).collect())
    }

    async fn get_primary_keys_for_table(&self, db: &str, table: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT column_name FROM information_schema.key_column_usage WHERE table_schema = ? AND table_name = ? AND constraint_name = 'PRIMARY'"
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("column_name")).collect())
    }

    async fn get_foreign_keys_for_table(&self, db: &str, table: &str) -> Result<HashMap<String, String>> {
        let rows = sqlx::query(
            "SELECT column_name, referenced_table_schema, referenced_table_name, referenced_column_name FROM information_schema.key_column_usage WHERE table_schema = ? AND table_name = ? AND referenced_table_name IS NOT NULL"
        )
        .bind(db)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        let mut map = HashMap::new();
        for row in rows {
            let col = row.get::<String, _>("column_name");
            let ref_schema = row.get::<String, _>("referenced_table_schema");
            let ref_table = row.get::<String, _>("referenced_table_name");
            let ref_col = row.get::<String, _>("referenced_column_name");
            map.insert(col, format!("{}.{}.{}", ref_schema, ref_table, ref_col));
        }
        Ok(map)
    }
}

#[async_trait]
impl DatabaseAccessor for MySqlAccessor {
    async fn extract_full_metadata(&mut self, db_filter: Option<&str>) -> Result<DatabaseMetadata> {
        let db = db_filter.unwrap_or("information_schema");
        let tables = self.get_tables(db).await?;
        let mut meta = DatabaseMetadata { tables: HashMap::new() };
        for table in tables {
            let mut columns = self.get_columns_for_table(db, &table).await?;
            let primary_keys = self.get_primary_keys_for_table(db, &table).await?;
            for col in columns.iter_mut() {
                col.primary_key = primary_keys.contains(&col.name);
            }
            let foreign_keys = self.get_foreign_keys_for_table(db, &table).await?;
            meta.tables.insert(format!("{}.{}", db, table), TableMetadata {
                columns,
                primary_keys,
                foreign_keys,
            });
        }
        Ok(meta)
    }
}

// ------------------- SQLite -------------------
pub struct SqliteAccessor {
    pool: sqlx::Pool<sqlx::Sqlite>,
}

impl SqliteAccessor {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect(connection_string)
            .await
            .context("Failed to connect to SQLite")?;
        Ok(Self { pool })
    }

    async fn get_tables(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>("name")).collect())
    }

    async fn get_columns_for_table(&self, table: &str) -> Result<Vec<ColumnMetadata>> {
        let rows = sqlx::query(&format!("PRAGMA table_info('{}')", table))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|row| ColumnMetadata {
            name: row.get("name"),
            data_type: row.get("type"),
            nullable: row.get::<i64, _>("notnull") == 0,
            primary_key: row.get::<i64, _>("pk") == 1,
            field_length: None,
            unique: None,
            spec: None,
            is_checked: Some(true),
        }).collect())
    }

    async fn get_primary_keys_for_table(&self, table: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(&format!("PRAGMA table_info('{}')", table))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().filter(|row| row.get::<i64, _>("pk") == 1).map(|row| row.get("name")).collect())
    }

    async fn get_foreign_keys_for_table(&self, table: &str) -> Result<HashMap<String, String>> {
        let rows = sqlx::query(&format!("PRAGMA foreign_key_list('{}')", table))
            .fetch_all(&self.pool)
            .await?;
        let mut map = HashMap::new();
        for row in rows {
            let col = row.get::<String, _>("from");
            let ref_table = row.get::<String, _>("table");
            let ref_col = row.get::<String, _>("to");
            map.insert(col, format!("{}.{}", ref_table, ref_col));
        }
        Ok(map)
    }
}

#[async_trait]
impl DatabaseAccessor for SqliteAccessor {
    async fn extract_full_metadata(&mut self, _schema_or_db_filter: Option<&str>) -> Result<DatabaseMetadata> {
        let tables = self.get_tables().await?;
        let mut meta = DatabaseMetadata { tables: HashMap::new() };
        for table in tables {
            let columns = self.get_columns_for_table(&table).await?;
            let primary_keys = self.get_primary_keys_for_table(&table).await?;
            let foreign_keys = self.get_foreign_keys_for_table(&table).await?;
            meta.tables.insert(table.clone(), TableMetadata {
                columns,
                primary_keys,
                foreign_keys,
            });
        }
        Ok(meta)
    }
}
