use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub enum MetadataValue {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Vector(Vec<MetadataValue>),

    // non-string keys are painful, serde_yaml does evil things to accomplish it
    Mapping(HashMap<String, MetadataValue>),
}

impl MetadataValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MetadataValue::String(s) => Some(s.deref()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetadataValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_mapping(&self) -> Option<&HashMap<String, MetadataValue>> {
        match self {
            MetadataValue::Mapping(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&Vec<MetadataValue>> {
        match self {
            MetadataValue::Vector(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum YamlToValueError {
    #[error("non-string keys are unsupported")]
    NonStringKey,
}

impl TryFrom<serde_yaml::Value> for MetadataValue {
    type Error = YamlToValueError;

    fn try_from(value: serde_yaml::Value) -> Result<Self, Self::Error> {
        Ok(match value {
            serde_yaml::Value::Null => MetadataValue::Null,
            serde_yaml::Value::Bool(b) => MetadataValue::Bool(b),
            serde_yaml::Value::Number(n) => MetadataValue::Number(n.as_f64().unwrap()),
            serde_yaml::Value::String(s) => MetadataValue::String(s.into()),
            serde_yaml::Value::Sequence(v) => MetadataValue::Vector(
                v.into_iter()
                    .map(MetadataValue::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            serde_yaml::Value::Mapping(m) => MetadataValue::Mapping(yaml_mapping_to_value_map(m)?),
            serde_yaml::Value::Tagged(t) => MetadataValue::Mapping(
                [
                    // tag with leading "!" - no other way to access tag
                    (
                        "tag".to_owned(),
                        MetadataValue::String(t.tag.to_string().into()),
                    ),
                    ("value".to_owned(), t.value.try_into()?),
                ]
                .into(),
            ),
        })
    }
}

pub fn yaml_mapping_to_value_map(
    mapping: serde_yaml::Mapping,
) -> Result<HashMap<String, MetadataValue>, YamlToValueError> {
    mapping
        .into_iter()
        .map(
            |(k, v)| -> Result<(String, MetadataValue), YamlToValueError> {
                Ok((
                    k.as_str().ok_or(YamlToValueError::NonStringKey)?.to_owned(),
                    MetadataValue::try_from(v)?,
                ))
            },
        )
        .collect::<Result<_, _>>()
}
