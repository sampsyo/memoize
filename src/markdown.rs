use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use std::collections::VecDeque;

pub fn render(source: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(source, options);

    // TODO generate slugified anchors, produce table of contents
    // TODO gather top-level heading as title

    let mut buf = String::new();
    let wrapped = AddHeadingIds::new(parser); // TODO silly name
    html::push_html(&mut buf, wrapped);
    buf
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
        match &event {
            // A heading without an ID.
            Event::Start(Tag::Heading {
                level: _,
                id: None,
                classes: _,
                attrs: _,
            }) => {
                assert!(self.buffer.is_empty(), "nested headings are not allowed");

                // Buffer up all the events until the header ends.
                // TODO is there a clever iterator helper that can "chop off"
                // another iterator, then we can just `collect` that
                let mut buf = vec![];
                let mut textbuf = String::new(); // TODO avoid all the concatenation
                while let Some(buf_event) = self.iter.next() {
                    let is_end = match &buf_event {
                        Event::End(TagEnd::Heading(_)) => true,
                        Event::Text(text) => {
                            dbg!("hi", text);
                            textbuf.push_str(text);
                            false
                        }
                        _ => false,
                    };
                    buf.push(buf_event);
                    if is_end {
                        break;
                    }
                }
                self.buffer.extend(buf); // TODO avoid the vec

                Some(event)
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
        assert_eq!(render_with_ids("# *hi*"), "<h1 id=\"hi\">hi</h1>\n");
    }

    #[test]
    fn spaces() {
        assert_eq!(render_with_ids("# h i"), "<h1 id=\"h-i\">hi</h1>\n");
    }
}
