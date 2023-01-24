use std::result::Result;

use super::common::{
    decode_string, HtmlItem, HtmlLink, HtmlLoader, HtmlParser, HtmlText, InnerResult, ParseErr,
    ParseState, ParserResult, TagType,
};

extern crate html_escape;

const TOP_NAVIGATION_SIZE: usize = 4;
const BOTTOM_NAVIGATION_SIZE: usize = 6;
// All links parsed from html document are 12 characters
const HTML_LINK_SIZE: usize = 12;
/// The middle texts are always maximum of 39 characters
pub const MIDDLE_TEXT_MAX_LEN: usize = 39;

/// Contains the fields of Yle telext site
#[derive(Debug)]
pub struct TeleText {
    pub title: HtmlText,
    pub page_navigation: Vec<HtmlItem>,
    pub bottom_navigation: Vec<HtmlLink>,
    pub sub_pages: Vec<HtmlItem>,
    pub middle_rows: Vec<Vec<HtmlItem>>,
}

impl TeleText {
    /// Parse the title part of yle teletext page
    fn parse_title<'a>(state: &'a mut ParseState<'a>) -> InnerResult<'a, HtmlText> {
        // Title is always between `<big></big>`
        let state = Self::skip_next_tag(state, "big", false)?.0;
        // Text ends at the start of the next html tag
        let text_end = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
        // self.title = decode_string(&state.current()[0..text_end]);
        let title = decode_string(&state.current[0..text_end]);
        Ok((state, title))
    }
    /// Parse the top navigation par tof yle teletext page
    fn parse_top_navigation<'a>(
        mut state: &'a mut ParseState<'a>,
    ) -> InnerResult<'a, Vec<HtmlItem>> {
        state = Self::skip_next_tag(state, "SPAN", false)?.0;

        let mut navigation: Vec<HtmlItem> = Vec::new();
        for ii in 0..TOP_NAVIGATION_SIZE {
            let last_link = ii == TOP_NAVIGATION_SIZE - 1;
            if ii != 0 {
                state = Self::skip_next_string(state, "&nbsp;")?.0;
            }

            match Self::get_tag_type(state.current) {
                TagType::Link => {
                    let (new_state, link) = Self::parse_current_link(state)?;
                    state = new_state;
                    navigation.push(HtmlItem::Link(link));
                }
                _ => {
                    // There is only texts and links in top nav so if
                    // it's not a link, parse it as a text

                    // The text ends either in &nbsp; or start of a html tag
                    let endchar = if last_link { '<' } else { '&' };
                    let text_end = state.current.find(endchar).ok_or(ParseErr::InvalidPage)?;
                    let text = state.current[..text_end].to_string();
                    navigation.push(HtmlItem::Text(text));
                    state = Self::skip_next_char(state, endchar)?.0;
                }
            }

            if !last_link {
                state = Self::skip_next_string(state, "nbsp;|")?.0;
            }
        }

        Ok((state, navigation))
    }

    /// If the current link isn't a valid teletext link, this will Err
    /// and return a `HtmlText` instead of the `HtmlLink`
    fn parse_middle_link<'a>(
        mut state: &'a mut ParseState<'a>,
    ) -> InnerResult<'a, Result<HtmlLink, HtmlText>> {
        let (new_state, link) = Self::parse_current_link(state)?;
        state = new_state;

        if link.url.len() != HTML_LINK_SIZE {
            return Ok((state, Err(link.inner_text)));
        }

        Ok((state, Ok(link)))
    }

    fn parse_middle<'a>(mut state: &'a mut ParseState<'a>) -> InnerResult<'a, Vec<Vec<HtmlItem>>> {
        state = Self::skip_next_tag(state, "pre", false)?.0;

        let mut middle_rows: Vec<Vec<HtmlItem>> = Vec::new();
        while !state.current.starts_with("</pre>") {
            let mut row: Vec<HtmlItem> = Vec::new();
            // ref the current string
            let parse_text = state.current;
            // each middle row is in a regular line so lets find the new line
            // so we can now the size of it, so we can skip the line after parsing
            let line_len = state.current.find('\r').ok_or(ParseErr::InvalidPage)?;
            // Temporarly ref the current text as the row_text we're parsing
            state.current = &state.current[..line_len];

            // lines that start with '&' don't actualy contain any text
            if parse_text.is_empty() || parse_text.starts_with('&') {
                middle_rows.push(row);
                state.current = &parse_text[line_len + 2..]; // +2 for "\r\n"
                continue;
            }

            while !state.current.is_empty() {
                match Self::get_tag_type(state.current) {
                    TagType::Link => {
                        let (new_state, middle) = Self::parse_middle_link(state)?;
                        state = new_state;
                        match middle {
                            Ok(link) => {
                                row.push(HtmlItem::Link(link));
                            }
                            Err(text) => {
                                row.push(HtmlItem::Text(text));
                            }
                        }
                    }
                    _ => {
                        // There is only texts and links in middle so if
                        // it's not a link, parse it as a text

                        let link_start = state.current.find('<');
                        let row_str = if let Some(start) = link_start {
                            // link_start is some so we can unwrap it here safely
                            decode_string(&state.current[..start])
                        } else {
                            // If '<' is not found, the rest of the line
                            // is the string, since there are no more links
                            decode_string(state.current)
                        };

                        if let Some(start) = link_start {
                            state.current = &state.current[start..];
                        } else {
                            state.current = "";
                        }

                        row.push(HtmlItem::Text(row_str));
                    }
                }
            }

            // Pushed the crated row and make the text refer
            // to the whole document again
            middle_rows.push(row);
            state.current = &parse_text[line_len + 2..]; // +2 for "\r\n"
        }

        Ok((state, middle_rows))
    }

    fn parse_sub_pages<'a>(mut state: &'a mut ParseState<'a>) -> InnerResult<'a, Vec<HtmlItem>> {
        state = Self::skip_next_tag(state, "p", false)?.0;

        let mut sub_pages: Vec<HtmlItem> = Vec::new();
        while !state.current.starts_with("</p>") {
            match Self::get_tag_type(state.current) {
                TagType::Font => {
                    state = Self::skip_next_tag(state, "font", true)?.0;
                }
                TagType::Link => {
                    let (new_state, link) = Self::parse_current_link(state)?;
                    state = new_state;
                    sub_pages.push(HtmlItem::Link(link));
                }
                _ => {
                    let link_start = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
                    let row_str = state.current[..link_start].to_string();
                    sub_pages.push(HtmlItem::Text(row_str));
                    state.current = &state.current[link_start..];
                }
            }
        }

        Ok((state, sub_pages))
    }

    fn parse_bottom_navigation<'a>(
        mut state: &'a mut ParseState<'a>,
    ) -> InnerResult<'a, Vec<HtmlLink>> {
        state = Self::skip_next_tag(state, "p", false)?.0;
        let mut links: Vec<HtmlLink> = Vec::new();
        for _ in 0..BOTTOM_NAVIGATION_SIZE {
            let (new_state, link) = Self::parse_current_link(state)?;
            state = new_state;
            links.push(link);
        }

        Ok((state, links))
    }
}

impl HtmlParser for TeleText {
    fn new() -> TeleText {
        TeleText {
            title: "".to_string(),
            page_navigation: vec![],
            bottom_navigation: vec![],
            sub_pages: vec![],
            middle_rows: vec![],
        }
    }

    fn parse(mut self, loader: HtmlLoader) -> ParserResult<Self> {
        let mut state = ParseState::new(&loader.page_data);
        let (state, title) = Self::parse_title(&mut state)?;
        self.title = title;
        let (state, top_nav) = Self::parse_top_navigation(state)?;
        self.page_navigation = top_nav;
        let (state, middle) = Self::parse_middle(state)?;
        self.middle_rows = middle;
        let (state, sub_pages) = Self::parse_sub_pages(state)?;
        self.sub_pages = sub_pages;
        self.bottom_navigation = Self::parse_bottom_navigation(state)?.1;

        Ok(self)
    }
}
