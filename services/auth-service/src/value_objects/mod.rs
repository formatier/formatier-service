use std::{env, sync::LazyLock};

use forma_core::{inline_mod, value_objects::get_env};

inline_mod!(oauth);
inline_mod!(database);

pub static TOKEN_SECRET: LazyLock<String> = LazyLock::new(|| get_env("TOKEN_SECRET"));
