use chrono::{DateTime, Utc};
use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Error, Reader,
};

use crate::Post;

#[derive(Debug)]
enum Tag {
    Item,
    Title,
    Link,
    PubDate,
    None,
}
impl Tag {
    fn name(&self) -> Option<&[u8]> {
        match self {
            Tag::Item => Some(b"item"),
            Tag::Title => Some(b"title"),
            Tag::Link => Some(b"link"),
            Tag::PubDate => Some(b"pubDate"),
            Tag::None => None,
        }
    }

    fn from_name(name: &[u8]) -> Self {
        match name {
            b"item" => Tag::Item,
            b"title" => Tag::Title,
            b"link" => Tag::Link,
            b"pubDate" => Tag::PubDate,
            _ => Tag::None,
        }
    }
}

/// NOTE: This currently assumes (mostly) that the xml is
/// well-structured
pub struct Parser<'a, 'b> {
    reader: Reader<&'a [u8]>,
    author: &'b str,
    done: bool,
}
impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &'a str, author: &'b str) -> Self {
        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(true);
        let parser = Parser {
            reader: reader,
            author: &author,
            done: false,
        };

        parser
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

    /// Returns the next tag type and its contents. Assumes you are _in_ an `<item>`
    fn consume_next_tag(&mut self) -> Result<Option<(Tag, String)>, Error> {
        let next_event = self.reader.read_event()?;
        let start = match next_event {
            Event::Start(t) => t,
            Event::Eof => return Ok(None),
            _ => panic!("Should be impossible. XML is likely malformed."),
        };
        let end = start.to_end();
        let tag = Tag::from_name(start.name().as_ref());

        let text = self.reader.read_text(end.name())?.into_owned();
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
        let start = match self.read_through_start(Tag::Item)? {
            Some(s) => s.to_owned(),
            None => return Ok(None),
        };

        // get post parts
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        let mut date: Option<DateTime<Utc>> = None;
        while link.is_none() || title.is_none() || date.is_none() {
            let (tag, text) = match self.consume_next_tag()? {
                Some((tag, text)) => (tag, text),
                None => return Ok(None),
            };
            match tag {
                Tag::Item => panic!("Shouldn't happen"), //TODO: remove panic via wrapped error
                Tag::Link => link = Some(text),
                Tag::Title => title = Some(text),
                Tag::PubDate => {
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
