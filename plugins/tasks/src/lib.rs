mod card;

use context::Context;
use hirola::prelude::*;
use interface::Plugin;
use macros::FromSelect;
use markdown::{AsMarkdown, MarkdownEvent};
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{Error, Event, File, PluginEvent, Rsx};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromSelect)]
pub struct Task {
    title: String,
    completed: bool,
    source: Option<String>,
}

impl Task {
    pub fn new(text: String) -> Self {
        Self {
            title: text,
            completed: false,
            source: None,
        }
    }
}

#[plugin_impl]
impl Plugin for Tasks {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.subscribe(&TaskEvent::Add)?;
        ctx.subscribe(&MarkdownEvent::TaskList)?;
        ctx.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                title TEXT NOT NULL,
                completed BOOLEAN NOT NULL,
                source TEXT,
                due DATE,
            );",
        )?;
        Ok(())
    }

    fn process_file(&self, ctx: &mut Context, file: File) -> Result<File, Error> {
        let path = &file.path;
        ctx.execute(&format!("DELETE FROM tasks WHERE source = '{path}';"))?;
        let md = file.data.to_markdown()?;

        let events = md.get_contents();

        for (index, event) in events.iter().enumerate() {
            if let markdown::Event::TaskListMarker(state) = event {
                let next = events.get(index + 1);
                if let Some(markdown::Event::Text(text)) = next {
                    ctx.execute(&format!("INSERT INTO tasks(title, completed, source) VALUES('{text}', {state}, '{path}');"))?;
                }
            }
        }

        Ok(file)
    }

    fn on_event(&self, _ctx: &Context, _ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, _ev: Event) -> Result<Rsx, Error> {
        let tasks: Vec<Task> = ctx
            .query("Select * from tasks")?
            .first()
            .cloned()
            .map(|payload| payload.try_into().unwrap())
            .unwrap();
        let len = tasks.len();
        let node: node::Node = html! {
            <>
            <p>{format!("{len} tasks found")}</p>
            <ul>
                {
                    for task in tasks {
                        html! {
                            <li data-completed={task.completed} data-source={task.source.unwrap_or("none".to_string())}>{task.title}</li>
                        }
                    }
                }
            </ul>
            </>
        };
        node.try_into()
    }
}
