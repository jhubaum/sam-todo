pub mod server;

use std::path::Path;

fn main() -> Result<(), server::Error> {
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
    Ok(())
}
