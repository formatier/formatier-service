use forma_core::inline_mod;
use strum::{Display, EnumString};

inline_mod!(oauth);

#[derive(EnumString, PartialEq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
    Initialization,
}
