use async_trait::async_trait;
use forma_core::{
    domain::entities::{
        BsonTime, FormaError, FormaErrorDatabase, FormaErrorExt, FormaErrorMongoExt, Saved,
        Timestamp,
        auth_service::{
            AccountModel, AccountModelPairs, Provider, SessionMetadata, SessionModel,
            SessionModelPairs, UserModel, UserModelPairs,
        },
    },
    value_objects::{
        AUTH_DB_ACCOUNT_COLLECTION_KEY, AUTH_DB_SESSION_COLLECTION_KEY, AUTH_DB_USER_COLLECTION_KEY,
    },
};
use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{self, oid};

use crate::domain::blueprints::{
    AccountRepositoryBlueprint, RepositoryBlueprint, TokenRepositoryBlueprint,
};

pub struct MongodbAdaptor {
    db: mongodb::Database,
    account_collection: mongodb::Collection<bson::Document>,
    user_collection: mongodb::Collection<bson::Document>,
    session_collection: mongodb::Collection<bson::Document>,
}

impl MongodbAdaptor {
    pub fn new(db: mongodb::Database) -> Self {
        let account_collection = db.collection(AUTH_DB_ACCOUNT_COLLECTION_KEY);
        let user_collection = db.collection(AUTH_DB_USER_COLLECTION_KEY);
        let session_collection = db.collection(AUTH_DB_SESSION_COLLECTION_KEY);

        Self {
            db,
            account_collection,
            user_collection,
            session_collection,
        }
    }
}

fn parse_object_id(object_id: &str) -> Result<oid::ObjectId, FormaError> {
    oid::ObjectId::parse_str(object_id).map_forma_err(FormaErrorDatabase::InvalidObjectId, "")
}

#[async_trait]
impl AccountRepositoryBlueprint for MongodbAdaptor {
    async fn create_account(
        &self,
        account: AccountModelPairs,
    ) -> Result<AccountModel<Saved>, FormaError> {
        let model = AccountModel::from_pairs(account);
        let res = self
            .account_collection
            .insert_one(model.document()?)
            .await
            .map_mongo_forma_err(FormaErrorDatabase::InsertError, "cannot insert account")?;

        if let Some(id) = res.inserted_id.as_object_id() {
            Ok(model.with_id(id))
        } else {
            Err(FormaError::new(
                FormaErrorDatabase::InsertError,
                "cannot insert account",
            ))
        }
    }

    async fn add_oauth_provider(
        &self,
        account_id: &str,
        provider: Provider,
        provider_id: &str,
    ) -> Result<(), FormaError> {
        let account_id = parse_object_id(account_id)?;

        let path = format!("providers.{}.provider_id", provider);

        let res = self
            .account_collection
            .update_one(
                bson::doc! {
                    "_id": account_id,
                    "$or": [
                        { "version": "v1" },
                    ]
                },
                bson::doc! {
                    "$set": {
                        (path): provider_id.to_string()
                    }
                },
            )
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::InsertError,
                "cannot insert provider in an account",
            )?;

