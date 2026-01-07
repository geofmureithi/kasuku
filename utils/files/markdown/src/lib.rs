use std::ops::Deref;
use std::ops::DerefMut;
use std::str::FromStr;

pub use pulldown_cmark::Alignment;
pub use pulldown_cmark::Event;
pub use pulldown_cmark::HeadingLevel;
pub use pulldown_cmark::LinkType;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use types::Error;
use types::File;
use types::IdentityPlugin;
use types::PluginEvent;

pub mod cmark {
    pub use pulldown_cmark::*;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownFile<'a> {
    #[serde(borrow)]
    pub events: Vec<Event<'a>>,
}

// fn handle_page_links<'a, I>(events: I) -> impl Iterator<Item = Event<'a>>
// where
//     I: Iterator<Item = Event<'a>>,
// {
//     // Compile the regex once to improve performance
//     let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();

//     events.flat_map(move |event| match event {
//         Event::Text(text) => {
//             let mut transformed = Vec::new();
//             let mut last_end = 0;

//             // Iterate over all matches of [[...]] in the text
//             for cap in re.captures_iter(&text) {
//                 let full_match = cap.get(0).unwrap(); // e.g., [[Project X]]
//                 let link_text = cap.get(1).unwrap().as_str(); // e.g., Project X

//                 // Add any text before the current match as a Text event
//                 if full_match.start() > last_end {
//                     transformed.push(Event::Text(CowStr::Boxed(
//                         text[last_end..full_match.start()]
//                             .to_owned()
//                             .into_boxed_str(),
//                     )));
//                 }

//                 // Create the Start(Link) event
//                 transformed.push(Event::Start(Tag::Link(
//                     LinkType::Inline,
//                     CowStr::Boxed(link_text.to_owned().into_boxed_str()), // Destination URL
//                     CowStr::Boxed("".into()), // Title (empty in this case)
//                 )));

//                 // Add the link text as a Text event
//                 transformed.push(Event::Text(CowStr::Boxed(
//                     link_text.to_owned().into_boxed_str(),
//                 )));

//                 // Create the End(Link) event
//                 transformed.push(Event::End(TagEnd::Link));

//                 // Update the last_end to the end of the current match
//                 last_end = full_match.end();
//             }

//             // Add any remaining text after the last match as a Text event
//             if last_end < text.len() {
//                 transformed.push(Event::Text(CowStr::Boxed(
//                     text[last_end..].to_owned().into_boxed_str(),
//                 )));
//             }

//             transformed
//         }
//         // Pass through all other events unchanged
//         other => vec![other],
//     })
// }

#[cfg(feature = "backend")]
pub fn parse(content: &str) -> Result<MarkdownFile<'_>, types::Error> {
    let options = pulldown_cmark::Options::all();
    let parser = pulldown_cmark::Parser::new_ext(content, options);

    let events: Vec<pulldown_cmark::Event<'_>> = parser.collect();
    Ok(MarkdownFile { events })
}

impl<'a> Deref for MarkdownFile<'a> {
    type Target = Vec<Event<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl DerefMut for MarkdownFile<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    #[serde(with = "serde_regex")]
    inner: Regex,
}

impl FromStr for Pattern {
    type Err = regex::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { inner: s.parse()? })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarkdownEvent {
    #[serde(rename = "MarkdownEvent::Tag")]
    Tag(Tag),
    #[serde(rename = "MarkdownEvent::Text")]
    Text(Pattern),
    #[serde(rename = "MarkdownEvent::InlineCode")]
    InlineCode(Pattern),
    #[serde(rename = "MarkdownEvent::FootNote")]
    FootNote(Pattern),
    #[serde(rename = "MarkdownEvent::TaskList")]
    TaskList,
}

impl PluginEvent for MarkdownEvent {
    type Plugin = IdentityPlugin;
}

/// Codeblock kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CodeBlockKind {
    Indented,
    Fenced(Pattern),
}

impl CodeBlockKind {
    pub fn is_indented(&self) -> bool {
        matches!(*self, CodeBlockKind::Indented)
    }

