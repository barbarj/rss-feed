use chrono::NaiveDateTime;
use quick_xml::{events::Event, Reader};
use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use std::{
    fmt::Display,
    fs::{self, File},
    io::Write,
};

const OUTPUT_HTML_DIR: &str = "./html/";

#[derive(Debug)]
enum DownloadError {
    RequestError(ReqwestError),
}

struct Site<'a> {
    slug: &'a str,
    rss_link: &'a str,
    author: &'a str,
}
impl Site<'_> {
    fn get_rss_text(&self, client: &Client) -> Result<String, DownloadError> {
        let text = client
            .get(self.rss_link)
            .send()
            .map(|response| response.text());
        match text {
            Ok(Ok(t)) => Ok(t),
            Err(err) => {
                return Err(DownloadError::RequestError(err));
            }
            Ok(Err(err)) => {
                return Err(DownloadError::RequestError(err));
            }
        }
    }
}

static SITE_LIST: [Site; 3] = [
    Site {
        slug: "eatonphil",
        rss_link: "https://notes.eatonphil.com/rss.xml",
        author: "Phil Eaton",
    },
    Site {
        slug: "danluu",
        rss_link: "https://danluu.com/atom.xml",
        author: "Dan Luu",
    },
    Site {
        slug: "hillelwayne",
        rss_link: "https://buttondown.email/hillelwayne/rss",
        author: "Hillel Wayne",
    },
];

fn main() {
    initialize();

    let client = Client::new();

    let mut total_list = Vec::new();
    for site in SITE_LIST.as_ref() {
        let text = site.get_rss_text(&client);
        if let Err(err) = &text {
            match err {
                DownloadError::RequestError(err) => eprintln!("{err}"),
            }
        }
        let text = text.expect("Should be impossible");
        println!("Fetched rss file for {}", site.slug);
        let mut list = parse_rss(text, site.author);
        total_list.append(&mut list);
    }
    total_list.sort_by_key(|item| item.date);
    total_list.reverse();
    output_list_to_html(&total_list);
}

/// initialize the working directory
///
/// # Panics
/// - Panics if the directory creation fails
fn initialize() {
    fs::create_dir_all(OUTPUT_HTML_DIR).expect("Failed creating html directory");
}

struct FeedItem<'a> {
    link: String,
    title: String,
    date: NaiveDateTime,
    author: &'a str,
}
impl<'a> FeedItem<'a> {
    fn default(author: &'a str) -> Self {
        FeedItem {
            link: String::new(),
            title: String::new(),
            date: NaiveDateTime::UNIX_EPOCH,
            author: author,
        }
    }
}
impl Display for FeedItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} \"{}\" ({}) - {}",
            self.date, self.title, self.author, self.link
        ))
    }
}

enum CurrentTag {
    Title,
    Link,
    PubDate,
    None,
}

// TODO: Improve this state machine. I don't like the mutating of `item`. I'd rather collect parts then emit it at the end if possible
// TODO: Handle errors more appropriately
fn parse_rss<'a>(text: String, author: &'a str) -> Vec<FeedItem<'a>> {
    let mut reader = Reader::from_str(&text);
    let mut buffer = Vec::new();

    let mut item = FeedItem::default(&author);
    let mut current_tag: CurrentTag = CurrentTag::None;

    let mut list = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Err(e) => eprintln!("ERROR: {e}"),
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"item" => (),
                b"title" => current_tag = CurrentTag::Title,
                b"link" => current_tag = CurrentTag::Link,
                b"pubDate" => current_tag = CurrentTag::PubDate,
                _ => (),
            },
            Ok(Event::Text(text)) => match current_tag {
                // TODO: Possiby use COWs in FeedItem instead of forcing copy here
                CurrentTag::Link => item.link = text.unescape().unwrap().into_owned(),
                CurrentTag::Title => item.title = text.unescape().unwrap().into_owned(),
                CurrentTag::PubDate => {
                    item.date = NaiveDateTime::parse_from_str(
                        text.unescape().unwrap().into_owned().as_ref(),
                        "%a, %d %b %Y %H:%M:%S%::z",
                    )
                    .expect("Date parsing failed");
                }
                CurrentTag::None => (),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"item" => {
                    list.push(item);
                    item = FeedItem::default(&author);
                }
                b"title" | b"link" | b"pubDate" => current_tag = CurrentTag::None,
                _ => (),
            },
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    list
}

fn output_list_to_html(list: &Vec<FeedItem>) {
    let filepath = format!("{}feed.html", OUTPUT_HTML_DIR);
    let mut file = File::create(filepath).expect("Failed to create html file.");
    file.write_all(
        "<html lang=\"en\"><head><link rel=\"stylesheet\" href=\"style.css\"></head><body>"
            .as_bytes(),
    )
    .unwrap();
    for item in list {
        file.write_fmt(format_args!(
            " \
            <div class=\"item\"> \
                <span class=\"date\">{}</span> \
                <a href=\"{}\">{}</a> \
                - <span class=\"author\">{}</span> \
            </div> \
        ",
            item.date.date(),
            item.link,
            item.title,
            item.author
        ))
        .unwrap();
    }
    file.write_all("</body></html>".as_bytes()).unwrap();
    file.flush().unwrap();
}
