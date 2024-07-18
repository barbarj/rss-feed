use chrono::{DateTime, Utc};
use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Error, Reader,
};

use crate::Post;

#[derive(Debug)]
enum Tag {
    Item,
    Entry,
    Title,
    Link,
    PubDate,
    Updated,
    None,
}
impl Tag {
    fn name(&self) -> Option<&[u8]> {
        match self {
            Tag::Item => Some(b"item"),
            Tag::Entry => Some(b"entry"),
            Tag::Title => Some(b"title"),
            Tag::Link => Some(b"link"),
            Tag::PubDate => Some(b"pubDate"),
            Tag::Updated => Some(b"updated"),
            Tag::None => None,
        }
    }

    fn from_name(name: &[u8]) -> Self {
        match name {
            b"item" => Tag::Item,
            b"entry" => Tag::Entry,
            b"title" => Tag::Title,
            b"link" => Tag::Link,
            b"pubDate" => Tag::PubDate,
            b"updated" => Tag::Updated,
            _ => Tag::None,
        }
    }
}
impl From<&BytesStart<'_>> for Tag {
    fn from(value: &BytesStart) -> Self {
        Tag::from_name(value.name().as_ref())
    }
}

// TODO: Add atom parser

enum DocStyle {
    RSS,
    Atom,
}

pub struct Parser<'a, 'b> {
    reader: Reader<&'a [u8]>,
    author: &'b str,
    style: DocStyle,
    done: bool,
}
impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &'a str, author: &'b str) -> Self {
        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(true);

        let _first_event = reader.read_event().expect("Reading first event failed.");
        // Skip xml declaration event
        assert!(matches!(Event::Decl, _first_event));

        // Determine type of document by first tag
        // - rss starts with <rss>
        // - atom starts with <feed>
        // - can panic on unknown tag
        let first_tag_event = reader
            .read_event()
            .expect("Reading first tag event failed.");
        let style = match first_tag_event {
            Event::Start(t) => match t.name().as_ref() {
                b"rss" => DocStyle::RSS,
                b"feed" => DocStyle::Atom,
                _ => panic!("Invalid first tag name"),
            },
            _ => panic!("Invalid first event type."),
        };

        Parser {
            reader,
            author: &author,
            style,
            done: false,
        }
    }

    fn read_through_start(&mut self, tag: Tag) -> Result<Option<BytesStart>, Error> {
        assert!(!matches!(tag, Tag::None));

        while let Ok(event) = self.reader.read_event() {
            match event {
                Event::Start(t) if t.name().as_ref() == tag.name().unwrap() => return Ok(Some(t)),
                Event::Eof => {
                    self.done = true;
                    break;
                }
                _ => (),
            }
        }
        Ok(None)
    }

    /// Consumes through the next closing tag of of type `tag`
    fn consume_close_tag(&mut self, end: BytesEnd) -> Result<(), Error> {
        self.reader.read_to_end(end.name())?;
        Ok(())
    }

    // TODO: Mimic code style of consume_next_tag_atom
    /// Returns the next tag type and its contents. Assumes you are _in_ an `<item>`
    fn consume_next_tag_rss(&mut self) -> Result<Option<(Tag, String)>, Error> {
        let next_event = self.reader.read_event()?;
        let start = match next_event {
            Event::Start(t) => t,
            Event::Eof => return Ok(None),
            _ => {
                eprintln!("failed on: {next_event:?}");
                panic!("Should be impossible. XML is likely malformed.");
            }
        };
        let end = start.to_end();
        let tag = Tag::from_name(start.name().as_ref());

        let text = self.reader.read_text(end.name())?;
        let text = Parser::extract_text(&text);
        Ok(Some((tag, text)))
    }

    fn consume_next_tag_atom(&mut self) -> Result<Option<(Tag, String)>, Error> {
        let next_event = self.reader.read_event()?;
        let (tag, text) = match next_event {
            Event::Start(t) => {
                let text = self.reader.read_text(t.to_end().name())?;
                (Tag::from(&t), Parser::extract_text(&text))
            }
            Event::Empty(t) => {
                let text = t
                    .attributes()
                    .find(|res| res.as_ref().unwrap().key.as_ref() == b"href")
                    .expect("Finding href tag on link failed.")?
                    .unescape_value()?;
                (Tag::from(&t), Parser::extract_text(&text))
            }
            Event::Eof => return Ok(None),
            _ => {
                eprintln!("failed on: {next_event:?}");
                panic!("Should be impossible. XML is likely malformed.");
            }
        };

        Ok(Some((tag, text)))
    }

    /// Consumes the xml enough to produce the next item.
    ///
    /// # How:
    /// 1. Find's next opening item tag: `<item>`
    /// 2. Extracts text from the relevant tags within that item
    /// 3. Returns the completed `FeedItem`
    fn next_item(&mut self) -> Result<Option<Post>, Error> {
        if self.done {
            return Ok(None);
        }

        // find item opening tag
        let opening_tag_type = match self.style {
            DocStyle::Atom => Tag::Entry,
            DocStyle::RSS => Tag::Item,
        };
        let start = match self.read_through_start(opening_tag_type)? {
            Some(s) => s.to_owned(),
            None => return Ok(None),
        };

        // get post parts
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        let mut date: Option<DateTime<Utc>> = None;
        while link.is_none() || title.is_none() || date.is_none() {
            let tag_and_text = match self.style {
                DocStyle::Atom => self.consume_next_tag_atom()?,
                DocStyle::RSS => self.consume_next_tag_rss()?,
            };
            let (tag, text) = match tag_and_text {
                Some((tag, text)) => (tag, text),
                None => return Ok(None),
            };
            // TODO: Extract handling this based on doc style
            match tag {
                Tag::Item | Tag::Entry => panic!("Shouldn't happen"), //TODO: remove panic via wrapped error
                Tag::Link => link = Some(text),
                Tag::Title => title = Some(text),
                Tag::PubDate | Tag::Updated => {
                    let d = DateTime::parse_from_rfc3339(&text)
                        .or(DateTime::parse_from_rfc2822(&text))
                        .expect("Date parsing failed");
                    date = Some(d.with_timezone(&Utc));
                }
                Tag::None => (),
            }
        }
        let link = link.take().expect("There should be an link here");
        let title = title.take().expect("There should be an title here");
        let date = date.take().expect("There should be an date here");

        // consume the closing tag
        self.consume_close_tag(start.to_end())?;

        Ok(Some(Post {
            link,
            title,
            date,
            author: self.author.to_owned(),
        }))
    }

    fn extract_text(text: &str) -> String {
        const CDATA_START: &'static str = "<![CDATA[";
        // remove cdata wrapper if necessary
        if text.starts_with(CDATA_START) {
            text.trim_start_matches(CDATA_START)
                .trim_end_matches("]]>")
                .to_string()
        } else {
            text.to_string()
        }
    }
}

impl<'a, 'b> Iterator for Parser<'a, 'b> {
    type Item = Result<Post, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_item() {
            Ok(Some(post)) => Some(Ok(post)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
