use std::fs;
use std::result::Result;

extern crate html_escape;

// TODO: is this fast enough or do we need to build our own?
fn decode_string(string: &str) -> String {
    let mut new_string = String::new();
    html_escape::decode_html_entities_to_string(string, &mut new_string);
    return new_string;
}

const TOP_NAVIGATION_SIZE: usize = 4;
const BOTTOM_NAVIGATION_SIZE: usize = 6;
// All links parsed from html document are 12 characters
const HTML_LINK_SIZE: usize = 12;

enum TagType {
    Unknown,
    P,
    Big,
    Pre,
    Link,
    Font,
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

/// Contains the fields of Yle telext site
#[derive(Debug)]
pub struct TeleText<'a> {
    // TODO: since TeleText is going to live as long as the HtmlLoader ref
    //       we could make all the HtmlTexts to be &str to avoid heap allocs
    pub title: HtmlText,
    pub top_navigation: Vec<HtmlItem>,
    pub bottom_navigation: Vec<HtmlLink>,
    pub sub_pages: Vec<HtmlItem>,
    pub middle_rows: Vec<Vec<HtmlItem>>,

    // TODO: should this be part of `HTML_PARSER`
    /// The current slice of the `page_data` that we are currently parsing
    current_text: &'a str,
}

impl<'a> TeleText<'a> {
    /// Get tag type in the current position
    fn get_tag_type(&self) -> TagType {
        let mut html = self.current_text;

        if html.chars().nth(0).unwrap() == '<' {
            html = &html[1..];
        }

        if html.starts_with('p') {
            return TagType::P;
        } else if html.starts_with('a') {
            return TagType::Link;
        } else if html.starts_with("big") {
            return TagType::Big;
        } else if html.starts_with("pre") {
            return TagType::Pre;
        } else if html.starts_with("font") {
            return TagType::Font;
        } else if html.starts_with("center") {
            return TagType::Center;
        }

        TagType::Unknown
    }

    // TODO: combine `skip_next_string` and `skip_next_char` to skip_next_pattern
    //       when the pattern is stable enough to use
    /*  fn skip_next_pattern<P: std::str::pattern::Pattern>(&mut self, pattern: P) {} */

    fn skip_next_char(&mut self, chr: char) {
        let chr_start = self.current_text.find(chr).unwrap();
        let chr_end = chr_start + 1; // char is always + 1
        let slicer = &self.current_text[chr_end..];
        self.current_text = slicer;
    }

    fn skip_next_string(&mut self, string: &str) {
        let string_start = self.current_text.find(string).unwrap();
        let string_end = string_start + string.len();
        let slicer = &self.current_text[string_end..];
        self.current_text = slicer;
    }

    fn skip_next_tag(&mut self, tag: &str, closing: bool) {
        // TODO: Do we need to heap allocate a new string here?
        let html_tag = if closing {
            format!("</{}", tag)
        } else {
            format!("<{}", tag)
        };

        // First skip the start of the tag definition
        self.skip_next_string(&html_tag);
        // Then skip all the way to the end, skipping class="" etc
        self.skip_next_char('>');
    }

    fn parse_current_link(&mut self) -> HtmlLink {
        self.skip_next_string("href=\"");
        let url_end = self.current_text.find('"').unwrap();
        let url = self.current_text[..url_end].to_string();

        // Go to the end of the link thag
        self.skip_next_string(">");

        let inner_end = self.current_text.find('<').unwrap();
        let inner_text = decode_string(&self.current_text[..inner_end]);

        self.skip_next_tag("a", true);

        HtmlLink {
            url: url,
            inner_text: inner_text,
        }
    }

    /// Parse the title part of yle teletext page
    fn parse_title(&mut self) {
        // Title is always between `<big></big>`
        self.skip_next_tag("big", false);
        // Text ends at the start of the next html tag
        let text_end = self.current_text.find('<').unwrap();
        self.title = decode_string(&self.current_text[0..text_end]);
    }

    /// Parse the top navigation par tof yle teletext page
    fn parse_top_navigation(&mut self) {
        self.skip_next_tag("SPAN", false);

        let mut navigation: Vec<HtmlItem> = Vec::new();
        for ii in 0..TOP_NAVIGATION_SIZE {
            let last_link = ii == TOP_NAVIGATION_SIZE - 1;
            if ii != 0 {
                self.skip_next_string("&nbsp;")
            }

            match self.get_tag_type() {
                TagType::Link => {
                    let link = self.parse_current_link();
                    navigation.push(HtmlItem::Link(link));
                }
                _ => {
                    // There is only texts and links in top nav so if
                    // it's not a link, parse it as a text

                    // The text ends either in &nbsp; or start of a html tag
                    let endchar = if last_link { '<' } else { '&' };
                    let text_end = self.current_text.find(endchar).unwrap();
                    let text = self.current_text[..text_end].to_string();
                    navigation.push(HtmlItem::Text(text));
                    self.skip_next_char(endchar);
                }
            }

            if !last_link {
                self.skip_next_string("nbsp;|");
            }
        }

        self.top_navigation = navigation;
    }

