pub mod server;

use std::path::Path;

fn main() -> Result<(), server::ConfigError> {
    let config = server::Config::from_toml(Path::new("config.toml"))?;
    println!("Config: {:?}", config);

    Ok(())
}
