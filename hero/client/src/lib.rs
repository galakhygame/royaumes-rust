mod start;

use crate::start::Start;
use custom_elements::CustomElement;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;
use weblog::console_info;
use yew::AppHandle;

#[derive(Default)]
struct ComponentWrapper {
    content: Option<AppHandle<Start>>,
}

impl CustomElement for ComponentWrapper {
    fn inject_children(&mut self, this: &HtmlElement) {
        self.content = Some(yew::start_app_in_element::<Start>(this.clone().into()));
    }

    fn shadow() -> bool {
        false
    }

    fn connected_callback(&mut self, _this: &HtmlElement) {
        console_info!("connected");
    }

    fn disconnected_callback(&mut self, _this: &HtmlElement) {
        console_info!("disconnected");
    }
}

#[wasm_bindgen]
pub fn run() {
    ComponentWrapper::define("hero-start");
}
