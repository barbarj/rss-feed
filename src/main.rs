use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use rss_feed::parse::{self, FeedItem};
use std::{
    fs::{self, File},
    io::Write,
    sync::{Arc, Mutex},
    thread,
};

// TODO: Faster updates (parrallelize them)
// TODO: Figure out how to schedule for me

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

    let total_list = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for site in SITE_LIST.as_ref() {
        let thread_total_list = Arc::clone(&total_list);
        let handle = thread::spawn(move || {
            let client = Client::new();
            let text = site.get_rss_text(&client);
            if let Err(err) = &text {
                match err {
                    DownloadError::RequestError(err) => eprintln!("{err}"),
                }
            }
            let text = text.expect("Should be impossible");
            println!("Fetched rss file for {}, size: {}", site.slug, text.len());

            let parser = parse::Parser::new(&text, site.author);
            let mut list: Vec<FeedItem> = parser.into_iter().collect();

            let mut full_list = thread_total_list
                .lock()
                .expect("Failed unlocking total list");
            full_list.append(&mut list);
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().expect("Thread failed");
    }
    let mut total_list = total_list
        .lock()
        .expect("Failed getting lock on total list after all threads complete.");
    total_list.sort_by_key(|item: &FeedItem| item.date);
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

fn output_list_to_html(list: &Vec<parse::FeedItem>) {
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
                <span class=\"author\">{}</span> \
                <a href=\"{}\">{}</a> \
            </div> \
        ",
            item.date.date(),
            item.author,
            item.link,
            item.title,
        ))
        .unwrap();
    }
    file.write_all("</body></html>".as_bytes()).unwrap();
    file.flush().unwrap();
}
