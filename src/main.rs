use reqwest::blocking::Client;
use std::{fs, io::Write};

const WORKING_FILES_LOCATION: &str = "./_rss_feed/";

struct Site<'a> {
    slug: &'a str,
    rss_link: &'a str,
}

static SITE_LIST: [Site; 3] = [
    Site {
        slug: "eatonphil",
        rss_link: "https://notes.eatonphil.com/rss.xml",
    },
    Site {
        slug: "danluu",
        rss_link: "https://danluu.com/atom.xml",
    },
    Site {
        slug: "hillelwayne",
        rss_link: "https://buttondown.email/hillelwayne/rss",
    },
];

fn main() {
    // initialize working environment if it doesn't exist yet.
    fs::create_dir_all(WORKING_FILES_LOCATION).expect("Failed creating working directory");

    // TODO: Read files and write them locally.
    let client = Client::new();
    for site in SITE_LIST.as_ref() {
        let res = client
            .get(site.rss_link)
            .send()
            .expect(format!("Failed to fetch rss for site: {}", site.slug).as_ref());
        let bytes = res
            .bytes()
            .expect("Failed converting response to its bytes");
        let output_file_name = format!("{}{}.xml", WORKING_FILES_LOCATION, site.slug);

        let mut file = fs::File::create(&output_file_name)
            .expect(format!("Failed to create file: {}", output_file_name).as_ref());
        file.write_all(&bytes)
            .expect("Failed to write bytes to file");
        file.flush().expect("Failed to flush file");
    }
}
