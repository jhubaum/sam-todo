pub mod server;

use chrono;
use std::env;
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

    println!("\n\n\n");
    let mut tasks = calendar.query_tasks(&credentials)?;
    tasks.sort_by_key(|e| e.etag.clone());
    for (i, task) in tasks.iter().enumerate() {
        println!(
            "{}. [{}] {}",
            i + 1,
            if task.completed().is_some() {
                "X"
            } else {
                " "
            },
            task.summary()
        );
    }

    // Temporary fast toggle to find out how server interaction works
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if let Ok(i) = args[1].parse::<usize>() {
            if i > 0 && i <= tasks.len() {
                // Toggle task
                if tasks[i].completed().is_some() {
                    tasks[i].set_completed(None);
                } else {
                    tasks[i].set_completed(Some(chrono::offset::Utc::now()));
                }
                println!("Toggling task {}", i);
                tasks[i].sync()?;
            }
        }
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
