mod add_ids;
mod toc;

use pulldown_cmark::{Options, Parser, html::push_html};

pub fn render(source: &str) -> (String, Vec<toc::TocEntry>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    // TODO gather top-level heading as title

    let mut html_buf = String::new();
    let mut toc_entries = vec![];

    let iter = Parser::new_ext(source, options);
    let iter = add_ids::AddHeadingIds::new(iter);
    let iter = toc::TableOfContents::new(iter, &mut toc_entries);

    push_html(&mut html_buf, iter);
    (html_buf, toc_entries)
}
