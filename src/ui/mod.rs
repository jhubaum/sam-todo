use iced::executor;
use iced::{Application, Command, Element, Theme};
use std::path::Path;

use crate::server::caldav::{Calendar, Credentials};
use crate::server::{Config, Error as ServerError};

pub struct LoadedApp {
    credentials: Credentials,
    calendar: Calendar,
}

impl LoadedApp {
    fn view(&self) -> Element<Message> {
        "Not yet implemented".into()
    }
}

pub enum App {
    Loading,
    Ready(LoadedApp),
    LoadingFailed(ServerError),
}

async fn initialize_calendar() -> Result<(Credentials, Calendar), ServerError> {
    let config = Config::from_toml(Path::new("config.toml"))?;

    let calendars = Calendar::query_url(&config.url, &config.credentials)?;
    let calendar = calendars
        .into_iter()
        .find(|c| c.name == config.calendar_name)
        .ok_or(ServerError::InvalidConfigValue("Unknown Calendar entry"))?;

    Ok((config.credentials, calendar))
}

#[derive(Debug)]
pub enum Message {
    InitializeCalendar(Result<(Credentials, Calendar), ServerError>),
    SyncCalendars,
    SendTaskCollectionUpdate(usize),
}

impl Application for App {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        // TODO: Use a command to initialize the calendar instead

        (
            App::Loading,
            Command::perform(initialize_calendar(), Message::InitializeCalendar),
        )
    }

    fn title(&self) -> String {
        String::from("Sam – A simple tasklist")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::InitializeCalendar(res) => {
                *self = match res {
                    Ok((credentials, calendar)) => App::Ready(LoadedApp {
                        credentials,
                        calendar,
                    }),
                    Err(err) => App::LoadingFailed(err),
                }
            }
            _ => {}
        };
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        match self {
            App::Loading => "Loading app".into(),
            App::LoadingFailed(err) => {
                iced::widget::Text::new(format!("Something went wrong: {:?}", err)).into()
            }
            App::Ready(app) => app.view(),
        }
    }
}
