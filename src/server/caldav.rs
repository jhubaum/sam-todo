use base64;
use std::collections::HashMap;
use ureq::Agent;
use url::{ParseError as UrlError, Url};
use chrono::ParseError as DateTimeError;

#[derive(Debug)]
// TODO: Implement std error trait
pub enum Error {
    // TODO: Make message error more meaningful.
    Message(String),
    Url(url::ParseError),
    DateTime(DateTimeError),
    Xml(xmltree::ParseError),
}

impl From<UrlError> for Error {
    fn from(err: UrlError) -> Error {
        Error::Url(err)
    }
}

impl From<DateTimeError> for Error {
    fn from(err: DateTimeError) -> Error {
        Error::DateTime(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO: Only use Bearer token for authentification?
pub struct Credentials {
    pub user: String,
    pub password: String,
}

impl Credentials {
    fn encode(&self) -> String {
        format!(
            "Basic {}",
            base64::encode(format!("{}:{}", self.user, self.password))
        )
    }
}

mod ical {
    use super::{Error, Result};
    use chrono::{DateTime, Utc, NaiveDateTime};
    type UtcDateTime = DateTime<Utc>;

    #[derive(Debug, Default)]
    pub struct Data {
        pub tasks: Vec<Task>,
    }

    #[derive(Debug, Default, Clone)]
    pub struct Task {
        pub summary: String,
        pub description: Option<String>,
        pub completed: Option<UtcDateTime>,
    }

    #[derive(Default)]
    struct TaskBuilder {
        task: Option<Task>,
        // The required fields for the task
        summary: Option<String>,
    }

    fn parse_utc_timestamp(ts: &str) -> Result<UtcDateTime> {
        Ok(NaiveDateTime::parse_from_str(ts, "%Y%m%dT%H%M%SZ")?.and_utc())
    }

    impl TaskBuilder {
        fn consume_field(&mut self, key: &str, value: &str) -> Result<()> {
            match key {
                "SUMMARY" => self.summary(value),
                "COMPLETED" => self.data()?.completed = Some(parse_utc_timestamp(value)?),
                _ => println!("Unhandled task line: {}:{}", key, value)
            }
            Ok(())
        }
        fn summary(&mut self, summary: &str) {
            self.summary = Some(summary.to_owned());
        }

        fn start_new(&mut self) -> Result<()> {
            match self.task {
                None => {
                    self.task = Some(Task::default());
                    Ok(())
                },
                Some(_) => Err(Error::Message("Tried to create new task while another one is being build".to_owned()))
            }
        }

        fn data(&mut self) -> Result<&mut Task> {
            self.task.as_mut().ok_or(Error::Message("Tried to modify task from empty TaskBuilder".to_owned()))
        }

        fn build(&mut self) -> Result<Task> {
            let mut task = self.task.take().ok_or(Error::Message("Tried to create task from empty TaskBuilder".to_owned()))?;
            self.task = None;

            task.summary = self
                .summary
                .clone()
                .ok_or(Error::Message("Missing Summary in TaskBuilder".to_owned()))?;
            Ok(task)
        }
    }

    #[derive(PartialEq)]
    enum State {
        Done,
        Calendar,
        TimeZone,
        TimeZoneStandard,
        TimeZoneDaylight,
        Task,
    }

    impl TryFrom<std::str::Lines<'_>> for Data {
        type Error = Error;

        fn try_from(mut lines: std::str::Lines) -> Result<Self> {
            // TODO: Wrap into KeyValue (or Line) struct
            fn split<'a>(line: &'a str) -> Result<(&'a str, &'a str)> {
                let delim = line.find(":").ok_or(Error::Message(format!(
                    "Invalid ICalFormat line: '{}'",
                    line
                )))?;
                // TODO: Split key at ';' for additional arguments
                Ok((&line[..delim], &line[delim + 1..]))
            }

            fn expect(line: (&str, &str), key: &str, value: &str) -> Result<()> {
                Ok(())
            }

            fn expect_key(line: (&str, &str), key: &str) -> Result<()> {
                Ok(())
            }

            // Begin
            // TODO: Rewrite as Line::from(lines.next())?.expect(...)?, get rid of unwrap
            expect(split(lines.next().unwrap())?, "BEGIN", "VCALENDAR")?;
            expect(split(lines.next().unwrap())?, "VERSION", "2.0")?;
            expect_key(split(lines.next().unwrap())?, "PRODID")?;

            let mut state = State::Calendar;

            let mut data = Data::default();
            let mut taskbuilder = TaskBuilder::default();

            for line in lines {
                // TODO: Add debug logging for the line here
                //println!("Line: {}", line);
                match (split(line)?, &state) {
                    (_, State::Done) => {
                        return Err(Error::Message(String::from(
                            "Unexpected line after \"END:VCALENDAR\"",
                        )))
                    }
                    (("BEGIN", "VTODO"), State::Calendar) => {
                        state = State::Task;
                        taskbuilder.start_new()?;
                    }
                    (("BEGIN", tag), _) => println!("Unhandled begin: {}", tag),
                    (("END", "VTODO"), State::Task) => {
                        state = State::Calendar;
                        data.tasks.push(
                            taskbuilder.build()?
                        );
                    }
                    (("END", "VCALENDAR"), State::Calendar) => state = State::Done,
                    (("END", tag), _) => println!("Unhandled end: {}", tag),
                    ((key, value), State::Task) => {
                        taskbuilder.consume_field(key, value)?;
                    }
                    //((key, value), _) => println!("Unhandled iCal line: {}:{}", key, value),
                    _ => {}
                };
            }

            if state != State::Done {
                return Err(Error::Message("Unexpected end of iCal data".to_owned()));
            }

            Ok(data)
        }
    }
}

#[derive(Debug)]
pub struct Calendar {
    pub url: Url,
    pub etag: String,
    pub name: String,
}

// TODO: consolidate with ical::Task, pass url and etag as default values to TaskBuilder
#[derive(Debug)]
pub struct Task {
    pub url: Url,
    pub etag: Option<String>,
    pub data: ical::Task
}

impl Task {
    fn from_data(data: &XMLData, calendar_url: &Url) -> Result<Vec<Self>> {
        let mut tasks = Vec::new();
        let url = calendar_url.join(&data.href)?;
        let etag = data.etag();
        for task in data.ical().unwrap_or(&ical::Data::default()).tasks.iter() {
            tasks.push(Task { url: url.clone(), etag: etag.clone(), data: task.clone() } );
        }
        Ok(tasks)
    }
}

enum CalProp {
    Etag(String),
    IsCalendar(bool),
    SupportedComponents(Vec<String>),
    Name(String),
}

impl CalProp {
    fn from_xml(xml: &xmltree::Element) -> Result<Self> {
        match xml.name.as_str() {
            // TODO: Throw error instead of using a default value
            // TODO: Can I build a base parser for the multistatus stuff for generalisation
            // between calendar and task?
            "getetag" => Ok(Self::Etag(xml_to_text(xml, ""))),
            "displayname" => Ok(Self::Name(xml_to_text(xml, ""))),
            "resourcetype" => Ok(Self::IsCalendar(
                xml.children
                    .iter()
                    .filter_map(|c| c.as_element())
                    .find(|c| c.name == "calendar")
                    .is_some(),
            )),
            "supported-calendar-component-set" => Ok(Self::SupportedComponents(
                xml.children
                    .iter()
                    .filter_map(|c| c.as_element())
                    // TODO: Throw error when attribute isn't found
                    .map(|c| c.attributes.get("name").unwrap().clone())
                    .collect(),
            )),
            val => Err(Error::Message(format!(
                "Unable to parse XML response: Unknown property {}",
                val
            ))),
        }
    }
}

#[derive(Debug)]
enum Property {
    Etag(String),
    // TODO: Store lines iterator and make the entire XMLData work based on references
    // over the xml data (-> no string copy for xml_to_text)
    CalendarData(ical::Data),
    // A fallback handler for data I'm not reading (yet)
    Unknown(String),
}

// TODO: Better name, more webdav focused. MultiStatusElement?
#[derive(Debug)]
struct XMLData {
    href: String,
    status: Option<String>,
    // TODO: List instead of hashmap or (static) string refs?
    // TODO: Better expose unhandled properties for debugging
    properties: HashMap<String, Property>,
}

impl XMLData {
    fn etag(&self) -> Option<String> {
        self.properties.get("getetag").map(|v| match v {
            Property::Etag(t) => t.clone(),
            // TODO: Rename XMLData in error message
            _ => panic!("Logic error while constructing XMLData"),
        })
    }