    /// If the current link isn't a valid teletext link, this will Err
    /// and return a `HtmlText` instead of the `HtmlLink`
    fn parse_middle_link(&mut self) -> Result<HtmlLink, HtmlText> {
        let link = self.parse_current_link();
        if link.url.len() != HTML_LINK_SIZE {
            return Err(link.inner_text);
        }

        Ok(link)
    }

    fn parse_middle(&mut self) {
        self.skip_next_tag("pre", false);

        let mut middle_rows: Vec<Vec<HtmlItem>> = Vec::new();
        while !self.current_text.starts_with("</pre>") {
            let mut row: Vec<HtmlItem> = Vec::new();
            // ref the current string
            let parse_text = self.current_text;
            // each middle row is in a regular line so lets find the new line
            // so we can now the size of it, so we can skip the line after parsing
            let line_len = self.current_text.find('\r').unwrap();
            // Temporarly ref the current text as the row_text we're parsing
            self.current_text = &self.current_text[..line_len];

            // lines that start with '&' don't actualy contain any text
            if parse_text.len() == 0 || parse_text.starts_with('&') {
                middle_rows.push(row);
                self.current_text = &parse_text[line_len + 2..]; // +2 for "\r\n"
                continue;
            }

            while self.current_text.len() > 0 {
                match self.get_tag_type() {
                    TagType::Link => match self.parse_middle_link() {
                        Ok(link) => {
                            row.push(HtmlItem::Link(link));
                        }
                        Err(text) => {
                            row.push(HtmlItem::Text(text));
                        }
                    },
                    _ => {
                        // There is only texts and links in middle so if
                        // it's not a link, parse it as a text

                        let link_start = self.current_text.find('<');
                        let row_str = if link_start.is_some() {
                            // link_start is some so we can unwrap it here safely
                            decode_string(&self.current_text[..link_start.unwrap()])
                        } else {
                            // If '<' is not found, the rest of the line
                            // is the string, since there are no more links
                            decode_string(&self.current_text[..])
                        };

                        if link_start.is_some() {
                            self.current_text = &self.current_text[link_start.unwrap()..];
                        } else {
                            self.current_text = "";
                        }

                        row.push(HtmlItem::Text(row_str));
                    }
                }
            }

            // Pushed the crated row and make the text refer
            // to the whole document again
            middle_rows.push(row);
            self.current_text = &parse_text[line_len + 2..]; // +2 for "\r\n"
        }

        self.middle_rows = middle_rows;
    }

    fn parse_sub_pages(&mut self) {
        self.skip_next_tag("p", false);

        let mut sub_pages: Vec<HtmlItem> = Vec::new();
        while !self.current_text.starts_with("</p>") {
            match self.get_tag_type() {
                TagType::Font => {
                    self.skip_next_tag("font", true);
                }
                TagType::Link => {
                    let link = self.parse_current_link();
                    sub_pages.push(HtmlItem::Link(link));
                }
                _ => {
                    let link_start = self.current_text.find('<').unwrap();
                    let row_str = self.current_text[..link_start].to_string();
                    sub_pages.push(HtmlItem::Text(row_str));
                    self.current_text = &self.current_text[link_start..];
                }
            }
        }

        self.sub_pages = sub_pages;
    }

    fn parse_bottom_navigation(&mut self) {
        self.skip_next_tag("p", false);
        let mut links: Vec<HtmlLink> = Vec::new();
        for _ in 0..BOTTOM_NAVIGATION_SIZE {
            let link = self.parse_current_link();
            links.push(link);
        }

        self.bottom_navigation = links;
    }

    pub fn parse(&mut self, loader: &'a HtmlLoader) -> Result<(), String> {
        self.current_text = &loader.page_data[..];
        self.parse_title();
        self.parse_top_navigation();
        self.parse_middle();
        self.parse_sub_pages();
        self.parse_bottom_navigation();

        Ok(())
    }

    pub fn new() -> TeleText<'a> {
        TeleText {
            title: "".to_string(),
            top_navigation: vec![],
            bottom_navigation: vec![],
            sub_pages: vec![],
            middle_rows: vec![],
            current_text: "",
        }
    }
}

#[derive(Debug)]
pub struct HtmlLoader {
    page_data: String,
}

impl HtmlLoader {
    pub fn new(file: &str) -> HtmlLoader {
        let data = fs::read_to_string(file).expect(&format!("Can't find \"{}\"", file));

        HtmlLoader { page_data: data }
    }
}
