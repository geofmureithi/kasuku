pub use pulldown_cmark::Alignment;
pub use pulldown_cmark::Event;
pub use pulldown_cmark::HeadingLevel;
pub use pulldown_cmark::LinkType;
use serde::Deserialize;
use serde::Serialize;
use types::Error;
use types::File;
use types::FileType;
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

#[cfg(feature = "backend")]
pub fn parse(content: &str) -> Result<MarkdownFile<'_>, types::Error> {
    let options = pulldown_cmark::Options::all();
    let parser = pulldown_cmark::Parser::new_ext(content, options);
    let events: Vec<pulldown_cmark::Event<'_>> = parser.collect();
    Ok(MarkdownFile { events })
}

impl<'a> MarkdownFile<'a> {
    pub fn get_contents(&'a self) -> &Vec<Event<'a>> {
        &self.events
    }

    pub fn get_contents_mut(&'a mut self) -> &mut Vec<Event<'a>> {
        &mut self.events
    }
}
pub type Regex = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarkdownEvent {
    #[serde(rename = "MarkdownEvent::Tag")]
    Tag(Tag),
    Text(Regex),
    InlineCode(Regex),
    FootNote(Regex),
    TaskList,
}

impl PluginEvent for MarkdownEvent {
    type Plugin = IdentityPlugin;
}

/// Codeblock kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CodeBlockKind {
    Indented,
    Fenced(Regex),
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
    Heading(Option<HeadingLevel>, Option<Regex>, Vec<Regex>),

    BlockQuote,
    /// A code block.
    CodeBlock(CodeBlockKind),

    List,
    /// A list item.
    Item,
    /// A footnote definition. The value contained is the footnote's label by which it can
    /// be referred to.
    FootnoteDefinition(Regex),

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
    Link(Option<LinkType>, Option<Regex>, Option<Regex>),

    /// An image. The first field is the link type, the second the destination URL and the third is a title.
    Image(Option<LinkType>, Option<Regex>, Option<Regex>),
}

pub trait AsMarkdown {
    fn to_markdown<'a: 'de, 'de>(&'a self) -> Result<MarkdownFile<'de>, Error>;
    fn to_file(file: MarkdownFile<'_>, path: String) -> Result<File, Error>;
}

impl AsMarkdown for FileType {
    fn to_file(file: MarkdownFile<'_>, path: String) -> Result<File, Error> {
        Ok(File {
            path,
            data: FileType::Markdown(
                bincode::serialize(&file).map_err(|e| Error::FileCodec(e.to_string()))?,
            ),
        })
    }
    fn to_markdown<'a: 'de, 'de>(&'a self) -> Result<MarkdownFile<'de>, Error> {
        let res = match self {
            FileType::Markdown(bytes) => {
                bincode::deserialize(bytes).map_err(|e| Error::FileCodec(e.to_string()))?
            }
            _ => unreachable!(),
        };
        Ok(res)
    }
}
