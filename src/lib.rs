use std::fs::File;
use std::io::Write;

use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;

pub mod parse;

pub struct Site<'a> {
    pub slug: &'a str,
    pub rss_link: &'a str,
    pub author: &'a str,
}
impl Site<'_> {
    pub fn get_rss_text(&self, client: &Client) -> Result<String, ReqwestError> {
        client.get(self.rss_link).send()?.text()
    }
}

pub fn output_list_to_html(list: &Vec<parse::FeedItem>, filepath: &str) {
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
