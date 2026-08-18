use crate::domain::entities::{FormaError, FormaErrorApp, FormaErrorExt};

#[macro_export]
macro_rules! inline_mod {
    ($($name:ident), + $(,)?) => {
        $(
            mod $name; pub use $name::*;
        )+
    };
}

#[macro_export]
macro_rules! use_env {
    ($name:ident, $dev_env:expr, $prod_env:expr) => {
        #[cfg(not(debug_assertions))]
        pub const $name: &str = $prod_env;

        #[cfg(debug_assertions)]
        pub const $name: &str = $dev_env;
    };
}

pub fn string_to_uuid(s: impl Into<String>) -> Result<uuid::Uuid, FormaError> {
    uuid::Uuid::parse_str(s.into().as_str())
        .map_forma_err(FormaErrorApp::InvalidType, "cannot parse string to uuid")
}

pub fn init_rustls() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
}
