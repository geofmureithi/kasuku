use context::Context;
use interface::Plugin;
use markdown::{cmark::CowStr, AsMarkdown, CodeBlockKind, MarkdownEvent, Tag, MarkdownFile};
use plugy::macros::plugin_impl;
use serde::Deserialize;
use types::{Error, File, FileType, Task};

#[derive(Debug, Deserialize, Default)]
pub struct DataView;

enum DisplayType {
    List,
    Tasks,
    Table,
}

#[plugin_impl]
impl Plugin for DataView {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.subscribe(&MarkdownEvent::Tag(Tag::CodeBlock(CodeBlockKind::Fenced(
            "/dataview/".to_owned(),
        ))))?;
        Ok(())
    }

    fn process_file(&self, ctx: &Context, file: File) -> Result<File, Error> {
        let mut md = file.data.to_markdown()?;

        let events = md.get_contents_mut();

        for (index, event) in events.clone().iter().enumerate() {
            if let markdown::Event::Start(markdown::cmark::Tag::CodeBlock(
                markdown::cmark::CodeBlockKind::Fenced(tag),
            )) = event
            {
                if (tag.contains("dataview")) {
                    let next = events.get_mut(index + 1);
                    if let Some(markdown::Event::Text(ref mut text)) = next {
                        let res = ctx.query(&text)?;
                        *text = CowStr::Borrowed("Data goes here");
                    }
                }
            }
        }
        let file = FileType::to_file(MarkdownFile {
            events: events.to_vec()
        })?;
        Ok(file)
    }
}
