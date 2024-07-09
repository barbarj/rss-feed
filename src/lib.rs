pub mod parse {
    use chrono::NaiveDateTime;
    use quick_xml::{events::Event, Reader};
    use std::fmt::Display;

    pub struct FeedItem<'a> {
        pub link: String,
        pub title: String,
        pub date: NaiveDateTime,
        pub author: &'a str,
    }
    impl<'a> FeedItem<'a> {
        fn default(author: &'a str) -> Self {
            FeedItem {
                link: String::new(),
                title: String::new(),
                date: NaiveDateTime::UNIX_EPOCH,
                author: author,
            }
        }
    }
    impl Display for FeedItem<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "{} \"{}\" ({}) - {}",
                self.date, self.title, self.author, self.link
            ))
        }
    }

    enum CurrentTag {
        Title,
        Link,
        PubDate,
        None,
    }
    enum ParsingMessage {
        ShouldContinue,
        ShouldStop,
    }

    struct ParseState<'a> {
        author: &'a str,
        current_tag: CurrentTag,
        output_items: Option<Vec<FeedItem<'a>>>,
        current_item: Option<FeedItem<'a>>,
    }
    impl<'a> ParseState<'a> {
        fn new(author: &'a str) -> Self {
            ParseState {
                author: &author,
                current_tag: CurrentTag::None,
                output_items: Some(Vec::new()),
                current_item: Some(FeedItem::default(author)),
            }
        }

        // TODO: Improve this state machine. I don't like the mutating of `item`. I'd rather collect parts then emit it at the end if possible
        // TODO: Handle errors more appropriately
        fn handle_event(&mut self, event: Event) -> ParsingMessage {
            match event {
                Event::Start(tag) => match tag.name().as_ref() {
                    b"item" => self.current_tag = CurrentTag::None,
                    b"title" => self.current_tag = CurrentTag::Title,
                    b"link" => self.current_tag = CurrentTag::Link,
                    b"pubDate" => self.current_tag = CurrentTag::PubDate,
                    _ => self.current_tag = CurrentTag::None,
                },
                Event::Text(text) => {
                    let item = self
                        .current_item
                        .as_mut()
                        .expect("This feed item should be present here");
                    match self.current_tag {
                        // TODO: Possiby use COWs in FeedItem instead of forcing copy here
                        CurrentTag::Link => item.link = text.unescape().unwrap().into_owned(),
                        CurrentTag::Title => item.title = text.unescape().unwrap().into_owned(),
                        CurrentTag::PubDate => {
                            item.date = NaiveDateTime::parse_from_str(
                                text.unescape().unwrap().into_owned().as_ref(),
                                "%a, %d %b %Y %H:%M:%S%::z",
                            )
                            .expect("Date parsing failed");
                        }
                        CurrentTag::None => (),
                    }
                }
                Event::End(tag) => match tag.name().as_ref() {
                    b"item" => {
                        let item = self
                            .current_item
                            .take()
                            .expect("There should be an item here.");
                        let list = self
                            .output_items
                            .as_mut()
                            .expect("There should be a list here.");
                        list.push(item);
                        self.current_item = Some(FeedItem::default(self.author));
                    }
                    b"title" | b"link" | b"pubDate" => self.current_tag = CurrentTag::None,
                    _ => (),
                },
                Event::Eof => {
                    return ParsingMessage::ShouldStop;
                }
                _ => (),
            }
            ParsingMessage::ShouldContinue
        }

        fn take_list(&mut self) -> Vec<FeedItem<'a>> {
            let list = self
                .output_items
                .take()
                .expect("There should be a list when we take it.");
            self.output_items = Some(Vec::new()); // TODO: Not sure if need this. Can I somehow mark this object as "dead"?
            list
        }
    }

    pub fn parse_rss<'a>(text: String, author: &'a str) -> Vec<FeedItem<'a>> {
        let mut reader = Reader::from_str(&text);
        let mut buffer = Vec::new();

        let mut parse_state = ParseState::new(&author);
        loop {
            match reader.read_event_into(&mut buffer) {
                Err(e) => eprintln!("ERROR: {e}"),
                Ok(event) => match parse_state.handle_event(event) {
                    ParsingMessage::ShouldContinue => (),
                    ParsingMessage::ShouldStop => {
                        break;
                    }
                },
            }
        }
        parse_state.take_list()
    }
}
