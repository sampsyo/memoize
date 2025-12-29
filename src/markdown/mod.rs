mod add_ids;
mod rel_links;
mod toc;

use pulldown_cmark::{Options, Parser, html::push_html};

pub fn render(source: &str) -> (String, Vec<toc::TocEntry>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    // TODO gather top-level heading as title

    let mut html_buf = String::new();
    let mut toc_entries = vec![];

    let iter = Parser::new_ext(source, options);
    let iter = add_ids::AddHeadingIds::new(iter);
    let iter = toc::TableOfContents::new(iter, &mut toc_entries);
    let iter = rel_links::RewriteRelativeLinks::new(iter);

    push_html(&mut html_buf, iter);
    (html_buf, toc_entries)
}
