use std::{error::Error, fmt::Display};

use axum::{http::StatusCode, response::IntoResponse};
use mongodb::bson::{self};
use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::inline_mod;

pub mod auth_service;

inline_mod!(db, dto);

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, Default)]
#[error("{kind} :: {message}")]
pub struct FormaError {
    #[source]
    pub kind: FormaErrorKind,
    pub message: String,
    pub detail: Option<FormaErrorDetail>,

    #[serde(skip)]
    pub internal_info: Option<IntenalInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormaErrorDetail {
    Key {
        key: String,
        message: Option<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub enum InternalLevel {
    Panic,
    ClusterException,
    #[default]
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct IntenalInfo {
    pub target_kind: Option<String>,
    pub internal_kind: FormaErrorKind,
    pub message: Option<String>,
    pub stack_trace: Option<String>,
    pub level: InternalLevel,
}

impl FormaError {
    pub fn new<T: Into<FormaErrorKind>>(kind: T, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            ..Default::default()
        }
    }

    pub fn new_with_detail<T: Into<FormaErrorKind>>(
        kind: T,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            detail,
            ..Default::default()
        }
    }

    pub fn get_status(&self) -> StatusCode {
        match &self.kind {
            FormaErrorKind::FormaError(err) => match err {
                FormaErrorApp::InvalidType => StatusCode::BAD_REQUEST,
                FormaErrorApp::InvalidTimestamp => StatusCode::UNPROCESSABLE_ENTITY,
                FormaErrorApp::InvalidCredentials | FormaErrorApp::InvalidVersionNumber => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                FormaErrorApp::DataConflict => StatusCode::CONFLICT,
            },
            FormaErrorKind::AuthError(err) => match err {
                FormaErrorAuth::InternalEncryptionFail => StatusCode::INTERNAL_SERVER_ERROR,
                FormaErrorAuth::InvalidSignature => StatusCode::FORBIDDEN,
                FormaErrorAuth::TokenExpired
                | FormaErrorAuth::TokenInvalid
                | FormaErrorAuth::TokenRevoked => StatusCode::UNAUTHORIZED,
                FormaErrorAuth::InvalidUserAgent => StatusCode::IM_A_TEAPOT,
            },
            FormaErrorKind::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            FormaErrorKind::ExternalServiceError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            FormaErrorKind::OAuthError(err) => match err {
                FormaErrorOAuth::Unauthorized => StatusCode::UNAUTHORIZED,
                FormaErrorOAuth::NotFound => StatusCode::NOT_FOUND,
            },
            FormaErrorKind::CacheError(err) => match err {
                FormaErrorCache::CacheUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                FormaErrorCache::IOError => StatusCode::INTERNAL_SERVER_ERROR,
                FormaErrorCache::Timeout => StatusCode::REQUEST_TIMEOUT,
            },
            FormaErrorKind::Internal | FormaErrorKind::Unhandled => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn response(self) -> impl IntoResponse {
        let body = serde_json::to_string(&self).unwrap();
        axum::http::Response::builder()
            .status(self.get_status())
            .body(body)
            .unwrap()
    }
}

pub trait FormaErrorExt<T> {
    fn map_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError>;
    fn map_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError>;
}

impl<T, E> FormaErrorExt<T> for Result<T, E>
where
    E: Display + std::error::Error,
{
    fn map_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError> {
        self.map_err(|err| FormaError {
            kind: kind.into(),
            message: format!("{}", message),
            detail: None,
            internal_info: Some(IntenalInfo {
                target_kind: None,
                internal_kind: FormaErrorKind::Internal,
                message: Some(err.to_string()),
                stack_trace: err.source().map(|v| v.to_string()),
                level: InternalLevel::Error,
            }),
        })
    }

    fn map_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError> {
        self.map_err(|err| FormaError {
            kind: kind.into(),
            message: format!("{} {}", message, err.to_string()),
            detail: detail,
            internal_info: Some(IntenalInfo {
                target_kind: None,
                internal_kind: FormaErrorKind::Internal,
                message: Some(err.to_string()),
                stack_trace: err.source().map(|v| v.to_string()),
                level: InternalLevel::Error,
            }),
        })
    }
}

pub trait FormaErrorMongoExt<T> {
    fn map_mongo_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError>;
    fn map_mongo_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError>;
}

impl<T> FormaErrorMongoExt<T> for Result<T, mongodb::error::Error> {
    fn map_mongo_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError> {
        self.map_mongo_forma_err_with_detail(kind, message, None)
    }

    fn map_mongo_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError> {
        self.map_err(move |err| {
            let intenal_info = expand_mongodb_error(err);

            FormaError {
                kind: kind.into(),
                message: message.into(),
                detail,
                internal_info: Some(intenal_info),
            }
        })
    }
}

pub trait FormaErrorRedisExt<T> {
    fn map_redis_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError>;
    fn map_redis_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError>;
}

impl<T> FormaErrorRedisExt<T> for Result<T, redis::RedisError> {
    fn map_redis_forma_err<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
    ) -> Result<T, FormaError> {
        self.map_redis_forma_err_with_detail(kind, message, None)
    }

    fn map_redis_forma_err_with_detail<K: Into<FormaErrorKind>>(
        self,
        kind: K,
        message: &str,
        detail: Option<FormaErrorDetail>,
    ) -> Result<T, FormaError> {
        self.map_err(|err| {
            let internal_info = expand_redis_error(err);
            FormaError {
                kind: kind.into(),
                message: message.to_string(),
                detail,
                internal_info: Some(internal_info),
            }
        })
    }
}

fn expand_mongodb_error(value: mongodb::error::Error) -> IntenalInfo {
    let mut kind: FormaErrorKind = FormaErrorKind::Unhandled;
    let mut message: Option<String> = None;
    let mut level: InternalLevel = InternalLevel::Error;

    match value.kind.as_ref() {
        mongodb::error::ErrorKind::InvalidArgument { message: msg, .. } => {
            kind = FormaErrorDatabase::InvalidArgument.into();
            message = Some(msg.clone());
        }
        mongodb::error::ErrorKind::Bson(err) => {
            message = Some(err.to_string());

            match err.kind {
                bson::error::ErrorKind::Binary { .. }
                | bson::error::ErrorKind::Decimal128 { .. }
                | bson::error::ErrorKind::DateTime { .. } => {
                    kind = FormaErrorApp::DataConflict.into();
                }

                bson::error::ErrorKind::ObjectId { .. } | bson::error::ErrorKind::Uuid { .. } => {
                    kind = FormaErrorDatabase::InvalidObjectId.into();
                }

                bson::error::ErrorKind::TooLargeUnsignedInteger { .. }
                | bson::error::ErrorKind::Utf8Encoding { .. } => {
                    kind = FormaErrorDatabase::InvalidArgument.into()
                }

                _ => kind = FormaErrorDatabase::IoStuck.into(),
            };
        }
        mongodb::error::ErrorKind::InsertMany(_) | mongodb::error::ErrorKind::BulkWrite(_) => {
            kind = FormaErrorDatabase::InsertError.into();
        }
        mongodb::error::ErrorKind::DnsResolve { message: msg, .. }
        | mongodb::error::ErrorKind::ServerSelection { message: msg, .. } => {
            kind = FormaErrorExternalService::ConnectionError.into();
            message = Some(msg.clone());
            level = InternalLevel::ClusterException;
        }
        mongodb::error::ErrorKind::Io(_)
        | mongodb::error::ErrorKind::InvalidResponse { .. }
        | mongodb::error::ErrorKind::Shutdown => {
            kind = FormaErrorDatabase::IoStuck.into();
            level = InternalLevel::Panic;
        }
        mongodb::error::ErrorKind::ConnectionPoolCleared { message: msg, .. } => {
            kind = FormaErrorExternalService::PoolError.into();
            message = Some(msg.clone());
        }
        mongodb::error::ErrorKind::IncompatibleServer { .. } => {
            kind = FormaErrorExternalService::Incompatible.into();
        }
        mongodb::error::ErrorKind::MissingResumeToken => {
            kind = FormaErrorKind::Unhandled;
        }
        _ => {}
    };

    let message = format!("{} {}", value.to_string(), message.unwrap_or_default());

    IntenalInfo {
        target_kind: Some(value.kind.to_string()),
        internal_kind: kind,
        message: Some(message),
        stack_trace: value.source().map(|e| e.to_string()),
        level,
    }
}

fn expand_redis_error(value: redis::RedisError) -> IntenalInfo {
    let (kind, level) = {
        if value.is_timeout() {
            (FormaErrorCache::Timeout.into(), InternalLevel::Error)
        } else if value.is_io_error() {
            (
                FormaErrorCache::CacheUnavailable.into(),
                InternalLevel::Error,
            )
        } else if value.is_connection_dropped() || value.is_connection_refusal() {
            (
                FormaErrorExternalService::ConnectionError.into(),
                InternalLevel::ClusterException,
            )
        } else if value.is_cluster_error() {
            (
                FormaErrorCache::CacheUnavailable.into(),
                InternalLevel::ClusterException,
            )
        } else {
            match value.kind() {
                redis::ErrorKind::AuthenticationFailed | redis::ErrorKind::InvalidClientConfig => {
                    (FormaErrorKind::Internal, InternalLevel::Panic)
                }
                _ => (FormaErrorKind::Unhandled, InternalLevel::Error),
            }
        }
    };

    IntenalInfo {
        target_kind: Some(format!("{:?}", value.kind())),
        internal_kind: kind,
        message: Some(value.to_string()),
        stack_trace: value.source().map(|v| v.to_string()),
        level: level,
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FormaErrorKind {
    #[error("oauth error :: {0}")]
    OAuthError(#[from] FormaErrorOAuth),
    #[error("database error :: {0}")]
    DatabaseError(#[from] FormaErrorDatabase),
    #[error("auth error :: {0}")]
    AuthError(#[from] FormaErrorAuth),
    #[error("app error :: {0}")]
    FormaError(#[from] FormaErrorApp),
    #[error("external serice error :: {0}")]
    ExternalServiceError(#[from] FormaErrorExternalService),
    #[error("cache error :: {0}")]
    CacheError(#[from] FormaErrorCache),
    #[error("internal")]
    Internal,
    #[default]
    #[error("unhandled")]
    Unhandled,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorAuth {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("token invalid")]
    TokenInvalid,
    #[error("token revoked")]
    TokenRevoked,
    #[error("token expired")]
    TokenExpired,
    #[error("internal encryption fail")]
    InternalEncryptionFail,
    #[error("invalid user agent")]
    InvalidUserAgent,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorOAuth {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorApp {
    #[error("invalid type")]
    InvalidType,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid version number")]
    InvalidVersionNumber,
    #[error("data conflict")]
    DataConflict,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorDatabase {
    #[error("invalid object id")]
    InvalidObjectId,
    #[error("duplicate key")]
    DuplicateKey,
    #[error("invalid argument")]
    InvalidArgument,
    #[error("io stuck")]
    IoStuck,
    #[error("insert error")]
    InsertError,
    #[error("select error")]
    SelectError,
    #[error("not found")]
    NotFound,
    #[error("update error")]
    UpdateError,
    #[error("delete error")]
    DeleteError,
    #[error("transaction error")]
    TransactionError,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorExternalService {
    #[error("pool error")]
    PoolError,
    #[error("connection error")]
    ConnectionError,
    #[error("incompatible")]
    Incompatible,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormaErrorCache {
    #[error("cache unavailable")]
    CacheUnavailable,
    #[error("time out")]
    Timeout,
    #[error("io error")]
    IOError,
}

#[derive(FromRepr, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MajorVersion {
    Candidate = 0,
    Valentina = 1,
}

impl MajorVersion {
    fn into_u16(&self) -> u16 {
        self.clone() as u16
    }
}

#[derive(FromRepr, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum VersionProfile {
    Release = 0,
    Beta = 1,
}

impl VersionProfile {
    fn into_u16(&self) -> u16 {
        self.clone() as u16
    }
}

#[derive(FromRepr, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Quarter {
    First = 0,
    Second = 1,
    Third = 2,
    Fourth = 3,
}

impl Quarter {
    fn into_u16(&self) -> u16 {
        self.clone() as u16
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Patch(u8);

impl Patch {
    pub fn new(value: u8) -> Result<Self, FormaError> {
        if value > max_value_of_bit(7) {
            Err(FormaError::new(
                FormaErrorApp::InvalidVersionNumber,
                "invalid major version",
            ))
        } else {
            Ok(Patch(value))
        }
    }
}

impl TryFrom<u8> for Patch {
    type Error = FormaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > max_value_of_bit(7) {
            Err(FormaError::new(
                FormaErrorApp::InvalidVersionNumber,
                "invalid major version",
            ))
        } else {
            Ok(Patch(value))
        }
    }
}

impl Patch {
    fn into_u16(&self) -> u16 {
        self.0 as u16
    }
}

const fn max_value_of_bit(length: u8) -> u8 {
    if length >= 8 {
        u8::MAX
    } else {
        (1 << length) - 1
    }
}

/* การเก็บเลขเวอร์ชั่นแบบ 2 bytes
 *
 * 3/4 byte สุดท้ายของ byte ที่สอง หรือ xxxxxx00 00000000
 * เก็บเลข version release ใหญ่
 *
 * 1/4 byte ด้านหน้าสุดของ byte ที่สอง หรือ 000000xx 00000000
 * เก็บเลข ไตรมาส bitNumber = x - 1
 *
 * 7 bits สุดท้ายของ byte ที่ 1 หรือ 00000000 xxxxxxx0
 * เก็บเลข patch
 *
 * 1 bit ด้านหน้าสุดของ byte ที่ 1 หรือ 00000000 0000000x
 * เก็บ release profile
 * 0 คือ release
 * 1 คือ beta
*/
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "u16")]
#[serde(into = "u16")]
pub struct Version {
    major_version: MajorVersion,
    quarter: Quarter,
    patch: Patch,
    version_profile: VersionProfile,
}

impl Version {
    fn new(
        major_version: MajorVersion,
        quarter: Quarter,
        patch: Patch,
        version_profile: VersionProfile,
    ) -> Self {
        Self {
            major_version,
            quarter,
            patch,
            version_profile,
        }
    }
}

impl TryFrom<u16> for Version {
    type Error = FormaError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let major_version = ((value >> 10) & 0b111111) as u8;
        let quarter = ((value >> 8) & 0b11) as u8;
        let patch = ((value >> 1) & 0b1111111) as u8;
        let version_profile = (value & 0b1) as u8;

        Ok(Version {
            major_version: MajorVersion::from_repr(major_version).ok_or(FormaError::new(
                FormaErrorApp::InvalidVersionNumber,
                "invalid major version",
            ))?,
            quarter: Quarter::from_repr(quarter).ok_or(FormaError::new(
                FormaErrorApp::InvalidVersionNumber,
                "invalid quarter",
            ))?,
            patch: patch.try_into().map_err(|e| {
                FormaError::new(
                    FormaErrorApp::InvalidVersionNumber,
                    format!("invalid patch: {}", e),
                )
            })?,
            version_profile: VersionProfile::from_repr(version_profile).ok_or(FormaError::new(
                FormaErrorApp::InvalidVersionNumber,
                "invalid version profile",
            ))?,
        })
    }
}

impl Into<u16> for Version {
    fn into(self) -> u16 {
        let mut version_number = 0u16;

        version_number |= (self.major_version.into_u16()) << 10;
        version_number |= (self.quarter.into_u16()) << 8;
        version_number |= (self.patch.into_u16()) << 1;
        version_number |= self.version_profile.into_u16();

        version_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_version() {
        let version = Version::new(
            MajorVersion::Candidate,
            Quarter::Second,
            Patch::try_from(max_value_of_bit(7)).unwrap(),
            VersionProfile::Release,
        );

        let version_number: u16 = version.clone().into();

        let parsed_version: Version = version_number.try_into().unwrap();

        assert_eq!(
            version, parsed_version,
            "version and parsed version from version number are equal"
        );
    }

    #[test]
    #[should_panic]
    fn test_compile_version_error() {
        Version::new(
            MajorVersion::Candidate,
            Quarter::Second,
            Patch::try_from(max_value_of_bit(8)).unwrap(),
            VersionProfile::Release,
        );
    }
}
