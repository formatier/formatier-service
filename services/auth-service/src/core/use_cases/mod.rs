use std::{collections::HashMap, sync::Arc};

use forma_core::domain::entities::{
    FormaError, FormaErrorApp, FormaErrorAuth, FormaErrorDatabase,
    auth_service::{
        Account, AccountMetadata, AccountModel, AccountModelPairs, AccountV1, AccountV1Provider,
        Provider, User, UserModel,
    },
};

use crate::{
    core::services::TokenService,
    domain::{
        blueprints::{OAuthBlueprint, RepositoryBlueprint, TokenCacheBlueprint},
        entities::Token,
    },
};

pub struct UseCase {
    google_oauth: Arc<dyn OAuthBlueprint + Send + Sync>,
    github_oauth: Arc<dyn OAuthBlueprint + Send + Sync>,
    repository: Arc<dyn RepositoryBlueprint + Send + Sync>,
    token_service: TokenService,
}

impl UseCase {
    pub fn new(
        google_oauth: Arc<dyn OAuthBlueprint + Send + Sync>,
        github_oauth: Arc<dyn OAuthBlueprint + Send + Sync>,
        repository: Arc<dyn RepositoryBlueprint + Send + Sync>,
        token_cache: Arc<dyn TokenCacheBlueprint + Send + Sync>,
    ) -> Self {
        Self {
            google_oauth: google_oauth.clone(),
            github_oauth: github_oauth.clone(),
            repository: repository.clone(),

            token_service: TokenService::new(repository.clone(), token_cache.clone()),
        }
    }

    pub async fn signin_by_oauth_provider(
        &self,
        provider: Provider,
        code: &str,
        user_agent: &str,
    ) -> Result<(String, Option<String>), FormaError> {
        let oauth_user_info = match provider {
            Provider::Google => {
                let token = self.google_oauth.exchange_code(code).await?;
                self.google_oauth.get_authorized_user(&token).await?
            }
            Provider::Github => {
                let token = self.github_oauth.exchange_code(code).await?;
                self.github_oauth.get_authorized_user(&token).await?
            }
        };

        let account = self
            .repository
            .get_account_by_oauth_provider(provider, &oauth_user_info.provider_id)
            .await?;

        let account = match account {
            Some(account) => account,
            None => {
                self.repository
                    .create_account(AccountModelPairs {
                        data: AccountV1 {
                            email: oauth_user_info.email,
                            providers: HashMap::from([(
                                provider,
                                AccountV1Provider {
                                    provider_id: oauth_user_info.provider_id,
                                },
                            )]),
                            ..Default::default()
                        }
                        .into(),
                        metadata: AccountMetadata::default(),
                    })
                    .await?
            }
        };

        let account_id = account.id().to_hex();
        let (account_data, _) = account.data.migrate();
        let account_metadata = account.metadata;

        if let Some(user_id) = account_metadata.user_id {
            let user_agent =
                woothee::parser::Parser::default()
                    .parse(user_agent)
                    .ok_or(FormaError::new(
                        FormaErrorAuth::InvalidUserAgent,
                        "cannot parse user-agent",
                    ))?;
            let (access_token, refresh_token) = self
                .token_service
                .issue_tokens_and_create_session(
                    &account_id,
                    &user_id.to_hex(),
                    &account_data.email,
                    &["all"],
                    user_agent.into(),
                )
                .await?;

            Ok((access_token, Some(refresh_token)))
        } else {
            let initialization_token =
                self.token_service.issue_initialization_token(&account_id)?;
            Ok((initialization_token, None))
        }
    }
}
