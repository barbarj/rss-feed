use reqwest::blocking::Client;
use rss_feed::{output_list_to_html, storage, Site};
use rss_feed::{parse, FeedItem};
use rusqlite::Connection;
use std::{fs, sync::mpsc::channel, thread};

// TODO: Figure out how to schedule for me
// TODO: Make iterative, so I can keep a history of all posts, since the feed contents
//       may change over time.

const APP_DIR: &'static str = "./app/";
const DB_PATH: &'static str = constcat::concat!(APP_DIR, "db.sqlite");

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
    let mut sqlit_conn = initialize();

    let (tx, rx) = channel();
    let mut handles = Vec::new();
    for site in SITE_LIST.as_ref() {
        let thread_tx = tx.clone();

        // fetches feed items for this site
        let handle = thread::spawn(move || {
            let client = Client::new();
            let text = site.get_rss_text(&client).unwrap();
            println!("Fetched rss file for {}, size: {}", site.slug, text.len());

            let parser = parse::Parser::new(&text, site.author);
            for item in parser.into_iter() {
                thread_tx.send(item).unwrap();
            }
        });
        handles.push(handle);
    }
    drop(tx); // main thread doesn't need a sender

    let total_list: Vec<FeedItem> = rx.iter().collect();
    storage::upsert_posts(&mut sqlit_conn, &total_list).expect("Upserting posts failed");

    // let output_handle = thread::spawn(move || {
    //     let total_list: Vec<FeedItem> = rx.iter().collect();

    //     total_list.sort_by_key(|item: &FeedItem| item.date);
    //     total_list.reverse();
    //     let filepath = format!("{}feed.html", APP_DIR);
    //     output_list_to_html(&total_list, &filepath);
    // });

    // for handle in handles {
    //     handle.join().expect("Thread failed");
    // }
    // output_handle.join().unwrap();
}

/// initialize the working directory, database, and return a database connection
///
/// # Panics
/// - Panics if the directory creation fails
fn initialize() -> Connection {
    fs::create_dir_all(APP_DIR).expect("Failed creating app directory");

    let conn = Connection::open(DB_PATH).expect("Failed to establish database connection");

    storage::idempotently_create_posts_table(&conn).expect("Failed to create posts table.");

    conn
}
