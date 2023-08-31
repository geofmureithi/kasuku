use hirola::prelude::*;
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{FileEvent, HandleError, Plugin, RenderEvent};

#[derive(Debug, Deserialize, Default)]
pub struct Tasks;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum TaskEvent {
    #[serde(alias = "TaskEvent::Add")]
    Add,
    Completed(u32), // Line Number
}

#[plugin_impl]
impl Plugin for Tasks {
    fn handle(&self, _msg: FileEvent) -> Result<bool, HandleError> {
        Ok(false)
    }
    fn render(&self, evt: RenderEvent) -> String {
        match evt.path.as_str() {
            "/" => html! {
                <div kasuku-click={"emit(TaskEvent::Add)"}>
                    "Markdown Goes here"
                </div>
            },
            _ => html! { <div>"Not Found"</div> },
        }
        .inner_html()
    }
}
