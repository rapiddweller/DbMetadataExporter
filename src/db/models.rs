// Data structures for metadata and DataMimic models

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AttributeSpecification {
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ColumnMetadata {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub field_length: Option<i64>,
    pub unique: Option<bool>,
    pub spec: Option<AttributeSpecification>,
    #[serde(rename = "isChecked")]
    pub is_checked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TableMetadata {
    pub columns: Vec<ColumnMetadata>,
    pub primary_keys: Vec<String>,
    pub foreign_keys: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DatabaseMetadata {
    pub tables: HashMap<String, TableMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DbMetaDataSchema {
    pub id: Option<i64>,
    pub system_environment_id: i64,
    pub tc_creation_src: Option<String>,
    pub tc_creation: Option<DateTime<Utc>>,
    pub tc_update_src: Option<String>,
    pub tc_update: Option<DateTime<Utc>>,
    pub db_metadata: DatabaseMetadata,
    pub user_config_db_metadata: Option<DatabaseMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataMimicColumnConfig {
    pub name: String,
    pub generator_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataMimicTableConfig {
    pub schema: String,
    pub name: String,
    pub columns: Vec<DataMimicColumnConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataMimicModel {
    pub version: String,
    pub source_database_type: String,
    pub tables: Vec<DataMimicTableConfig>,
}
