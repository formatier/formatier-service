use std::collections::HashMap;

use forma_proc_macro::serde_migrator;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::{
    domain::entities::{BsonTime, ChronoTime, Migratable, Model, ModelPairs, Timestamp},
    inline_mod,
};

inline_mod!(dto);

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, EnumString, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Provider {
    Google,
    Github,
}

serde_migrator! {
    #[derive(Serialize, Deserialize)]
    pub Account
    "v1": AccountV1;
}

impl Migratable for Account {
    type LatestVersion = LatestAccount;
    fn migrate(self) -> (Self::LatestVersion, bool) {
        Account::migrate(self)
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AccountV1 {
    pub email: String,
    pub user_name: Option<String>,
    pub password: Option<String>,
    pub user_verified: bool,

    pub providers: HashMap<Provider, AccountV1Provider>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountV1Provider {
    pub provider_id: String,
}

serde_migrator! {
    #[derive(Serialize, Deserialize)]
    pub User
    "v1": UserV1;
}

impl Migratable for User {
    type LatestVersion = LatestUser;
    fn migrate(self) -> (Self::LatestVersion, bool) {
        User::migrate(self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserV1 {
    pub display_name: String,
    pub name: Option<String>,
    pub middle_name: Option<String>,
    pub family_name: Option<String>,
    pub description: Option<String>,
    pub profile_bucket_id: Option<String>,
    pub formatier_verified: bool,
}

serde_migrator! {
    #[derive(Serialize, Deserialize)]
    pub Badge
    "v1": BadgeV1;
}

#[derive(Serialize, Deserialize)]
pub struct BadgeV1 {
    pub name: String,
    pub description: Option<String>,
    pub category: Category,
}

#[derive(Serialize, Deserialize)]
pub enum Category {
    Achievement,
    Event,
    Challenge,
    Competition,
    Reward,
    FormatierVerified,
}

#[derive(Serialize, Deserialize)]
pub struct InitializationTokenClaims {
    #[serde(rename = "sub")]
    pub account_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct AccessTokenClaims {
    #[serde(rename = "jti")]
    pub session_id: String,
    #[serde(rename = "sub")]
    pub account_id: String,

    pub email: String,
    pub user_id: String,

    pub scope: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    #[serde(rename = "jti")]
    pub session_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenClaims<T> {
    #[serde(rename = "iss")]
    pub issuer: Issuer,
    #[serde(rename = "aud")]
    pub audience: Vec<Audience>,

    #[serde(rename = "exp")]
    pub expiration_time: Timestamp<ChronoTime>,
    #[serde(rename = "iat")]
    pub issue_at: Timestamp<ChronoTime>,

    pub claims: T,
}

#[derive(Serialize, Deserialize, Display)]
pub enum Audience {
    FormatierAPI,
    FormatierUser,
}

#[derive(Serialize, Deserialize, Display)]
pub enum Issuer {
    #[strum(serialize = "formatier")]
    Formatier,
}

serde_migrator! {
    #[derive(Serialize, Deserialize)]
    pub Session
    "v1": SessionV1;
}

impl Migratable for Session {
    type LatestVersion = LatestSession;
    fn migrate(self) -> (Self::LatestVersion, bool) {
        Session::migrate(self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SessionV1 {
    pub os: String,
    pub os_version: String,
    pub browser: String,
    pub browser_version: String,
    pub vendor: String,
}

impl From<woothee::parser::WootheeResult<'_>> for SessionV1 {
    fn from(value: woothee::parser::WootheeResult) -> Self {
        Self {
            os: value.os.into(),
            os_version: value.os_version.into(),
            browser: value.browser_type.into(),
            browser_version: value.version.into(),
            vendor: value.vendor.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AccountMetadata {
    pub create_at: Timestamp<BsonTime>,
    pub update_at: Timestamp<BsonTime>,

    pub user_id: Option<ObjectId>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserMetadata {
    pub create_at: Timestamp<BsonTime>,
    pub update_at: Timestamp<BsonTime>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    pub create_at: Timestamp<BsonTime>,
    pub update_at: Timestamp<BsonTime>,

    pub account_id: ObjectId,
}

pub type AccountModel<S> = Model<Account, AccountMetadata, S>;
pub type UserModel<S> = Model<User, UserMetadata, S>;
pub type SessionModel<S> = Model<Session, SessionMetadata, S>;

pub type AccountModelPairs = ModelPairs<Account, AccountMetadata>;
pub type UserModelPairs = ModelPairs<User, UserMetadata>;
pub type SessionModelPairs = ModelPairs<Session, SessionMetadata>;
