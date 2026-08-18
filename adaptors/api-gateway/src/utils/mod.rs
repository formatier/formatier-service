use std::{fs, io::Read};

use crate::domain::entities::Config;

pub fn load_config() -> Config {
    let mut file = fs::File::open("./deployment/config/route.yml").unwrap();
    let mut raw_yml = String::new();

    file.read_to_string(&mut raw_yml).unwrap();

    let config: Config = serde_yaml_ng::from_str(&raw_yml).unwrap();

    config
}
