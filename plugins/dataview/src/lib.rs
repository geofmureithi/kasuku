use std::ops::Deref;

use context::{debug, Context};
use interface::Plugin;
use markdown::{cmark::CowStr, CodeBlockKind, Event, MarkdownEvent, MarkdownFile, Pattern, Tag};
use plugy::macros::plugin_impl;
use serde::Deserialize;
use types::{Error, File};

mod table;

#[derive(Debug, Deserialize, Default)]
pub struct DataView;

pub enum DisplayType {
    List,
    Tasks,
    Table,
}

const CODE_WRAPPER: &str = r##"
<div id="codeContent" class="p-4">
    <textarea id="sqlInput" class="w-full h-40 p-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500" placeholder="Enter your SQL query here...">_CODE_GOES_HERE_</textarea>
    <button id="executeBtn" class="mt-2 px-4 py-2 bg-blue-600 text-white font-semibold rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2">Execute</button>
</div>
"##;

#[plugin_impl]
impl Plugin for DataView {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        debug!("Loading the DataView plugin");
        ctx.subscribe(&MarkdownEvent::Tag(Tag::CodeBlock(CodeBlockKind::Fenced(
            "/dataview/"
                .parse::<Pattern>()
                .map_err(|e| Error::Regex(e.to_string()))?,
        ))))?;
        Ok(())
    }

    fn process_file(&self, ctx: &mut Context, file: File) -> Result<File, Error> {
        let mut md: MarkdownFile = (&file).try_into()?;

        let events = md.deref().clone();

        for (index, event) in events.iter().enumerate() {
            if let markdown::Event::Start(markdown::cmark::Tag::CodeBlock(
                markdown::cmark::CodeBlockKind::Fenced(tag),
            )) = event
            {
                if tag.contains("dataview") {
                    let next = md.get_mut(index + 1);
                    if let Some(e) = next {
                        if let markdown::Event::Text(ref text) = e {
                            let res = ctx.query_raw(text)?;
                            let mut table_html = String::new();
                            let code_html = CODE_WRAPPER.replace("_CODE_GOES_HERE_", text);
                            markdown::cmark::html::push_html(
                                &mut table_html,
                                table::to_events(&res).into_iter(),
                            );
                            // Replace text with a custom html
                            *e = Event::Html(CowStr::Boxed(
                                format!("{code_html}{table_html}").as_str().into(),
                            ));

                            let tag_start = md.get_mut(index).unwrap();
                            *tag_start = Event::Html(CowStr::Borrowed(
                                "<div class=\"border code-block-dataview\">",
                            ));
                            let tag_end = md.get_mut(index + 2).unwrap();
                            *tag_end = Event::Html(CowStr::Borrowed("</div>"))
                        }
                    }
                }
            }
        }
        let file = md.try_into()?;
        Ok(file)
    }
}
