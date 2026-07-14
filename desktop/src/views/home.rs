use dioxus_native::prelude::*;
use ui::Hero;

#[component]
pub fn Home() -> Element {
    rsx! {
        Hero {}

    }
}
