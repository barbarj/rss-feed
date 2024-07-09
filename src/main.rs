use reqwest::blocking::Client;
use rss_feed::parse::{self, FeedItem};
use rss_feed::{output_list_to_html, Site};
use std::{fs, sync::mpsc::channel, thread};

// TODO: Figure out how to schedule for me

const OUTPUT_HTML_DIR: &str = "./html/";

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

    let (tx, rx) = channel();
    let mut handles = Vec::new();
    for site in SITE_LIST.as_ref() {
        let thread_tx = tx.clone();
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

    let output_handle = thread::spawn(move || {
        let mut total_list: Vec<FeedItem> = rx.iter().collect();
        total_list.sort_by_key(|item: &FeedItem| item.date);
        total_list.reverse();
        let filepath = format!("{}feed.html", OUTPUT_HTML_DIR);
        output_list_to_html(&total_list, &filepath);
    });

    for handle in handles {
        handle.join().expect("Thread failed");
    }
    output_handle.join().unwrap();
}

/// initialize the working directory
///
/// # Panics
/// - Panics if the directory creation fails
fn initialize() {
    fs::create_dir_all(OUTPUT_HTML_DIR).expect("Failed creating html directory");
}
