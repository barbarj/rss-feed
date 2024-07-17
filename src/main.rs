use rss_feed::{output_css, output_list_to_html, storage, Site};
use rss_feed::{parse, Options};
use rusqlite::Connection;
use std::env;
use std::process::Command;
use std::{fs, sync::mpsc::channel, thread};

// TODO: Figure out how to schedule for me

const APP_DIR: &'static str = "./app/";
const DB_PATH: &'static str = constcat::concat!(APP_DIR, "db.sqlite");
const OUTPUT_HTML_PATH: &'static str = constcat::concat!(APP_DIR, "feed.html");
const CSS_LOC: &'static str = "./assets/style.css";

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
    // Site {
    //     slug: "thorstenball",
    //     rss_link: "https://thorstenball.com/atom.xml",
    //     author: "Thorsten Ball",
    // },
    // Site {
    //     slug: "registerspill",
    //     rss_link: "https://registerspill.thorstenball.com/feed",
    //     author: "Thorsten Ball",
    // },
];

fn main() {
    let options = Options::new(env::args());

    let mut sqlit_conn = initialize();

    let (tx, rx) = channel();
    for site in SITE_LIST.as_ref() {
        let thread_tx = tx.clone();

        // fetches posts for this site. Completion is guaranteed by blocking on the
        // channel receiver later
        thread::spawn(move || {
            // TODO: Make fail gracefully if something goes wrong. Don't kill everything
            let text = site.get_rss_text().unwrap();
            println!("Fetched rss file for {}, size: {}", site.slug, text.len());

            let parser = parse::Parser::new(&text, site.author);
            for item in parser.into_iter() {
                thread_tx.send(item).unwrap();
            }
        });
    }
    drop(tx); // main thread doesn't need a sender

    let mut txn = sqlit_conn.transaction().unwrap();
    let new_row_count =
        storage::upsert_posts(&mut txn, rx.iter().flatten()).expect("Upserting posts failed");
    txn.commit().unwrap();
    let all_posts = storage::fetch_all_posts(&sqlit_conn).expect("Fetching posts from db failed");

    output_list_to_html(&all_posts, &OUTPUT_HTML_PATH);
    output_css(CSS_LOC, APP_DIR);
    println!("Added {new_row_count} posts from feeds.");
    println!("Output {} posts to html.", all_posts.len());

    if options.open_feed {
        // TODO: May only work on MacOS
        Command::new("open")
            .arg(&OUTPUT_HTML_PATH)
            .spawn()
            .expect("Should have opened the html file in the browser");
    }
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
