use forma_core::inline_mod;
use strum::{Display, EnumString};

inline_mod!(oauth);

pub struct Token(String);

impl Token {
    pub fn new(token_string: String) -> Self {
        Self(token_string)
    }
}

#[derive(EnumString, PartialEq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
    Initialization,
}
