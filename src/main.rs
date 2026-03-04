use rss_feed::storage::Db;
use rss_feed::{load_sources, output_css, output_list_to_html};
use rss_feed::{parse, Options};
use std::env;
use std::process::Command;
use std::{fs, sync::mpsc::channel, thread};
use turso::Builder;

// TODO: Figure out how to schedule for me

// TODO: Make a more useful app. Allow things like:
//      - Switch to either table or flexbox-based styling
//      - Add a method to export all data
//      - Store sites in db
//          - this will require joins
//      - Add a way to interactively add a site
//      - Browse by author
//      - sort by other fields
//      - mark (and filter by) as read

const APP_DIR: &str = "./app/";
const DB_PATH: &str = constcat::concat!(APP_DIR, "rss.db");
const DB_DRY_PATH: &str = constcat::concat!(APP_DIR, "dry_rss.db");
const OUTPUT_HTML_PATH: &str = constcat::concat!(APP_DIR, "feed.html");
const CSS_LOC: &str = "./assets/style.css";
const SOURCES_FILE: &str = "./sources.csv";

#[tokio::main]
async fn main() {
    let options = Options::new(env::args());

    let mut db = initialize(options.dry_run).await;
    let (tx, rx) = channel();
    let sources = load_sources(SOURCES_FILE).expect("Failed to load sources");
    for site in sources {
        let thread_tx = tx.clone();

        // fetches posts for this site. Completion is guaranteed by blocking on the
        // channel receiver later
        thread::spawn(move || {
            // TODO: Make fail gracefully if something goes wrong. Don't kill everything
            let text = site.get_rss_text().unwrap();
            println!("Fetched rss file for {}, size: {}", site.slug, text.len());

            let parser = parse::Parser::new(&text, &site.author);
            for item in parser.into_iter() {
                thread_tx.send(item).unwrap();
            }
        });
    }
    drop(tx); // main thread doesn't need a sender

    let new_row_count = db
        .upsert_posts(rx.iter().flatten())
        .await
        .expect("Upserting posts failed");
    let all_posts = db
        .fetch_all_posts()
        .await
        .expect("Fetching posts from db failed");

    output_list_to_html(&all_posts, OUTPUT_HTML_PATH);
    output_css(CSS_LOC, APP_DIR);
    println!("Added {new_row_count} posts from feeds.");
    println!("Output {} posts to html.", all_posts.len());

    if options.open_feed {
        // TODO: May only work on MacOS
        Command::new("open")
            .arg(OUTPUT_HTML_PATH)
            .spawn()
            .expect("Should have opened the html file in the browser")
            .wait()
            .unwrap();
    }
}

/// initialize the working directory, database, and return a database connection
///
/// # Panics
/// - Panics if the directory creation fails
async fn initialize(dry_run: bool) -> Db {
    fs::create_dir_all(APP_DIR).expect("Failed creating app directory");

    let db_path = if dry_run {
        fs::copy(DB_PATH, DB_DRY_PATH).expect("Copying db file for dry run failed");
        DB_DRY_PATH
    } else {
        DB_PATH
    };

    let db = Builder::new_local(db_path)
        .build()
        .await
        .expect("Failed to initialize database");
    let conn = db
        .connect()
        .expect("Failed to establish database connection");

    Db::build(conn).await.unwrap()
}
