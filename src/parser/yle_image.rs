use base64::{engine::general_purpose, Engine as _};

use super::common::{
    decode_string, HtmlLink, HtmlLoader, HtmlParser, HtmlText, InnerResult, ParseErr, ParseState,
    ParserResult, TagType,
};

extern crate html_escape;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJMeta {
    code: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJDataPage {
    page: String,
    subpage: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJDataInfoPage {
    /// e.g. "898"
    number: String,
    /// e.g. "898_0003"
    name: String,
    /// e.g. "898/3"
    label: String,
    /// "?P=898#3"
    href: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJDataInfo {
    page: IJDataInfoPage,
    aspect_ratio: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJDataContent {
    /// Raw text repesentation of the image
    text: String,
    /// Image base64 data in html <img> tag
    image: String,
    image_map: String,
    pagination: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct IJData {
    page: IJDataPage,
    info: IJDataInfo,
    content: IJDataContent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ImageJson {
    meta: IJMeta,
    /// IJData is in array but there only seems to be on item at a time
    data: Vec<IJData>,
}

/// Contains the fields of Yle image site
#[derive(Debug)]
pub struct YleImage {
    pub title: HtmlText,
    pub image: Vec<u8>,
    pub botton_navigation: Vec<Option<HtmlLink>>,
}

impl YleImage {
    fn parse_image<'a>(state: &'a mut ParseState<'a>) -> InnerResult<'a, Vec<u8>> {
        let state = Self::skip_next_string(state, "data:image/png;base64,")?.0;
        let image_end = state.current.find('"').ok_or(ParseErr::InvalidPage)?;
        let image = general_purpose::STANDARD
            .decode(&state.current[..image_end])
            .unwrap();

        Ok((state, image))
    }

    fn parse_bottom_nav_link<'a>(mut state: &'a mut ParseState<'a>) -> InnerResult<'a, HtmlLink> {
        state = Self::skip_next_string(state, "data-yle-ttv-page-name=\"")?.0;
        let url_end = state.current.find('"').ok_or(ParseErr::InvalidPage)?;
        let url = state.current[..url_end].to_string();

        // Go to the end of the link tag
        state = Self::skip_next_char(state, '>')?.0;
        let inner_text = if state.current.starts_with('<') {
            // Skip the span open
            state = Self::skip_next_char(state, '>')?.0;
            let span_end = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
            let span_inner = &state.current[..span_end];
            // Skip the span close
            state = Self::skip_next_char(state, '>')?.0;
            let link_start = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
            let span_out = decode_string(&state.current[..link_start]);
            format!("{} {}", span_inner.trim(), span_out.trim())
        } else {
            // Get the string before span
            let span_start = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
            let span_out = decode_string(&state.current[..span_start]);
            // Skip the span open
            state = Self::skip_next_char(state, '>')?.0;
            let span_end = state.current.find('<').ok_or(ParseErr::InvalidPage)?;
            let span_inner = &state.current[..span_end];
            format!("{} {}", span_out.trim(), span_inner.trim())
        };

        state = Self::skip_next_tag(state, "a", true)?.0;
        Ok((state, HtmlLink { url, inner_text }))
    }

    fn parse_bottom_navigation<'a>(
        mut state: &'a mut ParseState<'a>,
    ) -> InnerResult<'a, Vec<Option<HtmlLink>>> {
        // state = Self::skip_next_string(state, "js-yle-ttv-pagination")?.0;

        let mut nav_links: Vec<Option<HtmlLink>> = Vec::new();
        while !state.current.is_empty() {
            state = Self::skip_next_char(state, '<')?.0;
            let tag = Self::get_tag_type(state.current);
            match tag {
                // Spans without link are the hidden navs
                TagType::Span => {
                    state = Self::skip_next_tag(state, "span", true)?.0;
                    nav_links.push(None);
                }
                // Div is the text page input, but we hadle it in title, like in text version
                TagType::Div => {
                    state = Self::skip_next_tag(state, "form", true)?.0;
                    state = Self::skip_next_tag(state, "div", true)?.0;
                }
                // Links contain the actual pages
                TagType::Link => {
                    let (new_state, link) = Self::parse_bottom_nav_link(state)?;
                    state = new_state;
                    nav_links.push(Some(link));
                }
                // Everything else is invalid
                _ => return Err(ParseErr::InvalidPage),
            }

            state.current = if let Some(chr_start) = state.current.find('<') {
                &state.current[chr_start..]
            } else {
                ""
            };
        }

        Ok((state, nav_links))
    }
}

impl HtmlParser for YleImage {
    // type ReturnType = Self;
    fn new() -> Self {
        Self {
            title: "".into(),
            image: Vec::new(),
            botton_navigation: Vec::new(),
        }
    }

    fn parse(mut self, loader: HtmlLoader) -> ParserResult<Self> {
        let json: ImageJson =
            serde_json::from_str(&loader.page_data).map_err(|_| ParseErr::InvalidPage)?;
        self.title = json.data[0].info.page.label.clone();
        let mut state = ParseState::new(&json.data[0].content.image);
        self.image = Self::parse_image(&mut state)?.1;
        let mut state = ParseState::new(&json.data[0].content.pagination);
        self.botton_navigation = Self::parse_bottom_navigation(&mut state)?.1;

        Ok(self)
    }
}