        if res.matched_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "account not found",
            ))
        } else {
            Ok(())
        }
    }

    async fn get_account(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountModel<Saved>>, FormaError> {
        let account_id = parse_object_id(account_id)?;
        self.account_collection
            .find_one(bson::doc! {
                "_id": account_id
            })
            .await
            .map_mongo_forma_err(FormaErrorDatabase::SelectError, "cannot find account by id")?
            .map(|doc| {
                bson::deserialize_from_document(doc).map_forma_err(
                    FormaErrorDatabase::InvalidArgument,
                    "failed to deserialize account",
                )
            })
            .transpose()
    }

    async fn get_account_by_oauth_provider(
        &self,
        provider: Provider,
        provider_id: &str,
    ) -> Result<Option<AccountModel<Saved>>, FormaError> {
        let path = format!("providers.{}.provider_id", provider);
        self.account_collection
            .find_one(bson::doc! {
                (path): provider_id
            })
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::SelectError,
                "cannot find account by provider id",
            )?
            .map(|doc| {
                bson::deserialize_from_document(doc).map_forma_err(
                    FormaErrorDatabase::InvalidArgument,
                    "failed to deserialize account",
                )
            })
            .transpose()
    }

    async fn update_account_username(
        &self,
        account_id: &str,
        username: &str,
    ) -> Result<(), FormaError> {
        let account_id = parse_object_id(account_id)?;

        let res = self
            .account_collection
            .update_one(
                bson::doc! {
                    "_id": account_id,
                    "$or": [
                        { "version": "v1" },
                    ]
                },
                bson::doc! {
                    "$set": {
                        "username": username
                    }
                },
            )
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::UpdateError,
                "cannot find account by account id",
            )?;

        if res.matched_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "account not found",
            ))
        } else {
            Ok(())
        }
    }

    async fn init_user(
        &self,
        account_id: &str,
        user: UserModelPairs,
    ) -> Result<UserModel<Saved>, FormaError> {
        let account_id = parse_object_id(account_id)?;

        let mut session = self
            .db
            .client()
            .start_session()
            .await
            .map_mongo_forma_err(FormaErrorDatabase::TransactionError, "cannot init session")?;

        session.start_transaction().await.map_mongo_forma_err(
            FormaErrorDatabase::TransactionError,
            "cannot start transaction",
        )?;

        let user_model = UserModel::from_pairs(user);

        let res = self
            .user_collection
            .insert_one(user_model.document()?)
            .session(&mut session)
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::InsertError,
                "cannot insert user to user row",
            )?;

        let user_id = res.inserted_id.as_object_id().ok_or_else(|| {
            FormaError::new(
                FormaErrorDatabase::InsertError,
                "cannot insert user to user row",
            )
        })?;

        let res = self
            .account_collection
            .update_one(
                bson::doc! {
                    "_id": account_id,
                    "$or": [
                        { "version": "v1" },
                    ]
                },
                bson::doc! {
                    "user_id": user_id
                },
            )
            .session(&mut session)
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::UpdateError,
                "cannot find account by account id",
            )?;

        if res.matched_count < 1 || res.modified_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "account not found",
            ))
        } else {
            session.commit_transaction().await.map_mongo_forma_err(
                FormaErrorDatabase::TransactionError,
                "cannot commit transaction",
            )?;
            Ok(user_model.with_id(user_id))
        }
    }

    async fn get_user(&self, user_id: &str) -> Result<UserModel<Saved>, FormaError> {
        let user_id = parse_object_id(user_id)?;

        self.user_collection
            .find_one(bson::doc! {
                "_id": user_id,
                "$or": [
                    { "version": "v1" },
                ]
            })
            .await
            .map_mongo_forma_err(FormaErrorDatabase::SelectError, "cannot find user by id")?
            .map(|doc| {
                bson::deserialize_from_document(doc).map_forma_err(
                    FormaErrorDatabase::InvalidArgument,
                    "failed to deserialize account",
                )
            })
            .ok_or(FormaError::new(
                FormaErrorDatabase::NotFound,
                "user not found",
            ))?
    }

    async fn hard_delete_oauth_provider(
        &self,
        account_id: &str,
        provider: Provider,
    ) -> Result<(), FormaError> {
        let account_id = parse_object_id(account_id)?;

        let path = format!("providers.{}", provider);

        let res = self
            .account_collection
            .update_one(
                bson::doc! {
                    "_id": account_id,
                    "$or": [
                        { "version": "v1" },
                    ],
                },
                bson::doc! {
                    "$unset": {
                        (path): ""
                    }
                },
            )
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::DeleteError,
                format!("cannot delete {} oauth provider", provider).as_str(),
            )?;

        if res.matched_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "account not found",
            ))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl TokenRepositoryBlueprint for MongodbAdaptor {
    async fn create_session_metadata(
        &self,
        account_id: &str,
    ) -> Result<SessionMetadata, FormaError> {
        let account_id = parse_object_id(account_id)?;
        let count = self
            .account_collection
            .count_documents(bson::doc! {
                "_id": account_id
            })
            .await
            .map_mongo_forma_err(FormaErrorDatabase::InsertError, "cannot insert session")?;

        if count < 1 {
            return Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "account not found",
            ));
        }

        Ok(SessionMetadata {
            account_id,
            ..Default::default()
        })
    }

    async fn create_session(
        &self,
        session: SessionModelPairs,
    ) -> Result<SessionModel<Saved>, FormaError> {
        let session_model = SessionModel::from_pairs(session);

        let res = self
            .session_collection
            .insert_one(session_model.document()?)
            .await
            .map_mongo_forma_err(FormaErrorDatabase::InsertError, "cannot insert session")?;

        if let Some(id) = res.inserted_id.as_object_id() {
            Ok(session_model.with_id(id))
        } else {
            Err(FormaError::new(
                FormaErrorDatabase::InsertError,
                "cannot insert account",
            ))
        }
    }

    async fn is_session_available(&self, session_id: &str) -> Result<bool, FormaError> {
        let session_id = parse_object_id(session_id)?;

        let res = self
            .session_collection
            .count_documents(bson::doc! {
                "_id": session_id
            })
            .await
            .map_mongo_forma_err(FormaErrorDatabase::SelectError, "cannot find session by id")?;

        Ok(res > 0)
    }

    async fn refresh_session(&self, session_id: &str) -> Result<(), FormaError> {
        let session_id = parse_object_id(session_id)?;

        let now: bson::DateTime = Timestamp::<BsonTime>::default().into();

        let res = self
            .session_collection
            .update_one(
                bson::doc! {
                    "_id": session_id
                },
                bson::doc! {
                    "$set": {
                        "create_at": now,
                    }
                },
            )
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::UpdateError,
                "cannot find account by account id",
            )?;

        if res.matched_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "session not found",
            ))
        } else {
            Ok(())
        }
    }

    async fn get_sessions(
        &self,
        account_id: &str,
    ) -> Result<Box<dyn Iterator<Item = SessionModel<Saved>> + Send>, FormaError> {
        let account_id = parse_object_id(account_id)?;

        let res = self
            .session_collection
            .find(bson::doc! {
                "account_id": account_id
            })
            .await
            .map_mongo_forma_err(
                FormaErrorDatabase::SelectError,
                "cannot find session by account id",
            )?;

        let sessions: Vec<SessionModel<Saved>> = res
            .map(|res| {
                res.map(|doc| {
                    bson::deserialize_from_document(doc).map_forma_err(
                        FormaErrorDatabase::InvalidArgument,
                        "failed to deserialize account",
                    )
                })
                .map_mongo_forma_err(
                    FormaErrorDatabase::SelectError,
                    "cannot find session by account id",
                )
                .flatten()
            })
            .try_collect()
            .await?;

        Ok(Box::new(sessions.into_iter()))
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), FormaError> {
        let session_id = parse_object_id(session_id)?;

        let res = self
            .session_collection
            .delete_one(bson::doc! {
                "id": session_id
            })
            .await
            .map_mongo_forma_err(FormaErrorDatabase::DeleteError, "cannot delete session")?;

        if res.deleted_count < 1 {
            Err(FormaError::new(
                FormaErrorDatabase::NotFound,
                "session not found",
            ))
        } else {
            Ok(())
        }
    }
}

impl RepositoryBlueprint for MongodbAdaptor {}
