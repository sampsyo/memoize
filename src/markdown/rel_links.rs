use pulldown_cmark::{CowStr, Event, Tag};

/// A pulldown_cmark adapter that rewrites relative Markdown links to be HTML
/// links. So a link to `./foo.md` becomes a link to `./foo.html` when rendered,
/// but all absolute links are left unchanged.
pub struct RewriteRelativeLinks<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    iter: I,
}

impl<'a, 'b, I> RewriteRelativeLinks<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'a, 'b, I> Iterator for RewriteRelativeLinks<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.iter.next()? {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let url = if is_absolute_url(&dest_url) {
                    dest_url
                } else {
                    rewrite_url(dest_url)
                };
                Event::Start(Tag::Link {
                    link_type,
                    dest_url: url,
                    title,
                    id,
                })
            }
            e => e,
        })
    }
}

/// Check whether a URL is absolute, i.e., starts with a protocol.
fn is_absolute_url(url: &str) -> bool {
    let colon = url.find(':');
    let slash = url.find('/');
    match (colon, slash) {
        (Some(c), Some(s)) if c < s => true,
        (_, Some(s)) => match url.find("//") {
            Some(ss) if ss <= s => true,
            _ => false,
        },
        (_, _) => false,
    }
}

/// Rewrite any `.md` extension to `.html`. If it doesn't have this extension,
/// return the path unchanged.
fn rewrite_url(url: CowStr) -> CowStr {
    match url.rsplit_once(".") {
        Some((base, ext)) if ext == "md" => format!("{base}.html").into(),
        _ => url,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn filename_is_relative() {
        assert!(!is_absolute_url("foo.html"));
    }

    #[test]
    fn full_url_is_absolute() {
        assert!(is_absolute_url("http://foo.org/bar"));
    }

    #[test]
    fn empty_protocol_is_absolute() {
        assert!(is_absolute_url("//foo.org/bar"));
    }

    #[test]
    fn absolute_path_is_relative() {
        assert!(!is_absolute_url("/foo/bar"));
    }

    #[test]
    fn dot_is_relative() {
        assert!(!is_absolute_url("./bar"));
    }

    #[test]
    fn dotdot_is_relative() {
        assert!(!is_absolute_url("../bar"));
    }

    #[test]
    fn later_double_slash_is_relative() {
        assert!(!is_absolute_url("foo/bar//baz"));
    }

    use super::*;
    use pulldown_cmark::{Parser, html};

    fn render_rewrite(source: &str) -> String {
        let parser = Parser::new(source);

        let mut buf = String::new();
        html::push_html(&mut buf, RewriteRelativeLinks::new(parser));
        buf
    }

    #[test]
    fn absolute_md_link() {
        assert_eq!(
            render_rewrite("[hi](http://foo.com/bar.md)"),
            "<p><a href=\"http://foo.com/bar.md\">hi</a></p>\n"
        );
    }

    #[test]
    fn relative_md_link() {
        assert_eq!(
            render_rewrite("[hi](bar.md)"),
            "<p><a href=\"bar.html\">hi</a></p>\n"
        );
    }

    #[test]
    fn relative_other_link() {
        assert_eq!(
            render_rewrite("[hi](./bar.png)"),
            "<p><a href=\"./bar.png\">hi</a></p>\n"
        );
    }

    #[test]
    fn relative_md_link_refstyle() {
        assert_eq!(
            render_rewrite("[hi][h]\n\n[h]: ./bar.md"),
            "<p><a href=\"./bar.html\">hi</a></p>\n"
        );
    }
}
