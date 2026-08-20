use std::sync::LazyLock;

use forma_core::{use_env, value_objects::get_env};

pub const GOOGLE_OAUTH_TOKEN_EXCHANGE_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_OAUTH_USER_INFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

pub static GOOGLE_OAUTH_CLIENT_ID: LazyLock<String> =
    LazyLock::new(|| get_env("GOOGLE_OAUTH_CLIENT_ID").unwrap());
pub static GOOGLE_OAUTH_CLIENT_SECRET: LazyLock<String> =
    LazyLock::new(|| get_env("GOOGLE_OAUTH_CLIENT_SECRET").unwrap());

use_env!(
    GOOGLE_OAUTH_REDIRECT_URI,
    "http://localhost:3000/auth/callback/google",
    "https://www.formatier.com/auth/callback/google"
);
