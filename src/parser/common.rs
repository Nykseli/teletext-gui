use std::fs;
use std::result::Result;

extern crate html_escape;

#[derive(Debug)]
pub enum ParseErr {
    InvalidPage,
}

pub struct ParseState<'a> {
    pub current: &'a str,
}

impl<'a> ParseState<'a> {
    pub fn new(current: &'a str) -> Self {
        Self { current }
    }
}

pub type InnerResult<'a, T> = Result<(&'a mut ParseState<'a>, T), ParseErr>;
pub type ParserResult<T> = Result<T, ParseErr>;

// TODO: is this fast enough or do we need to build our own?
pub fn decode_string(string: &str) -> String {
    let mut new_string = String::new();
    html_escape::decode_html_entities_to_string(string, &mut new_string);
    new_string
}

#[derive(Debug)]
pub enum TagType {
    Unknown,
    P,
    Big,
    Div,
    Pre,
    Link,
    Font,
    Span,
    Center,
}

// TODO: use this to avoid heap allocs
// pub type HtmlText<'a> = &'a str;
pub type HtmlText = String;

#[derive(Debug)]
pub struct HtmlLink {
    pub url: HtmlText,
    pub inner_text: HtmlText,
}

#[derive(Debug)]
pub enum HtmlItem {
    Text(HtmlText),
    Link(HtmlLink),
}

#[derive(Debug)]
pub struct HtmlImageArea {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub link: String,
}

impl HtmlImageArea {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, link: String) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            link,
        }
    }

    pub fn in_area(&self, x: f32, y: f32, scale: f32) -> bool {
        if x < (self.x1 * scale) || x > (self.x2 * scale) {
            return false;
        }

        if y < (self.y1 * scale) || y > (self.y2 * scale) {
            return false;
        }

        true
    }
}

pub trait HtmlParser {
    /// Get tag type in the current position
    fn get_tag_type(current: &str) -> TagType
    where
        Self: Sized,
    {
        let mut html = current;

        if html.starts_with('<') {
            html = &html[1..];
        }

        if html.starts_with('p') {
            return TagType::P;
        } else if html.starts_with('a') {
            return TagType::Link;
        } else if html.starts_with("big") {
            return TagType::Big;
        } else if html.starts_with("div") {
            return TagType::Div;
        } else if html.starts_with("pre") {
            return TagType::Pre;
        } else if html.starts_with("font") {
            return TagType::Font;
        } else if html.starts_with("span") {
            return TagType::Span;
        } else if html.starts_with("center") {
            return TagType::Center;
        }

        TagType::Unknown
    }

    // TODO: combine `skip_next_string` and `skip_next_char` to skip_next_pattern
    //       when the pattern is stable enough to use
    /*  fn skip_next_pattern<P: std::str::pattern::Pattern>(state, pattern: P) {} */

    fn skip_next_char<'a>(state: &'a mut ParseState<'a>, chr: char) -> InnerResult<'a, ()>
    where
        Self: Sized,
    {
        let chr_start = state.current.find(chr).ok_or(ParseErr::InvalidPage)?;
        let chr_end = chr_start + 1; // char is always + 1
        state.current = &state.current[chr_end..];
        Ok((state, ()))
    }

    fn skip_to_next_char<'a>(state: &'a mut ParseState<'a>, chr: char) -> InnerResult<'a, ()>
    where
        Self: Sized,
    {
        let chr_start = state.current.find(chr).ok_or(ParseErr::InvalidPage)?;
        state.current = &state.current[chr_start..];
        Ok((state, ()))
    }

    fn skip_next_string<'a>(state: &'a mut ParseState<'a>, string: &str) -> InnerResult<'a, ()>
    where
        Self: Sized,
    {
        let string_start = state.current.find(string).ok_or(ParseErr::InvalidPage)?;
        let string_end = string_start + string.len();
        state.current = &state.current[string_end..];
        Ok((state, ()))
    }

    fn skip_next_tag<'a>(
        mut state: &'a mut ParseState<'a>,
        tag: &str,
        closing: bool,
    ) -> InnerResult<'a, ()>
    where
        Self: Sized,
    {
        // TODO: Do we need to heap allocate a new string here?
        let html_tag = if closing {
            format!("</{}", tag)
        } else {
            format!("<{}", tag)
        };

        // First skip the start of the tag definition
        state = Self::skip_next_string(state, &html_tag)?.0;
        // Then skip all the way to the end, skipping class="" etc
        state = Self::skip_next_char(state, '>')?.0;
        Ok((state, ()))
    }

    fn parse_current_link<'a>(mut state: &'a mut ParseState<'a>) -> InnerResult<'a, HtmlLink>
    where
        Self: Sized,
    {
        state = Self::skip_next_string(state, "href=\"")?.0;
        let url_end = state.current.find('"').ok_or(ParseErr::InvalidPage)?;
        let url = state.current[..url_end].to_string();

        // Go to the end of the link tag
        state = Self::skip_next_string(state, ">")?.0;

        let inner_end = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
        let inner_text = decode_string(&state.current[..inner_end]);

        state = Self::skip_next_tag(state, "a", true)?.0;

        Ok((state, HtmlLink { url, inner_text }))
    }

    fn parse(self, loader: HtmlLoader) -> ParserResult<Self>
    where
        Self: Sized;
    fn new() -> Self
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct HtmlLoader {
    pub page_data: String,
}

impl HtmlLoader {
    #[allow(dead_code)]
    pub fn new(file: &str) -> HtmlLoader {
        let data = fs::read_to_string(file).unwrap_or_else(|_| panic!("Can't find \"{}\"", file));

        HtmlLoader { page_data: data }
    }
}
