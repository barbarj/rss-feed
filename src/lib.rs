use std::env::Args;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;

use chrono::{DateTime, Utc};
use csv::{Reader, Result as CSVResult};
use reqwest::Error as ReqwestError;
use serde::Deserialize;

pub mod parse;
pub mod storage;

pub struct Options {
    pub serial: bool,
    pub open_feed: bool,
    pub dry_run: bool,
    pub output_html_directory: Option<String>,
}
impl Options {
    pub fn new(mut args: Args) -> Self {
        // skip program name
        args.next();
        let args: Vec<String> = args.collect();

        let serial = args.iter().any(|a| a == "--serial");
        let open_feed = args.iter().any(|a| a == "-o" || a == "--open");
        let dry_run = args.iter().any(|a| a == "--dry-run");
        let output_html_directory = flag_arg_from_args(&args, "output_html_directory");

        Options {
            serial,
            open_feed,
            dry_run,
            output_html_directory,
        }
    }
}

fn flag_arg_from_args(args: &[String], flag_name: &str) -> Option<String> {
    let flag = "--".to_string() + flag_name;
    args.iter()
        .skip_while(|a| *a != &flag)
        .nth(1)
        .map(|v| v.to_string())
}

#[derive(Deserialize)]
pub struct Site {
    pub slug: String,
    pub rss_link: String,
    pub author: String,
}
impl Site {
    pub async fn get_rss_text(&self) -> Result<String, ReqwestError> {
        // TODO: Make retry on certain kinds of failures
        reqwest::get(&self.rss_link).await?.text().await
    }
}

#[derive(Clone)]
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
    let mut file =
        File::create(filepath).expect(&format!("Failed to create html file for '{filepath}'"));
    file.write_all(
        "<html lang=\"en\"><head><link rel=\"stylesheet\" href=\"./style.css\"></head><body>"
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
    fs::copy(css_path, app_dir.to_string() + "/style.css").expect("Copying CSS file failed.");
}

pub fn load_sources(sources_file: &str) -> CSVResult<Vec<Site>> {
    let mut reader = Reader::from_path(sources_file)?;
    reader.deserialize().collect()
}
