mod ws;

use hirola::prelude::*;
use hirola::signal::{Mutable, SignalExt};
use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Element, Event, HtmlElement, Node, WebSocket};
use ws::WsMessage;

#[derive(Debug, Clone)]
struct WebSocketContext {
    signal: Mutable<WsMessage>,
    inner: WebSocket,
}

// interface MorphDomOptions {
//     getNodeKey?: (node: Node) => any;
//     onBeforeNodeAdded?: (node: Node) => Node;
//     onNodeAdded?: (node: Node) => Node;
//     onBeforeElUpdated?: (fromEl: HTMLElement, toEl: HTMLElement) => boolean;
//     onElUpdated?: (el: HTMLElement) => void;
//     onBeforeNodeDiscarded?: (node: Node) => boolean;
//     onNodeDiscarded?: (node: Node) => void;
//     onBeforeElChildrenUpdated?: (fromEl: HTMLElement, toEl: HTMLElement) => boolean;
//     childrenOnly?: boolean;
// }

// declare function morphdom(
//     fromNode: Node,
//     toNode: Node | string,
//     options?: MorphDomOptions,
// ): void;

#[wasm_bindgen]
extern "C" {
    // TODO: implement MorphDomOptions
    #[wasm_bindgen(js_namespace = window)]
    fn morphdom(from_node: &Node, to_node: &str);
}

fn attach(socket: &WebSocketContext) {
    let clicks = window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all("[kasuku-click]")
        .unwrap();

    for elem in 0..clicks.length() {
        // If the iterator's `next` method throws an error, propagate it
        // up to the caller.
        let elem: Element = clicks.get(elem).unwrap().dyn_into().unwrap();
        let event_name = elem.get_attribute("kasuku-click").unwrap();
        let is_mounted = elem.get_attribute("kasuku-mounted").is_some();
        let ws = socket.clone();
        if !is_mounted {
            let cb = Closure::wrap(Box::new(move |e: Event| {
                ws.inner.send_with_str(&event_name).unwrap();
            }) as Box<dyn FnMut(_)>);
            elem.add_event_listener_with_callback("click", &cb.as_ref().unchecked_ref())
                .unwrap();
            // elem.set_attribute("kasuku-mounted", "true");
            cb.forget();
        }
    }
}

fn home(app: &App<WebSocketContext>) -> Dom {
    let signal = app.state().signal.clone();
    let state = app.state().clone();
    let effect = signal
        .signal_ref(move |v| {
            if let WsMessage::Text(txt) = v {
                let from_node = window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("content")
                    .unwrap()
                    .first_child()
                    .unwrap()
                    .into();

                morphdom(&from_node, &txt.as_string().unwrap());
                attach(&state);
            }
        })
        .to_future();
    attach(app.state());
    html! {
         <>
            <p use:effect>{signal}</p>
            <div id="content">
                <div>"Loading"</div>
            </div>
         </>
    }
}
fn main() {
    let ws_signal = Mutable::new(ws::WsMessage::Other(JsValue::null()));
    let socket = ws::start_websocket(&ws_signal).unwrap();
    let mut app = App::new(WebSocketContext {
        signal: ws_signal,
        inner: socket,
    });
    app.route("/", home);

    app.mount();
    // std::mem::forget(root);
    // routes:
    // / -> home
    // /plugins -> list plugins
    // /markdown/documents/notes.md
    // /tasks/documents/2023-
    // /recipes/
    // /entertainment/
    // /new?type=md&base=/recipes/
    //
    // let app =
    // /tasks/overview              Get overview of overdue, starred, today's, and tomorrow's tasks.
    // /tasks/overdue               Search *.md files for tasks due on previous dates.  (@due(YYYY-MM-DD) format.)
    // /tasks/process               Move lines from $MARKDO_INBOX to other files, one at a time.
    // /tasks/tag "string"          Search *.md files for @string.
    // /tasks/today                 Search *.md files for tasks due today.  (@due(YYYY-MM-DD) format.)
    // /tasks/tomorrow              Search *.md files for tasks due tomorrow.  (@due(YYYY-MM-DD) format.)
    // /tasks/star, starred         Search *.md files for @star.
    // /tasks/summary               Display counts.
    // /tasks/query, q "string"     Search *.md files for string.
    // /tasks/week                  Search *.md files for due dates in the next week.  (@due(YYYY-MM-DD) format.)
}
