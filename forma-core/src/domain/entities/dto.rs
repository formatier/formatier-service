use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{ChronoTime, Timestamp};

#[derive(Serialize, Deserialize)]
pub struct Request<T> {
    #[serde(flatten)]
    pub data: T,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
pub struct Response<T> {
    #[serde(flatten)]
    pub data: T,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {}

#[derive(Serialize, Deserialize)]
pub struct SagaRequest<T> {
    #[serde(flatten)]
    pub data: T,
    pub metadata: SagaTransactionMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct SagaResponse<T> {
    #[serde(flatten)]
    pub data: T,
    pub metadata: SagaTransactionMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct SagaTransactionMetadata {
    id: Uuid,
    create_at: Timestamp<ChronoTime>,
}
