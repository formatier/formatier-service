use std::sync::LazyLock;

use forma_core::value_objects::get_env;

pub static DATABASE_URL: LazyLock<String> = LazyLock::new(|| get_env("DATABASE_URL").unwrap());

pub static CACHE_URL: LazyLock<String> = LazyLock::new(|| get_env("CACHE_URL").unwrap());
