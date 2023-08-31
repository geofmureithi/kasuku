use hirola::signal::Mutable;
use std::fmt;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Debug, Clone)]
pub enum WsMessage {
    ArrayBuffer(js_sys::ArrayBuffer),
    Blob(web_sys::Blob),
    Text(js_sys::JsString),
    Other(wasm_bindgen::JsValue),
}

impl From<JsValue> for WsMessage {
    fn from(value: JsValue) -> Self {
        if let Some(array_buffer) = value.dyn_ref::<js_sys::ArrayBuffer>() {
            WsMessage::ArrayBuffer(array_buffer.clone())
        } else if let Some(blob) = value.dyn_ref::<web_sys::Blob>() {
            WsMessage::Blob(blob.clone())
        } else if let Some(js_string) = value.dyn_ref::<js_sys::JsString>() {
            WsMessage::Text(js_string.clone())
        } else {
            WsMessage::Other(value)
        }
    }
}

impl fmt::Display for WsMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsMessage::ArrayBuffer(_) => write!(f, "ArrayBuffer"),
            WsMessage::Blob(_) => write!(f, "Blob"),
            WsMessage::Text(_) => write!(f, "Text"),
            WsMessage::Other(_) => write!(f, "Other"),
        }
    }
}

pub fn start_websocket(signal: &Mutable<WsMessage>) -> Result<WebSocket, JsValue> {
    // Connect to an echo server
    let ws = WebSocket::new("ws://localhost:3001/ws")?;
    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let cloned_ws = ws.clone();
    let cloned_signal = signal.clone();
    let onmessage_callback =
        Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| cloned_signal.set(e.data().into()));
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let cloned_ws = ws.clone();
    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        console_log!("socket opened");
        match cloned_ws.send_with_str("ping") {
            Ok(_) => console_log!("message successfully sent"),
            Err(err) => console_log!("error sending message: {:?}", err),
        }
        // // send off binary message
        // match cloned_ws.send_with_u8_array(&[0, 1, 2, 3]) {
        //     Ok(_) => console_log!("binary message successfully sent"),
        //     Err(err) => console_log!("error sending message: {:?}", err),
        // }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(ws)
}
