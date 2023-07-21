use iced::executor;
use iced::widget::{
    self, button, checkbox, column, container, row, scrollable, text, text_input, Text,
};
use iced::{Application, Command, Element, Length, Theme};
use std::path::Path;

use crate::server::caldav::{Calendar, Credentials, TaskRef, UtcDateTime};
use crate::server::{Config, Error as ServerError};

#[derive(Debug)]
pub struct LoadedAppData {
    credentials: Credentials,
    calendar: Calendar,
}

#[derive(Debug)]
pub struct Tasks {
    tasks: Vec<Task>,
}

impl LoadedAppData {
    fn tasks(&mut self) -> Tasks {
        Tasks {
            tasks: self
                .calendar
                .tasks()
                .into_iter()
                .map(|t| Task::from_ref(t))
                .collect(),
        }
    }
}

impl Tasks {
    fn update_task(&mut self, task: TaskRef) {
        if let Some(i) = self
            .tasks
            .iter()
            .enumerate()
            .find(|t| {
                t.1.collection_index == task.collection_index && t.1.task_index == task.task_index
            })
            .map(|t| t.0)
        {
            self.tasks[i] = Task::from_ref(task)
        } else {
            // TODO: Throw error?
        }
    }
    fn view(&self) -> Element<Message> {
        column(
            self.tasks
                .iter()
                .map(|t| {
                    t.view().map(|msg| Message::Task {
                        collection: t.collection_index,
                        task: t.task_index,
                        msg,
                    })
                })
                .collect(),
        )
        .spacing(10)
        .into()
    }
}

#[derive(Debug)]
pub enum TaskMessage {
    Completed(bool),
}

impl TaskMessage {
    fn apply(self, task: &mut TaskRef) {
        match self {
            Self::Completed(true) => task.set_done(chrono::offset::Utc::now()),
            Self::Completed(false) => task.set_undone(),
        }
    }
}

#[derive(Debug)]
struct Task {
    collection_index: usize,
    task_index: usize,
    completed: Option<UtcDateTime>,
    summary: String,
}

impl Task {
    // TODO: Throw error when task is missing a value
    fn from_ref(task: TaskRef) -> Self {
        Self {
            collection_index: task.collection_index,
            task_index: task.task_index,
            completed: task.done(),
            summary: task.summary().clone(),
        }
    }
    fn view<'a>(&self) -> Element<'a, TaskMessage> {
        checkbox(
            &self.summary,
            self.completed.is_some(),
            TaskMessage::Completed,
        )
        .width(Length::Fill)
        .into()
    }
}

#[derive(Debug)]
pub enum Message {
    InitializeCalendar(Result<(Credentials, Calendar), ServerError>),
    SyncCalendars,
    SendTaskCollectionUpdate(usize),
    Task {
        collection: usize,
        task: usize,
        msg: TaskMessage,
    },
}

#[derive(Debug)]
pub enum App {
    Loading,
    Ready(LoadedAppData, Tasks),
    LoadingFailed(ServerError),
}

async fn initialize_calendar() -> Result<(Credentials, Calendar), ServerError> {
    let config = Config::from_toml(Path::new("config.toml"))?;

    let calendars = Calendar::query_url(&config.url, &config.credentials)?;
    let mut calendar = calendars
        .into_iter()
        .find(|c| c.name == config.calendar_name)
        .ok_or(ServerError::InvalidConfigValue("Unknown Calendar entry"))?;

    calendar.query_data(&config.credentials)?;

    Ok((config.credentials, calendar))
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
                    Ok((credentials, calendar)) => {
                        let mut data = LoadedAppData {
                            credentials,
                            calendar,
                        };
                        let tasks = data.tasks();
                        App::Ready(data, tasks)
                    }
                    Err(err) => App::LoadingFailed(err),
                }
            }
            Message::Task {
                collection,
                task,
                msg,
            } => self.update_task(collection, task, msg),
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
            App::Ready(_, tasks) => tasks.view(),
        }
    }
}

impl App {
    fn update_task(&mut self, collection: usize, task: usize, msg: TaskMessage) {
        if let App::Ready(data, tasks) = self {
            {
                let mut task_ref = data.calendar.task(collection, task).unwrap();
                msg.apply(&mut task_ref);
                tasks.update_task(task_ref);
            }
            // TODO: Replace this by a periodical sync
            data.calendar.send_collection_update(collection, &data.credentials).unwrap();
        } else {
            panic!(
                "Received TaskMessage on unready app. This points towards an implementation error"
            );
        }
    }
}
