use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

#[derive(Debug, PartialEq, Eq)]
struct TocEntry {
    level: HeadingLevel,
    id: Option<String>,
    title: String,
}

pub struct TableOfContents<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    iter: I,
    entries: Vec<TocEntry>,
    in_heading: bool,
}

impl<'a, I> TableOfContents<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            entries: Vec::new(),
            in_heading: false,
        }
    }
}

impl<'a, I> Iterator for TableOfContents<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.iter.next()?;
        match &event {
            Event::Start(Tag::Heading {
                level,
                id,
                classes: _,
                attrs: _,
            }) => {
                // Start building a new TOC entry for this heading.
                self.entries.push(TocEntry {
                    level: *level,
                    id: id.as_ref().map(|s| s.to_string()),
                    title: String::new(),
                });
                self.in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                // Finish a TOC entry.
                assert!(self.in_heading, "heading ended without starting");
                self.in_heading = false;
            }
            Event::Text(text) => {
                if self.in_heading {
                    if let Some(entry) = self.entries.last_mut() {
                        entry.title += text;
                    } else {
                        panic!("no entry created for heading");
                    }
                }
            }
            _ => (),
        }
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    fn get_toc<'a>(source: &'a str) -> Vec<TocEntry> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        let parser = Parser::new_ext(source, options);
        let mut tocgen = TableOfContents::new(parser);
        tocgen.by_ref().for_each(|_| {}); // Just consume the whole iterator.
        tocgen.entries
    }

    #[test]
    fn no_headings() {
        assert_eq!(get_toc("hi"), &[]);
    }

    #[test]
    fn a_heading() {
        assert_eq!(
            get_toc("# hi"),
            &[TocEntry {
                level: HeadingLevel::H1,
                id: None,
                title: "hi".to_string(),
            }]
        );
    }

    #[test]
    fn heading_with_id() {
        assert_eq!(
            get_toc("# hi {#x}"),
            &[TocEntry {
                level: HeadingLevel::H1,
                id: Some("x".to_string()),
                title: "hi".to_string(),
            }]
        );
    }

    #[test]
    fn two_headings() {
        assert_eq!(
            get_toc("# hi {#x}\n## bye {#y}"),
            &[
                TocEntry {
                    level: HeadingLevel::H1,
                    id: Some("x".to_string()),
                    title: "hi".to_string(),
                },
                TocEntry {
                    level: HeadingLevel::H2,
                    id: Some("y".to_string()),
                    title: "bye".to_string(),
                },
            ]
        );
    }

    #[test]
    fn heading_and_other_text() {
        assert_eq!(
            get_toc("above\n# hi\nbelow"),
            &[TocEntry {
                level: HeadingLevel::H1,
                id: None,
                title: "hi".to_string(),
            }]
        );
    }
}
