use arrow::datatypes::{DataType as ArrowDataType, TimeUnit};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Timestamp,
}

impl DataType {
    pub fn to_arrow_type(&self) -> ArrowDataType {
        match self {
            DataType::String => ArrowDataType::Utf8,
            DataType::Integer => ArrowDataType::Int64,
            DataType::Float => ArrowDataType::Float64,
            DataType::Boolean => ArrowDataType::Boolean,
            DataType::Date => ArrowDataType::Date32,
            DataType::DateTime | DataType::Timestamp => {
                ArrowDataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
            }
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::String => write!(f, "string"),
            DataType::Integer => write!(f, "integer"),
            DataType::Float => write!(f, "float"),
            DataType::Boolean => write!(f, "boolean"),
            DataType::Date => write!(f, "date"),
            DataType::DateTime => write!(f, "datetime"),
            DataType::Timestamp => write!(f, "timestamp"),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ColumnDefinition {
    pub column: String,
    #[serde(rename = "type")]
    pub column_type: DataType,
}
