use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use std::{fs, io::Error as IOError, io::Write};

const WORKING_FILES_LOCATION: &str = "./_rss_feed/";

enum DownloadError {
    RequestError(ReqwestError),
    FileError(IOError),
}

struct Site<'a> {
    slug: &'a str,
    rss_link: &'a str,
}
impl Site<'_> {
    fn download_to_working_dir(&self, client: &Client) -> Result<(), DownloadError> {
        let bytes = client
            .get(self.rss_link)
            .send()
            .map(|response| response.bytes());
        let bytes = match bytes {
            Ok(Ok(response)) => response,
            Err(err) => {
                return Err(DownloadError::RequestError(err));
            }
            Ok(Err(err)) => {
                return Err(DownloadError::RequestError(err));
            }
        };

        let output_file_name = format!("{}{}.xml", WORKING_FILES_LOCATION, self.slug);

        let file = fs::File::create(&output_file_name);
        let result = file.map(|mut file| {
            file.write_all(&bytes)?;
            file.flush()?;
            Ok::<(), IOError>(())
        });
        match result {
            Ok(Ok(_)) => Ok(()),
            Err(err) => Err(DownloadError::FileError(err)),
            Ok(Err(err)) => Err(DownloadError::FileError(err)),
        }
    }
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
    initialize();

    let client = Client::new();
    for site in SITE_LIST.as_ref() {
        let res = site.download_to_working_dir(&client);
        if let Err(err) = res {
            match err {
                DownloadError::RequestError(err) => eprintln!("{err}"),
                DownloadError::FileError(err) => eprintln!("{err}"),
            }
        }
        println!("Fetched rss file for {}", site.slug);
    }
}

/// initialize the working directory
///
/// # Panics
/// - Panics if the directory creation fails
fn initialize() {
    fs::create_dir_all(WORKING_FILES_LOCATION).expect("Failed creating working directory");
}
