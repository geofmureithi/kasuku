use hirola::prelude::*;
use plugy::macros::plugin_impl;
use serde::{Deserialize, Serialize};
use types::{FileEvent, HandleError, Plugin, RenderEvent, emit};

#[derive(Debug, Deserialize, Default)]
pub struct Tasks;

#[derive(Debug, Deserialize, Serialize)]
pub enum TaskEvent {
    Add,
    Completed(u32), // Line Number
    Delete { id: u32 },
}



#[plugin_impl]
impl Plugin for Tasks {
    fn handle(&self, _msg: FileEvent) -> Result<bool, HandleError> {
        Ok(false)
    }
    fn render(&self, _evt: RenderEvent) -> String {
        html! {
            <div onclick={emit(&TaskEvent::Delete { id: 50 })}>
                "Markdown Goes here"
            </div>
        }
        .inner_html()
    }
}
