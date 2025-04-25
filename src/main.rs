// src/main.rs
mod app;
mod db;
mod export;
mod datamimic;
mod models;

use clap::Parser;
use anyhow::{Result, anyhow};
use chrono::Utc;
use db::accessors::*;
use db::models::*;
use export::exporter::MetadataExporter;
use datamimic::datamimic::DataMimicModelGenerator;
use app::tui::run_tui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    db_type: Option<String>,
    #[arg(long)]
    connection_string: Option<String>,
    #[arg(long)]
    schema_or_database: Option<String>,
    #[arg(long)]
    output_file: Option<String>,
    #[arg(long, default_value = "json")]
    format: String,
    #[arg(long, default_value_t = false)]
    tui: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.tui {
        run_tui().await?;
        return Ok(());
    }

    let db_type = args.db_type.as_deref().ok_or_else(|| anyhow!("Missing --db-type"))?;
    let connection_string = args.connection_string.as_deref().ok_or_else(|| anyhow!("Missing --connection-string"))?;

    // Determine the correct file extension based on format
    let ext = match args.format.as_str() {
        "yaml" | "yml" => "yaml",
        _ => "json",
    };
    // Set output_file to user value or default to output.<ext>
    let output_file = args.output_file.clone().unwrap_or_else(|| format!("output.{}", ext));

    // Automatically generate datamimic_output based on output_file and format
    let datamimic_output = if let Some(dot_idx) = output_file.rfind('.') {
        format!("{}{}_datamimic.{}", &output_file[..dot_idx], &output_file[dot_idx..dot_idx], ext)
    } else {
        format!("{}_datamimic.{}", output_file, ext)
    };
    let creation_source = "metaextractor".to_string();

    println!("--- Database Metadata Export and DATAMIMIC Generator ---");
    println!("Database Type: {}", db_type);
    println!("Connection: [REDACTED]");
    if let Some(schema_or_db) = &args.schema_or_database {
        println!("Schema/DB Filter: {}", schema_or_db);
    }
    println!("Metadata Output: {} ({})", output_file, args.format);
    println!("DATAMIMIC Output: {}", datamimic_output);
    println!("-------------------------------------------------------");

    let mut db_accessor: Box<dyn DatabaseAccessor> = match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            println!("Initializing PostgreSQL accessor...");
            Box::new(PostgresAccessor::new(connection_string).await?)
        }
        "mysql" | "mariadb" => {
            println!("Initializing MySQL accessor...");
            Box::new(MySqlAccessor::new(connection_string).await?)
        }
        "sqlite" => {
            println!("Initializing SQLite accessor...");
            Box::new(SqliteAccessor::new(connection_string).await?)
        }
        _ => {
            return Err(anyhow!("Unsupported database type: '{}'. Supported types: postgres, mysql, sqlite", db_type));
        }
    };

    let extracted_metadata = db_accessor.extract_full_metadata(args.schema_or_database.as_deref()).await?;
    let final_schema = DbMetaDataSchema {
        id: None,
        system_environment_id: 0, // Not relevant anymore
        tc_creation_src: Some(creation_source.clone()),
        tc_creation: Some(Utc::now()),
        tc_update_src: Some(creation_source),
        tc_update: Some(Utc::now()),
        db_metadata: extracted_metadata,
        user_config_db_metadata: None,
    };

    let exporter = MetadataExporter;
    exporter.export_schema_to_file(&final_schema, &output_file, &args.format)?;

    let generator = DataMimicModelGenerator;
    let datamimic_model = generator.generate_from_metadata(&final_schema.db_metadata, db_type)?;

    generator.export_model_to_file(&datamimic_model, &datamimic_output)?;

    println!("-------------------------------------------------------");
    println!("Process completed successfully!");
    Ok(())
}
