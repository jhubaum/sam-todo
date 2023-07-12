use chrono::{DateTime, Local};
use minicaldav::Event;

pub enum Error {}

pub struct Task {
    pub completed: Option<DateTime<Local>>,
    pub start_date: Option<DateTime<Local>>,
    pub due_date: Option<DateTime<Local>>,
    pub created: DateTime<Local>,
    pub last_modified: DateTime<Local>,
    pub summary: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl Task {
    fn apply_changes(&self, event: &mut Event) {}

    fn from_event(event: &Event) -> Result<Self, Error> {
        unimplemented!();
    }

    fn to_event(self) -> Event {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_completed_task() {}

    #[test]
    fn parse_task_with_due_date() {}
}
