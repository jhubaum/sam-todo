use std::fs;
use std::io::Error as IOError;
use std::path::Path;
use toml;
use url::{ParseError as UrlError, Url};

pub mod caldav;
use caldav::{Calendar, Credentials, Error as CaldavError};

#[derive(Debug)]
pub enum Error {
    File(IOError),
    Url(UrlError),
    Parsing(toml::de::Error),
    MissingSection(&'static str),
    MissingField {
        field: &'static str,
        section: &'static str,
    },
    Caldav(CaldavError),
    InvalidConfigValue(&'static str),
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Self {
        Self::File(err)
    }
}

impl From<UrlError> for Error {
    fn from(err: UrlError) -> Self {
        Self::Url(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::Parsing(err)
    }
}

impl From<caldav::Error> for Error {
    fn from(err: caldav::Error) -> Self {
        Self::Caldav(err)
    }
}

#[derive(Debug)]
pub struct Config {
    pub url: Url,
    pub credentials: Credentials,
    pub calendar_name: String,
}

impl Config {
    pub fn from_toml(file: &Path) -> Result<Self, Error> {
        fn get_section<'a>(
            table: &'a toml::Table,
            section: &'static str,
        ) -> Result<&'a toml::Table, Error> {
            match table.get(section).ok_or(Error::MissingSection(section))? {
                toml::Value::Table(section) => Ok(section),
                _ => Err(Error::InvalidConfigValue("Invalid section")),
            }
        }

        fn get_string<'a>(
            table: &'a toml::Table,
            field: &'static str,
            section: &'static str,
        ) -> Result<&'a str, Error> {
            let field = table
                .get(field)
                .ok_or(Error::MissingField { field, section })?;
            match field {
                toml::Value::String(s) => Ok(s),
                _ => Err(Error::InvalidConfigValue("Invalid field")),
            }
        }

        let table = fs::read_to_string(file)?.parse::<toml::Table>()?;
        let server = get_section(&table, "Server")?;
        let calendar = get_section(&table, "Calendar")?;

        let user = get_string(&server, "user", "Server")?.to_owned();
        let password = get_string(&server, "password", "Server")?.to_owned();
        Ok(Config {
            url: Url::parse(get_string(&server, "url", "Server")?)?,
            credentials: Credentials { user, password },
            calendar_name: get_string(&calendar, "name", "Calendar")?.to_owned(),
        })
    }
}
