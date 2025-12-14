use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};
use std::collections::VecDeque;

pub fn render(source: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(source, options);

    // TODO produce table of contents
    // TODO gather top-level heading as title

    let mut buf = String::new();
    let iter = AddHeadingIds::new(parser);
    html::push_html(&mut buf, iter);
    buf
}

/// Slugify a string and append it to a buffer.
fn slug_append(buf: &mut String, s: &str) {
    let mut last_is_dash = false;
    buf.extend(s.chars().filter_map(|c| {
        if c.is_alphanumeric() {
            last_is_dash = false;
            Some(c.to_ascii_lowercase())
        } else if last_is_dash {
            None
        } else {
            last_is_dash = true;
            Some('-')
        }
    }));
}

struct AddHeadingIds<'a, I> {
    iter: I,
    buffer: VecDeque<Event<'a>>,
}

impl<'a, I> AddHeadingIds<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            buffer: VecDeque::new(),
        }
    }
}

impl<'a, I> Iterator for AddHeadingIds<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Unbuffer the next buffered event, if any.
        if let Some(event) = self.buffer.pop_front() {
            return Some(event);
        }

        let event = self.iter.next()?;
        match event {
            // A heading without an ID.
            Event::Start(Tag::Heading {
                level,
                id: None,
                classes,
                attrs,
            }) => {
                assert!(self.buffer.is_empty(), "nested headings are not allowed");

                // Buffer up all the events until the header ends.
                // TODO is there a clever iterator helper that can "chop off"
                // another iterator, then we can just `collect` that
                let mut slugbuf = String::new();
                for buf_event in self.iter.by_ref() {
                    let is_end = match &buf_event {
                        Event::End(TagEnd::Heading(_)) => true,
                        Event::Text(text) => {
                            slug_append(&mut slugbuf, text);
                            false
                        }
                        _ => false,
                    };
                    self.buffer.push_back(buf_event);
                    if is_end {
                        break;
                    }
                }

                Some(Event::Start(Tag::Heading {
                    level,
                    id: Some(CowStr::from(slugbuf)),
                    classes,
                    attrs,
                }))
            }
            _ => Some(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_with_ids(source: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        let parser = Parser::new_ext(source, options);

        let mut buf = String::new();
        html::push_html(&mut buf, AddHeadingIds::new(parser));
        buf
    }

    #[test]
    fn non_header() {
        assert_eq!(render_with_ids("*hi*"), "<p><em>hi</em></p>\n");
    }

    #[test]
    fn header_with_id() {
        assert_eq!(render_with_ids("# hi {#x}"), "<h1 id=\"x\">hi</h1>\n");
    }

    #[test]
    fn simple_header() {
        assert_eq!(render_with_ids("# hi"), "<h1 id=\"hi\">hi</h1>\n");
    }

    #[test]
    fn style() {
        assert_eq!(
            render_with_ids("# *hi*"),
            "<h1 id=\"hi\"><em>hi</em></h1>\n"
        );
    }

    #[test]
    fn space() {
        assert_eq!(render_with_ids("# h i"), "<h1 id=\"h-i\">h i</h1>\n");
    }

    #[test]
    fn punctuation() {
        assert_eq!(render_with_ids("# h'i"), "<h1 id=\"h-i\">h'i</h1>\n");
    }

    #[test]
    fn multi_gap() {
        assert_eq!(render_with_ids("# h ' i"), "<h1 id=\"h-i\">h ' i</h1>\n");
    }
}
