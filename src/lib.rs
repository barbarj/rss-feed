use std::env::Args;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;

use chrono::{DateTime, Utc};
use reqwest::Error as ReqwestError;

pub mod parse;
pub mod storage;

pub struct Options {
    pub open_feed: bool,
    pub dry_run: bool,
}
impl Options {
    pub fn new(mut args: Args) -> Self {
        // skip program name
        args.next();
        let args: Vec<String> = args.collect();

        let open_feed = args.iter().any(|a| a == "-o" || a == "--open");
        let dry_run = args.iter().any(|a| a == "--dry-run");
        // TODO: dry run flag

        Options { open_feed, dry_run }
    }
}

pub struct Site<'a> {
    pub slug: &'a str,
    pub rss_link: &'a str,
    pub author: &'a str,
}
impl Site<'_> {
    pub fn get_rss_text(&self) -> Result<String, ReqwestError> {
        // TODO: Make retry on certain kinds of failures
        reqwest::blocking::get(self.rss_link)?.text()
    }
}

pub struct Post {
    pub link: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub author: String,
}
impl Post {
    pub fn parse_stored_date(text: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        let dt = DateTime::parse_from_rfc3339(text)?;
        Ok(dt.with_timezone(&Utc))
    }
}
impl Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} \"{}\" ({}) - {}",
            self.date, self.title, self.author, self.link
        ))
    }
}

pub fn output_list_to_html(list: &Vec<Post>, filepath: &str) {
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
            item.date.date_naive(),
            item.author,
            item.link,
            item.title,
        ))
        .unwrap();
    }
    file.write_all("</body></html>".as_bytes()).unwrap();
    file.flush().unwrap();
}

pub fn output_css(css_path: &str, app_dir: &str) {
    fs::copy(css_path, app_dir.to_string() + "style.css").expect("Copying CSS file failed.");
}
