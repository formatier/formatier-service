use serde::Deserialize;

#[derive(Deserialize)]
pub struct GoogleOAuthTokenExchange {
    pub access_token: String,
    pub expires_in: u64,
}

impl Into<OAuthToken> for GoogleOAuthTokenExchange {
    fn into(self) -> OAuthToken {
        OAuthToken {
            access_token: self.access_token,
            expires_in: self.expires_in,
        }
    }
}

#[derive(Deserialize)]
pub struct GoogleOpenIDUserInfo {
    pub sub: String,
    pub given_name: String,
    pub family_name: Option<String>,
    pub email: String,
    pub picture: Option<String>,
}

impl Into<OAuthUserInfo> for GoogleOpenIDUserInfo {
    fn into(self) -> OAuthUserInfo {
        OAuthUserInfo {
            provider_id: self.sub,
            email: self.email,
            username: None,
            avatar_url: self.picture,
            display_name: Some(format!(
                "{} {}",
                self.given_name,
                self.family_name.unwrap_or("".into())
            ).trim_matches(' ').into()),
            bio: None,
            website: None,
        }
    }
}

#[derive(Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub expires_in: u64,
}

pub struct OAuthUserInfo {
    pub provider_id: String,
    pub email: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,

    pub display_name: Option<String>,

    pub bio: Option<String>,
    pub website: Option<String>,
}
