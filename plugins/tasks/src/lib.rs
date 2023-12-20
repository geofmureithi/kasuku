mod card;

use card::TaskCard;
use context::Context;
use hirola::prelude::*;
use interface::Plugin;
use markdown::{AsMarkdown, CodeBlockKind, LinkType, MarkdownEvent, Tag};
use node::emit;
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{Error, Event, File, PluginEvent, Rsx, Task};

#[derive(Debug, Deserialize, Default)]
pub struct Tasks;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum TaskEvent {
    #[serde(rename = "TaskEvent::Add")]
    Add,
    #[serde(rename = "TaskEvent::Completed")]
    Completed,
    #[serde(rename = "TaskEvent::Delete")]
    Delete,
}

impl PluginEvent for TaskEvent {
    type Plugin = Tasks;
}

#[derive(Debug, Deserialize, Serialize)]
enum Noop {}

#[plugin_impl]
impl Plugin for Tasks {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.subscribe(&TaskEvent::Add).unwrap();
        ctx.subscribe(&MarkdownEvent::Tag(Tag::Link(
            Some(LinkType::Email),
            Some("/gmail.com/".to_owned()),
            None,
        )))
        .unwrap();
        ctx.subscribe(&MarkdownEvent::Tag(Tag::CodeBlock(CodeBlockKind::Fenced(
            "rust".to_owned(),
        ))))
        .unwrap();
        let _res: Vec<_> = ctx.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                text TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                due INTEGER,
                meta TEXT
            );",
        )?;
        Ok(())
    }

    fn process_file(&self, ctx: &Context, file: File) -> Result<File, Error> {
        let md = file.data.to_markdown()?;

        let events = md.get_contents();

        for (index, event) in events.iter().enumerate() {
            if let markdown::Event::TaskListMarker(_state) = event {
                let next = events.get(index + 1);
                if let Some(markdown::Event::Text(text)) = next {
                    ctx.add_task(&Task::new(text.to_string()))?;
                }
            }
        }

        Ok(file)
    }

    fn on_event(&self, _ctx: &Context, _ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, _ev: Event) -> Result<Rsx, Error> {
        let _res: Vec<_> = ctx.query("Select * from tasks")?;
        html! {
            <>
                <task-card on:click=emit(&TaskEvent::Add)/>
                <script src="https://unpkg.com/mathlive"></script>
                <TaskCard />
            </>
        }
        .try_into()
    }
}
