use std::fs;
use std::io::Error as IOError;
use std::path::Path;
use toml;
use url::{ParseError as UrlError, Url};

#[derive(Debug)]
pub enum ConfigError {
    File(IOError),
    Url(UrlError),
    Parsing(toml::de::Error),
    MissingSection(&'static str),
    MissingField {
        field: &'static str,
        section: &'static str,
    },
    InvalidSyntax,
}

impl From<IOError> for ConfigError {
    fn from(err: IOError) -> Self {
        Self::File(err)
    }
}

impl From<UrlError> for ConfigError {
    fn from(err: UrlError) -> Self {
        Self::Url(err)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parsing(err)
    }
}

#[derive(Debug)]
pub struct Config {
    pub url: Url,
    pub user: String,
    pub password: String,
}

impl Config {
    pub fn from_toml(file: &Path) -> Result<Self, ConfigError> {
        let table = fs::read_to_string(file)?.parse::<toml::Table>()?;
        let server = table
            .get("Server")
            .ok_or(ConfigError::MissingSection("Server"))?;

        if let toml::Value::Table(server) = server {
            fn get_string<'a>(
                table: &'a toml::Table,
                field: &'static str,
            ) -> Result<&'a str, ConfigError> {
                let field = table.get(field).ok_or(ConfigError::MissingField {
                    field,
                    section: "Server",
                })?;
                match field {
                    toml::Value::String(s) => Ok(s),
                    _ => Err(ConfigError::InvalidSyntax),
                }
            }
            return Ok(Config {
                url: Url::parse(get_string(&server, "url")?)?,
                user: get_string(&server, "user")?.to_owned(),
                password: get_string(&server, "password")?.to_owned(),
            });
        }

        // TODO: Improve usefulness of this error
        Err(ConfigError::InvalidSyntax)
    }
}
