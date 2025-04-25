// datamimic/datamimic.rs
// DataMimic model generator and related logic

use crate::db::models::{DatabaseMetadata, DataMimicModel, DataMimicTableConfig, DataMimicColumnConfig};
use anyhow::Result;
use std::fs::File;
use std::io::Write;

pub struct DataMimicModelGenerator;

impl DataMimicModelGenerator {
    pub fn generate_from_metadata(&self, metadata: &DatabaseMetadata, db_type: &str) -> Result<DataMimicModel> {
        let tables = metadata.tables.iter().map(|(full_table_name, table_meta)| {
            // Split schema and table name for non-SQLite, otherwise use "main" as schema
            let (schema, name) = if let Some(idx) = full_table_name.find('.') {
                (&full_table_name[..idx], &full_table_name[idx+1..])
            } else {
                ("main", full_table_name.as_str())
            };
            let columns = table_meta.columns.iter().map(|col| {
                DataMimicColumnConfig {
                    name: col.name.clone(),
                    generator_type: map_db_type_to_datamimic(&col.data_type, db_type),
                    nullable: col.nullable,
                    is_primary_key: col.primary_key,
                }
            }).collect();
            DataMimicTableConfig {
                schema: schema.to_string(),
                name: name.to_string(),
                columns,
            }
        }).collect();
        Ok(DataMimicModel {
            version: env!("CARGO_PKG_VERSION").to_string(),
            source_database_type: db_type.to_string(),
            tables,
        })
    }
    pub fn export_model_to_file(&self, model: &DataMimicModel, output_file: &str) -> Result<()> {
        let serialized = serde_json::to_string_pretty(model)?;
        let mut file = File::create(output_file)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}

fn map_db_type_to_datamimic(data_type: &str, db_type: &str) -> String {
    // Simple mapping, you can extend this as needed
    let t = data_type.to_lowercase();
    match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => match t.as_str() {
            "integer" | "int4" => "int".to_string(),
            "bigint" | "int8" => "bigint".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "text" | "varchar" | "character varying" => "string".to_string(),
            "date" => "date".to_string(),
            "timestamp" | "timestamp without time zone" => "datetime".to_string(),
            _ => "string".to_string(),
        },
        "mysql" => match t.as_str() {
            "int" | "integer" => "int".to_string(),
            "bigint" => "bigint".to_string(),
            "tinyint" => "bool".to_string(),
            "varchar" | "text" | "char" => "string".to_string(),
            "date" => "date".to_string(),
            "datetime" | "timestamp" => "datetime".to_string(),
            _ => "string".to_string(),
        },
        "sqlite" => match t.as_str() {
            "integer" => "int".to_string(),
            "real" => "float".to_string(),
            "text" => "string".to_string(),
            "blob" => "binary".to_string(),
            _ => "string".to_string(),
        },
        _ => "string".to_string(),
    }
}
