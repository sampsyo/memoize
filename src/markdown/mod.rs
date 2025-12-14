mod add_ids;

use pulldown_cmark::{Options, Parser, html};

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
    let iter = add_ids::AddHeadingIds::new(parser);
    html::push_html(&mut buf, iter);
    buf
}
