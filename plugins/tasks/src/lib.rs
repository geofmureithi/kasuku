mod card;

use card::TaskCard;
use hirola::prelude::*;
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{
    emit, CodeBlockKind, Context, Error, Event, File, LinkType, MarkdownEvent, Plugin, PluginEvent,
    PulldownEvent, Rsx,
};

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

#[plugin_impl]
impl Plugin for Tasks {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.subscribe(&TaskEvent::Add);
        ctx.subscribe(&MarkdownEvent::Tag(types::Tag::Link(
            LinkType::Email,
            "/gmail.com/".to_owned(),
            "".to_owned(),
        )));
        ctx.subscribe(&MarkdownEvent::Tag(types::Tag::CodeBlock(
            CodeBlockKind::Fenced("rust".to_owned()),
        )));
        ctx.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                due INTEGER,
                meta TEXT
            );",
        );
        Ok(())
    }

    fn process_file(&self, _ctx: &Context, file: File) -> Result<File, Error> {
        match &file {
            File::Markdown(file) => {
                let _events: Vec<PulldownEvent<'_>> = file.get_contents();
            }
            _ => todo!(),
        }
        Ok(file)
    }

    fn on_event(&self, _ctx: &Context, _ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, _ev: Event) -> Result<Rsx, Error> {
        let res = ctx.query("Select id, name from Tasks");
        html! {
            <>
                <task-card on:click=emit(&TaskEvent::Add)/>
                <span>{res}</span>
                <script src="https://unpkg.com/mathlive"></script>
                <TaskCard />
            </>
        }
        .try_into()
    }
}
