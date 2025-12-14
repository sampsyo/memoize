use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::collections::VecDeque;

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

/// A pulldown-cmark adapter that adds IDs to headings that don't already have
/// them by "slugifying" the heading's text.
pub struct AddHeadingIds<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
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

    /// Assuming that `self` is now just after the beginning of a header, buffer
    /// up all the events until the header in `self.buffer`. Return the
    /// slugified version of the header's text contents.
    fn consume_heading(&mut self) -> String {
        assert!(self.buffer.is_empty(), "nested headings are not allowed");
        let mut slugbuf = String::new();

        // This is crying out for a `take_until` iterator method; `take_while`
        // doesn't quite cut it.
        for future_event in self.iter.by_ref() {
            let is_end = match &future_event {
                Event::End(TagEnd::Heading(_)) => true,
                Event::Text(text) => {
                    slug_append(&mut slugbuf, text);
                    false
                }
                _ => false,
            };
            self.buffer.push_back(future_event);
            if is_end {
                break;
            }
        }

        slugbuf
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
            Event::Start(Tag::Heading {
                level,
                id: None,
                classes,
                attrs,
            }) => {
                // It's a heading without an ID. We do our thing.
                let slug = self.consume_heading();
                Some(Event::Start(Tag::Heading {
                    level,
                    id: Some(CowStr::from(slug)),
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
    use pulldown_cmark::{Options, Parser, html};

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
