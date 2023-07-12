pub mod server;

use std::path::Path;

use minicaldav::Credentials;
fn get_credentials(credentials: &Credentials) -> server::caldav::Credentials {
    if let Credentials::Basic(user, password) = credentials {
        return server::caldav::Credentials {
            user: user.to_owned(),
            password: password.to_owned(),
        };
    }
    panic!();
}

fn main() -> Result<(), server::Error> {
    let config = server::Config::from_toml(Path::new("config.toml"))?;

    let credentials = get_credentials(&config.credentials);
    let calendars = server::caldav::get_calendars(&config.url, &credentials)?;
    for calendar in calendars.iter() {
        println!("Calendar: {:?}", calendar);
    }
    let calendar = calendars
        .iter()
        .find(|c| c.name == config.calendar_name)
        .unwrap();

    for task in calendar.query_tasks(&credentials)? {
        println!("Task: {:?}", task);
    }

    /*
        let server = server::Server::from_toml(Path::new("config.toml"))?;
        for task in server.query_tasks()? {
            println!(
                "Name: {} ({})",
                task.ical().name,
                task.etag().unwrap_or(&String::from(""))
            );
            println!("Properties:");
            for prop in task.ical().properties.iter() {
                println!("\t{:?}", prop);
            }
            println!("Children:");
            for child in task.ical().children.iter() {
                println!("\t{:?}", child);
            }
            println!()
        }
    */
    Ok(())
}
