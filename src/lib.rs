pub mod parse {
    use chrono::NaiveDateTime;
    use quick_xml::{events::Event, Error, Reader};
    use std::fmt::Display;

    pub struct FeedItem<'a> {
        pub link: String,
        pub title: String,
        pub date: NaiveDateTime,
        pub author: &'a str,
    }
    impl Display for FeedItem<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "{} \"{}\" ({}) - {}",
                self.date, self.title, self.author, self.link
            ))
        }
    }

    #[derive(Debug)]
    enum Tag {
        Item,
        Title,
        Link,
        PubDate,
        None,
    }

    #[derive(PartialEq)]
    enum ParseState {
        OutOfItem,
        InItem,
        InItemTagOpened,
        InItemTextConsumed,
    }

    /// NOTE: This currently assumes (mostly) that the xml is
    /// well-structured
    pub struct Parser<'a, 'b> {
        reader: Reader<&'a [u8]>,
        author: &'b str,
        state: ParseState,
        done: bool,
    }
    impl<'a, 'b> Parser<'a, 'b> {
        pub fn new(input: &'a str, author: &'b str) -> Self {
            Parser {
                reader: Reader::from_str(&input),
                author: &author,
                state: ParseState::OutOfItem,
                done: false,
            }
        }

        // TODO: Get some asserts on self.state to force correct usage

        /// Find the next opening tag type, of the types we recognize
        /// in `Tag`
        fn find_open_tag(&mut self) -> Result<Option<Tag>, Error> {
            // assert!(matches!(self.state, ParseState::InItem));
            let mut tag = Tag::None;
            while matches!(tag, Tag::None) {
                let event = match self.reader.read_event() {
                    Ok(event) => event,
                    Err(err) => {
                        return Err(err);
                    }
                };
                tag = match event {
                    Event::Start(t) => match t.name().as_ref() {
                        b"item" => Tag::Item,
                        b"title" => Tag::Title,
                        b"link" => Tag::Link,
                        b"pubDate" => Tag::PubDate,
                        _ => Tag::None,
                    },
                    Event::Eof => {
                        self.done = true;
                        return Ok(None);
                    }
                    _ => Tag::None,
                };
            }
            // self.state = ParseState::InItemTagOpened;
            Ok(Some(tag))
        }

        /// Find the next piece of text to consume
        fn find_text(&mut self) -> Result<Option<String>, Error> {
            // assert!(matches!(self.state, ParseState::InItemTagOpened));
            loop {
                let event = match self.reader.read_event() {
                    Ok(event) => event,
                    Err(err) => {
                        return Err(err);
                    }
                };
                match event {
                    Event::Text(text) => {
                        // self.state = ParseState::InItemTextConsumed;
                        return text.unescape().map(|t| String::from(t)).map(|t| Some(t));
                    }
                    Event::Eof => {
                        self.done = true;
                        return Ok(None);
                    }
                    _ => (),
                }
            }
        }

        /// Consumes through the next closing tag of of type `tag`
        fn consume_close_tag(&mut self, tag: &Tag) -> Result<(), Error> {
            loop {
                let event = match self.reader.read_event() {
                    Ok(event) => event,
                    Err(err) => {
                        return Err(err);
                    }
                };
                match event {
                    Event::End(t) => match (t.name().as_ref(), tag) {
                        (b"item", Tag::Item) => return Ok(()),
                        (b"title", Tag::Title) => return Ok(()),
                        (b"link", Tag::Link) => return Ok(()),
                        (b"pubDate", Tag::PubDate) => return Ok(()),
                        _ => (),
                    },
                    Event::Eof => {
                        self.done = true;
                        return Ok(());
                    }
                    _ => (),
                };
            }
        }

        /// Returns the next tag type and its contents. Assumes you are _in_ an `<item>`
        fn consume_next_tag(&mut self) -> Result<Option<(Tag, String)>, Error> {
            let tag = self.find_open_tag()?;
            let tag = match tag {
                Some(tag) => tag,
                None => return Ok(None),
            };

            let text = self.find_text()?;
            let text = match text {
                Some(text) => text,
                None => return Ok(None),
            };

            self.consume_close_tag(&tag)?;

            Ok(Some((tag, text)))
        }

        fn next_item(&mut self) -> Result<Option<FeedItem<'b>>, Error> {
            if self.done {
                return Ok(None);
            }

            // find item opening tag
            loop {
                let tag = match self.find_open_tag()? {
                    Some(t) => t,
                    None => return Ok(None),
                };
                if matches!(tag, Tag::Item) {
                    break;
                }
            }

            let mut link: Option<String> = None;
            let mut title: Option<String> = None;
            let mut date: Option<NaiveDateTime> = None;
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
                        date = NaiveDateTime::parse_from_str(&text, "%a, %d %b %Y %H:%M:%S%::z")
                            .map(|x| Some(x))
                            .expect("Date parsing failed")
                    }
                    Tag::None => panic!("Shouldn't happen"), //TODO: remove panic via wrapped error
                }
            }

            let link = link.take().expect("There should be an link here");
            let title = title.take().expect("There should be an title here");
            let date = date.take().expect("There should be an date here");

            Ok(Some(FeedItem {
                link,
                title,
                date,
                author: &self.author,
            }))
        }
    }

    impl<'a, 'b> Iterator for Parser<'a, 'b> {
        type Item = FeedItem<'b>;

        fn next(&mut self) -> Option<Self::Item> {
            // TODO: Figure out how to handle Result better
            self.next_item().expect("Failed to get next item")
        }
    }
}
