mod card;

use crate::card::TaskCard;
use context::{debug, Context};
use hirola::prelude::*;
use interface::Plugin;
use markdown::{MarkdownEvent, MarkdownFile};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Subscription {
    plugin: String,
    event: String,
    event_type: String,
    r#type: String,
}

#[plugin_impl]
impl Plugin for Tasks {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        debug!("Initializing Tasks plugin");
        let version = ctx.version();
        debug!("Running on version {version}");
        ctx.subscribe(&TaskEvent::Add)?;
        ctx.subscribe(&MarkdownEvent::TaskList)?;
        let res: Vec<Subscription> =
            ctx.query("SELECT event, event_type, json_extract (data, '$.type') as type, plugin from subscriptions")?;
        debug!("{:?}", res);
        ctx.execute(
            "CREATE TABLE tasks (
                title TEXT NOT NULL,
                completed BOOLEAN NOT NULL,
                source TEXT,
                due DATE
            );",
        )?;
        Ok(())
    }

    fn process_file(&self, ctx: &mut Context, file: File) -> Result<File, Error> {
        debug!("Handling File");
        let path = "&file.path";
        // ctx.execute::<usize>(&format!("DELETE FROM tasks WHERE source = '{path}';"))?;
        let md: MarkdownFile = (&file).try_into()?;
        debug!("got file");
        for (index, event) in md.iter().enumerate() {
            if let markdown::Event::TaskListMarker(state) = event {
                let next = md.get(index + 1);
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
        let tasks: Vec<Task> = ctx.query("Select * from tasks")?;
        let len = tasks.len();
        let node: node::Node = html! {
            <>
            <p>{format!("{len} tasks found")}</p>
            <ul>
                {
                    for task in tasks {
                        html! {
                            <>
                                <li data-completed={task.completed} data-source={task.source.unwrap_or("none".to_string())}>{task.title}</li>
                                <TaskCard/>
                            </>
                        }
                    }
                }
            </ul>
            </>
        };
        node.try_into()
    }
}
