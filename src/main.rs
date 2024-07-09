use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use std::{
    fs::{self, File},
    io::Write,
};

use rss_feed::parse;

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
        println!("Fetched rss file for {}, size: {}", site.slug, text.len());
        let mut list = parse::parse_rss(text, site.author);
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