    pub fn is_fenced(&self) -> bool {
        matches!(*self, CodeBlockKind::Fenced(_))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Tag {
    /// A paragraph of text and other inline elements.
    Paragraph,

    /// A heading. The first field indicates the level of the heading,
    /// the second the fragment identifier, and the third the classes.
    Heading(Option<HeadingLevel>, Option<Pattern>, Vec<Pattern>),

    BlockQuote,
    /// A code block.
    CodeBlock(CodeBlockKind),

    List,
    /// A list item.
    Item,
    /// A footnote definition. The value contained is the footnote's label by which it can
    /// be referred to.
    FootnoteDefinition(Pattern),

    /// A table. Contains a vector describing the text-alignment for each of its columns.
    Table(Vec<Alignment>),
    /// A table header. Contains only `TableCell`s. Note that the table body starts immediately
    /// after the closure of the `TableHead` tag. There is no `TableBody` tag.
    TableHead,
    /// A table row. Is used both for header rows as body rows. Contains only `TableCell`s.
    TableRow,
    TableCell,

    // span-level tags
    Emphasis,
    Strong,
    Strikethrough,

    /// A link. The first field is the link type, the second the destination URL and the third is a title.
    Link(Option<LinkType>, Option<Pattern>, Option<Pattern>),

    /// An image. The first field is the link type, the second the destination URL and the third is a title.
    Image(Option<LinkType>, Option<Pattern>, Option<Pattern>),
}

#[cfg(feature = "backend")]
pub trait IsMatched {
    fn is_matched(&self, tag: &pulldown_cmark::Event<'_>) -> Result<bool, regex::Error>;
}

#[cfg(feature = "backend")]
impl IsMatched for MarkdownEvent {
    fn is_matched(&self, event: &pulldown_cmark::Event<'_>) -> Result<bool, regex::Error> {
        let res = match &self {
            MarkdownEvent::Tag(tag) => {
                if let pulldown_cmark::Event::Start(inner) = event {
                    tag.is_matched(inner)
                } else {
                    false
                }
            }
            MarkdownEvent::Text(text) => {
                if let pulldown_cmark::Event::Text(inner) = event {
                    text.inner.is_match(inner)
                } else {
                    false
                }
            }
            MarkdownEvent::InlineCode(text) => {
                if let pulldown_cmark::Event::Code(inner) = event {
                    text.inner.is_match(inner)
                } else {
                    false
                }
            }
            MarkdownEvent::FootNote(text) => {
                if let pulldown_cmark::Event::FootnoteReference(inner) = event {
                    text.inner.is_match(inner)
                } else {
                    false
                }
            }
            MarkdownEvent::TaskList => {
                matches!(event, pulldown_cmark::Event::TaskListMarker(_))
            }
        };
        Ok(res)
    }
}

#[cfg(feature = "backend")]
impl Tag {
    pub fn is_matched(&self, tag: &pulldown_cmark::Tag<'_>) -> bool {
        match self {
            Tag::Paragraph => matches!(tag, pulldown_cmark::Tag::Paragraph),
            Tag::Heading(..) => {
                matches!(tag, pulldown_cmark::Tag::Heading { .. })
            }
            Tag::BlockQuote => matches!(tag, pulldown_cmark::Tag::BlockQuote(_)),
            Tag::CodeBlock(_) => matches!(tag, pulldown_cmark::Tag::CodeBlock(_)),
            Tag::List => matches!(tag, pulldown_cmark::Tag::List(_)),
            Tag::Item => matches!(tag, pulldown_cmark::Tag::Item),
            Tag::FootnoteDefinition(_) => todo!(),
            Tag::Table(_) => todo!(),
            Tag::TableHead => matches!(tag, pulldown_cmark::Tag::TableHead),
            Tag::TableRow => matches!(tag, pulldown_cmark::Tag::TableRow),
            Tag::TableCell => matches!(tag, pulldown_cmark::Tag::TableCell),
            Tag::Emphasis => matches!(tag, pulldown_cmark::Tag::Emphasis),
            Tag::Strong => matches!(tag, pulldown_cmark::Tag::Strong),
            Tag::Strikethrough => matches!(tag, pulldown_cmark::Tag::Strikethrough),
            Tag::Link(_, _, _) => todo!(),
            Tag::Image(_, _, _) => todo!(),
        }
    }
}

impl<'a> TryFrom<&'a File> for MarkdownFile<'a> {
    type Error = Error;

    fn try_from(value: &'a File) -> Result<Self, Self::Error> {
        plugy::core::codec::deserialize(&value.data).map_err(|e| Error::FileCodec(e.to_string()))
    }
}

impl TryInto<File> for MarkdownFile<'_> {
    type Error = Error;
    fn try_into(self) -> Result<File, Self::Error> {
        Ok(File {
            data: plugy::core::codec::serialize(&self)
                .map_err(|e| Error::FileCodec(e.to_string()))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{html, Options, Parser};

    #[test]
    fn test_handle_page_links() {
        let markdown = r#"```rs,test
println!("HelloWorld");
```"#;
        let parser = Parser::new_ext(markdown, Options::all());
        let events : Vec<_> = parser.collect();
        dbg!(&events);

    }
}