    fn ical(&self) -> Option<&ical::Data> {
        self.properties.get("calendar-data").map(|v| match v {
            Property::CalendarData(d) => d,
            // TODO: Rename XMLData in error message
            _ => panic!("Logic error while constructing XMLData"),
        })
    }

    fn parse(xml: &xmltree::Element) -> Result<Self> {
        // TODO: Add error checking if it's an response element?
        let propstat = xml
            .get_child("propstat")
            .ok_or(Error::Message(String::from("Failed to parse XML response")))?;
        let status = propstat.get_child("status").map(|s| xml_to_text(s, ""));

        // TODO: Use a custom type instead of xmltree::Element here, e.g. an enum
        let props = propstat
            .get_child("prop")
            .ok_or(Error::Message(String::from(
                "Failed to parse XML response: Missing Props field",
            )))?;

        let href = xml
            .get_child("href")
            .map(|e| xml_to_text(e, ""))
            .ok_or(Error::Message("Element doesn't have an url".to_owned()))?;

        let mut properties = HashMap::new();
        for prop in props.children.iter().filter_map(|p| p.as_element()) {
            properties.insert(
                prop.name.clone(),
                match prop.name.as_str() {
                    "getetag" => Property::Etag(xml_to_text(prop, "")),
                    "calendar-data" => {
                        Property::CalendarData(xml_to_text(prop, "").lines().try_into()?)
                    }
                    name => Property::Unknown(name.to_owned()),
                },
            );
        }

        Ok(Self {
            href,
            status,
            properties,
        })
    }
}

impl Calendar {
    pub fn query_tasks(&self, credentials: &Credentials) -> Result<Vec<Task>> {
        let data = Request {
            req_type: "REPORT",
            depth: "1",
            request_data: r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            <c:calendar-data />
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                <c:comp-filter name="VTODO" />
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
"#,
        }
        .perform(&self.url, credentials)?;

        let mut tasks = Vec::new();
        for task in data.children.iter().filter_map(|c| c.as_element()) {
            tasks.append(&mut Task::from_data(&XMLData::parse(task)?, &self.url)?);
        }
        Ok(tasks)
    }

