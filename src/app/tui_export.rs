use crate::db::accessors::*;
use crate::export::exporter::MetadataExporter;
use crate::datamimic::datamimic::DataMimicModelGenerator;
use crate::db::models::*;
use anyhow::{Result, anyhow};
use chrono::Utc;

/// Orchestrates the export flow for the TUI, reusing all shared logic from db, export, and datamimic modules.
/// Returns Ok(msg) on success, or Err(error) with context on failure.
pub async fn tui_export_flow(state: &super::tui::TuiState) -> Result<String> {
    let db_type = state.db_types[state.db_type_index];
    let connection_string = &state.connection_string;
    let schema = if !state.schema.is_empty() { Some(state.schema.as_str()) } else { None };
    let output_file = "output.json"; // TODO: let user customize
    let datamimic_output = "output_datamimic.json";
    let format = "json";

    // 1. Create DB accessor (delegated to db::accessors)
    let mut accessor: Box<dyn DatabaseAccessor + Send> = match db_type {
        "sqlite" => Box::new(SqliteAccessor::new(connection_string).await.map_err(|e| anyhow!("SQLite connection failed: {}", e))?),
        "postgres" => Box::new(PostgresAccessor::new(connection_string).await.map_err(|e| anyhow!("Postgres connection failed: {}", e))?),
        "mysql" => Box::new(MySqlAccessor::new(connection_string).await.map_err(|e| anyhow!("MySQL connection failed: {}", e))?),
        _ => return Err(anyhow!("Unsupported DB type: {}", db_type)),
    };

    // 2. Extract metadata (delegated to db::accessors)
    let extracted_metadata = accessor.extract_full_metadata(schema).await.map_err(|e| anyhow!("Metadata extraction failed: {}", e))?;
    let final_schema = DbMetaDataSchema {
        id: None,
        system_environment_id: 0,
        tc_creation_src: None,
        tc_creation: Some(Utc::now()),
        tc_update_src: None,
        tc_update: Some(Utc::now()),
        db_metadata: extracted_metadata,
        user_config_db_metadata: None,
    };

    // 3. Export metadata (delegated to export::exporter)
    let exporter = MetadataExporter;
    exporter.export_schema_to_file(&final_schema, output_file, format).map_err(|e| anyhow!("Export to file failed: {}", e))?;

    // 4. Generate DataMimic model (delegated to datamimic::datamimic)
    let generator = DataMimicModelGenerator;
    let datamimic_model = generator.generate_from_metadata(&final_schema.db_metadata, db_type).map_err(|e| anyhow!("DataMimic model generation failed: {}", e))?;
    generator.export_model_to_file(&datamimic_model, datamimic_output).map_err(|e| anyhow!("Export DataMimic model failed: {}", e))?;

    Ok("Export completed!".to_string())
}
