use crate::context::{self, ModalContent};
use dioxus::prelude::*;

#[component]
pub fn Modal() -> Element {
    let mut modal = context::use_modal();
    rsx! {
        if let Some(ModalContent(content)) = modal() {
            // TODO: dismiss modal by swiping down
            div { class: "modal-bg", onclick: move |_| modal.set(None),
                div {
                    class: "modal",
                    // TEMP: close the modal even when tapping on the content.
                    // This is to avoid accidental cases where a modal fills up the entire screen,
                    // leaving no room to tap outside the modal.
                    //onclick: |e| e.stop_propagation(),
                    {content}
                }
            }
        }
    }
}
