mod card;

use card::TaskCard;
use hirola::prelude::*;
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{
    emit, CodeBlockKind, Context, CowStr, Error, Event, File, LinkType, MarkdownEvent,
    MarkdownFile, Plugin, PluginEvent, PulldownEvent, Rsx, Tag, Task,
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
                text TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                due INTEGER,
                meta TEXT
            );",
        );
        Ok(())
    }

    fn process_file(&self, ctx: &Context, file: File) -> Result<File, Error> {
        match file {
            File::Markdown(ref file) => {
                let events: Vec<PulldownEvent<'_>> = file.get_contents();

                for (index, event) in events.iter().enumerate() {
                    match event {
                        PulldownEvent::TaskListMarker(state) => {
                            let next = events.get(index + 1);
                            if let Some(ev) = next {
                                match ev {
                                    PulldownEvent::Text(text) => {
                                        ctx.add_task(&Task::new(text.to_string()))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => todo!(),
        }
        Ok(file)
    }

    fn on_event(&self, _ctx: &Context, _ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, _ev: Event) -> Result<Rsx, Error> {
        let res = ctx.query("Select * from tasks");
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

fn replace_text(original_text: &str) -> String {
    original_text.replace("expected", "concluded")
}
