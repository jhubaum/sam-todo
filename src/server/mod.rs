use minicaldav::{Calendar, Credentials, Error as CaldavError};
use std::fs;
use std::io::Error as IOError;
use std::path::Path;
use toml;
use ureq::Agent;
use url::{ParseError as UrlError, Url};

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

impl From<minicaldav::Error> for Error {
    fn from(err: minicaldav::Error) -> Self {
        Self::Caldav(err)
    }
}

struct Config {
    url: Url,
    credentials: Credentials,
    calendar_name: String,
}

pub struct Server {
    config: Config,
    calendar: Calendar,
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
            credentials: minicaldav::Credentials::Basic(user, password),
            calendar_name: get_string(&calendar, "name", "Calendar")?.to_owned(),
        })
    }
}

impl Server {
    pub fn from_toml(file: &Path) -> Result<Self, Error> {
        let config = Config::from_toml(file)?;
        let calendar = minicaldav::get_calendars(Agent::new(), &config.credentials, &config.url)?
            .iter()
            .find(|c| c.name() == &config.calendar_name)
            .ok_or(Error::InvalidConfigValue("Invalid calendar name"))?
            .to_owned();

        Ok(Self { config, calendar })
    }

    pub fn query_tasks(&self) -> Result<Vec<minicaldav::Event>, Error> {
        // TODO: What to do with errors?
        let (events, _errors) =
            minicaldav::get_todos(Agent::new(), &self.config.credentials, &self.calendar)?;
        Ok(events)
    }
}
