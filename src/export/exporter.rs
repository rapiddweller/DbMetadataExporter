// export/exporter.rs
// Handles exporting database metadata to files

use crate::db::models::DbMetaDataSchema;
use anyhow::Result;
use std::fs::File;
use std::io::Write;

pub struct MetadataExporter;

impl MetadataExporter {
    pub fn export_schema_to_file(&self, schema_data: &DbMetaDataSchema, output_file: &str, format: &str) -> Result<()> {
        let serialized = match format {
            "json" => serde_json::to_string_pretty(schema_data)?,
            "yaml" => serde_yaml::to_string(schema_data)?,
            _ => return Err(anyhow::anyhow!("Unsupported format")),
        };
        let mut file = File::create(output_file)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}
