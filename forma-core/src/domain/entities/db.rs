use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use mongodb::bson::{self, oid};
use serde::{Deserialize, Serialize, de::Visitor};

use crate::domain::entities::{FormaError, FormaErrorApp, FormaErrorDatabase, FormaErrorExt};

#[derive(Clone)]
pub struct BsonTime;
#[derive(Clone)]
pub struct ChronoTime;

impl<'de> Visitor<'de> for ChronoTime {
    type Value = chrono::DateTime<Utc>;

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        chrono::DateTime::from_timestamp_millis(v)
            .ok_or(E::invalid_length(i32::MAX as usize, &self))
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "cannot parse timestamp")
    }
}

#[derive(Clone)]
pub struct Timestamp<T>(TimestampInner, PhantomData<T>);
#[derive(Clone)]
struct TimestampInner {
    timestamp: i64,
}

impl<T> Default for Timestamp<T> {
    fn default() -> Self {
        Self(
            TimestampInner {
                timestamp: Utc::now().timestamp_millis(),
            },
            PhantomData,
        )
    }
}

impl<T> Timestamp<T> {
    pub fn from_timestamp(timestamp: i64) -> Result<Self, FormaError> {
        Ok(Self(
            TimestampInner {
                timestamp: chrono::DateTime::from_timestamp_millis(timestamp)
                    .ok_or(FormaError::new(
                        FormaErrorApp::InvalidTimestamp,
                        "cannot parse timestamp",
                    ))?
                    .timestamp_millis(),
            },
            PhantomData,
        ))
    }
}

impl From<bson::DateTime> for Timestamp<BsonTime> {
    fn from(value: bson::DateTime) -> Self {
        Timestamp::<BsonTime>(
            TimestampInner {
                timestamp: value.timestamp_millis(),
            },
            PhantomData,
        )
    }
}

impl Into<bson::DateTime> for Timestamp<BsonTime> {
    fn into(self) -> bson::DateTime {
        bson::DateTime::from_millis(self.0.timestamp)
    }
}

impl From<DateTime<Utc>> for Timestamp<ChronoTime> {
    fn from(value: DateTime<Utc>) -> Self {
        Timestamp::<ChronoTime>(
            TimestampInner {
                timestamp: value.timestamp_millis(),
            },
            PhantomData,
        )
    }
}

impl Into<chrono::DateTime<Utc>> for Timestamp<ChronoTime> {
    fn into(self) -> chrono::DateTime<Utc> {
        chrono::DateTime::from_timestamp_millis(self.0.timestamp).unwrap()
    }
}

impl Serialize for Timestamp<BsonTime> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bson_datetime = bson::DateTime::from_millis(self.0.timestamp);
        bson_datetime.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp<BsonTime> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let time = bson::DateTime::deserialize(deserializer)?;
        Ok(Self::from(time))
    }
}

impl Serialize for Timestamp<ChronoTime> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.0.timestamp)
    }
}

impl<'de> Deserialize<'de> for Timestamp<ChronoTime> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let timestamp = deserializer.deserialize_i64(ChronoTime)?;
        Ok(Self::from(timestamp))
    }
}

pub struct Created;
pub struct Saved;

#[derive(Serialize, Deserialize, Default)]
pub struct Model<T, M, S> {
    id: Option<oid::ObjectId>,
    #[serde(flatten)]
    pub data: T,
    #[serde(flatten)]
    pub metadata: M,

    _state: PhantomData<S>,
}

impl<T, M> Model<T, M, Created> {
    pub fn from_pairs(pairs: ModelPairs<T, M>) -> Model<T, M, Created> {
        Model::<T, M, Created> {
            id: None,
            data: pairs.data,
            metadata: pairs.metadata,

            _state: PhantomData,
        }
    }
}

impl<T, M> Model<T, M, Created> {
    pub fn with_id(self, id: oid::ObjectId) -> Model<T, M, Saved> {
        Model::<T, M, Saved> {
            id: Some(id),
            data: self.data,
            metadata: self.metadata,
            _state: PhantomData,
        }
    }
}

impl<T, M> Model<T, M, Saved> {
    pub fn id(&self) -> oid::ObjectId {
        self.id.unwrap()
    }
}

impl<T, M, S> Model<T, M, S>
where
    Self: Serialize,
{
    pub fn document(&self) -> Result<bson::Document, FormaError> {
        bson::serialize_to_document(self).map_forma_err(
            FormaErrorDatabase::InvalidArgument,
            "failed to serialize model to document",
        )
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModelPairs<T, M> {
    #[serde(flatten)]
    pub data: T,
    #[serde(flatten)]
    pub metadata: M,
}

pub trait Migratable {
    type LatestVersion;

    fn migrate(self) -> (Self::LatestVersion, bool);
}