    fn from_xml(base_url: &Url, xml: &xmltree::Element) -> Result<Option<Self>> {
        // TODO: Use XMLData here instead of parsing it again
        // TODO: Add error checking if it's an response element?
        let propstat = xml
            .get_child("propstat")
            .ok_or(Error::Message(String::from("Failed to parse XML response")))?;
        let status = propstat
            .get_child("status")
            .map(|s| xml_to_text(s, ""))
            .unwrap_or(String::from(""));
        if status == "HTTP/1.1 404 Not Found" {
            // This is returned for the root element, ignore this case
            return Ok(None);
        }
        if status != "HTTP/1.1 200 OK" {
            return Err(Error::Message(String::from(
                "Unable to parse XML reponse: Invalid status in element",
            )));
        }

        // TODO: Throw error if url is empty or doesn't exist
        let url = xml.get_child("href").map(|e| xml_to_text(e, ""));
        if url.is_none() {
            // Some elements don't have url (e.g. the root element), ignore them
            return Ok(None);
        }
        let props = propstat
            .get_child("prop")
            .ok_or(Error::Message(String::from(
                "Failed to parse XML response: Missing Props field",
            )))?;

        let mut etag = None;
        let mut displayname = None;
        for prop in props.children.iter().filter_map(|p| p.as_element()) {
            match CalProp::from_xml(prop)? {
                CalProp::Etag(val) => etag = Some(val),
                CalProp::Name(val) => displayname = Some(val),
                CalProp::IsCalendar(false) => {
                    return Ok(None);
                }
                CalProp::IsCalendar(true) => {}
                CalProp::SupportedComponents(components) => {
                    println!("Components: {:?}", components);
                    if !components.contains(&String::from("VTODO")) {
                        return Ok(None);
                    }
                }
            };
        }

        println!("Url: {:?}", url);

        // TODO: Implement Into trait for error
        let url = base_url.join(&url.unwrap())?;

        if etag.is_none() {
            return Err(Error::Message(String::from(
                "XML response; Missing field: etag",
            )));
        }
        if displayname.is_none() {
            return Err(Error::Message(String::from(
                "XML response; Missing field: displayname",
            )));
        }

        Ok(Some(Calendar {
            url,
            etag: etag.unwrap(),
            name: displayname.unwrap(),
        }))
    }
}

struct Request {
    req_type: &'static str,
    depth: &'static str,
    request_data: &'static str,
}

fn xml_to_text(xml: &xmltree::Element, default: &'static str) -> String {
    xml.get_text()
        .map(|t| t.to_string())
        .unwrap_or(String::from(default))
        .replace("\"", "")
}

pub fn apply_xml_path(xml: &xmltree::Element, path: &[&str]) -> Result<String> {
    let mut element = xml;
    for prop in path {
        let mut found = false;
        for e in element.children.iter() {
            if let Some(child) = e.as_element() {
                if child.name == *prop {
                    found = true;
                    element = child;
                    break;
                }
            }
        }
        if !found {
            return Err(Error::Message(String::from("Unable to apply xml path")));
        }
    }

    Ok(element
        .get_text()
        .ok_or(Error::Message(String::from("Unable to apply xml path")))?
        .to_string())
}

impl Request {
    fn perform(self, url: &Url, credentials: &Credentials) -> Result<xmltree::Element> {
        println!("Request:{}", self.request_data);
        let content = Agent::new()
            .request(self.req_type, url.as_str())
            .set("Authorization", &credentials.encode())
            .set("depth", self.depth)
            .send_bytes(self.request_data.as_bytes())
            .map_err(|e| Error::Message(e.to_string()))?
            .into_string()
            .map_err(|e| Error::Message(e.to_string()))?;

        println!("Response:\n{}\n", content);

        Ok(xmltree::Element::parse(content.as_bytes()).map_err(|e| Error::Xml(e))?)
    }
}

fn get_principal_url(url: &Url, credentials: &Credentials) -> Result<Url> {
    let xml = Request {
        req_type: "PROPFIND",
        depth: "0",
        request_data: r#"
        <d:propfind xmlns:d="DAV:">
           <d:prop>
               <d:current-user-principal />
           </d:prop>
        </d:propfind>
    "#,
    }
    .perform(&url, credentials)?;

    Ok(url.join(&apply_xml_path(
        &xml,
        &[
            "response",
            "propstat",
            "prop",
            "current-user-principal",
            "href",
        ],
    )?)?)
}

pub fn get_home_set_url(url: Url, credentials: &Credentials) -> Result<Url> {
    let principal_url = get_principal_url(&url, credentials)?;

    let xml = Request {
        req_type: "PROPFIND",
        depth: "0",
        request_data: r#"
    <d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
      <d:self/>
      <d:prop>
        <c:calendar-home-set />
      </d:prop>
    </d:propfind>
"#,
    }
    .perform(&principal_url, credentials)?;

    Ok(principal_url.join(&apply_xml_path(&xml, &["response", "href"])?)?)
}

pub fn get_calendars(url: &Url, credentials: &Credentials) -> Result<Vec<Calendar>> {
    let home_set_url = get_home_set_url(url.clone(), &credentials)?;
    // TODO: Extract color using the prop <calendar-color xmlns="http://apple.com/ns/ical/" />
    let xml = Request {
        req_type: "PROPFIND",
        depth: "1",
        request_data: r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
    <d:prop>
        <d:getetag />
        <d:displayname />
        <d:resourcetype />
        <c:supported-calendar-component-set />
    </d:prop>
    <c:filter>
        <c:comp-filter name="VCALENDAR" />
    </c:filter>
</c:calendar-query>
"#,
    }
    .perform(&home_set_url, credentials)?;

    let mut calendars = Vec::new();
    for r in xml.children.iter().filter_map(|c| c.as_element()) {
        let calendar = Calendar::from_xml(url, r)?;
        if let Some(calendar) = calendar {
            calendars.push(calendar);
        }
    }
    Ok(calendars)
}
