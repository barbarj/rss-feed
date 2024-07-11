use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;

use chrono::NaiveDateTime;
use reqwest::Error as ReqwestError;

pub mod parse;

pub struct Site<'a> {
    pub slug: &'a str,
    pub rss_link: &'a str,
    pub author: &'a str,
}
impl Site<'_> {
    pub fn get_rss_text(&self) -> Result<String, ReqwestError> {
        reqwest::blocking::get(self.rss_link)?.text()
    }
}

pub struct Post {
    pub link: String,
    pub title: String,
    pub date: NaiveDateTime,
    pub author: String,
}
impl Post {
    pub fn parse_stored_date(text: &str) -> Result<NaiveDateTime, chrono::ParseError> {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
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

pub fn output_css(css_path: &str, app_dir: &str) {
    fs::copy(css_path, app_dir.to_string() + "style.css").expect("Copying CSS file failed.");
}

pub mod storage {
    use rusqlite::{Connection, Transaction};

    use crate::Post;

    pub fn idempotently_create_posts_table(conn: &Connection) -> Result<(), rusqlite::Error> {
        let rows_changed = conn.execute(
            "CREATE TABLE IF NOT EXISTS posts( \
                link TEXT PRIMARY KEY, \
                title TEXT, \
                date TEXT, \
                author TEXT \
            );",
            [],
        )?;
        assert_eq!(rows_changed, 0);
        Ok(())
    }

    pub fn upsert_posts(tx: &mut Transaction, posts: &[Post]) -> Result<usize, rusqlite::Error> {
        let mut stmt = tx.prepare(
            "INSERT INTO posts(link, title, date, author) \
                            VALUES(:link, :title, :date, :author) \
                            ON CONFLICT(link) DO NOTHING;",
        )?;

        let mut rows_affected = 0;

        for post in posts {
            rows_affected += stmt.execute(&[
                (":link", &post.link),
                (":title", &post.title),
                (":date", &post.date.to_string()),
                (":author", &post.author),
            ])?;
        }
        Ok(rows_affected)
    }

    pub fn fetch_all_posts(conn: &Connection) -> Result<Vec<Post>, rusqlite::Error> {
        let mut stmt =
            conn.prepare("SELECT link, title, date, author FROM posts ORDER BY date DESC;")?;

        let posts = stmt
            .query([])?
            .mapped(|row| {
                let d: String = row.get(2)?;
                let date = Post::parse_stored_date(&d).expect("Parsing stored date failed");
                let item = Post {
                    link: row.get(0)?,
                    title: row.get(1)?,
                    date,
                    author: row.get(3)?,
                };
                Ok(item)
            })
            .flatten()
            .collect();
        Ok(posts)
    }
}
