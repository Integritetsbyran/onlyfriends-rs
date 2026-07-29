use dioxus::prelude::*;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode as QrCodeDep};

fn generate_qr_code_svg_string(data: &str, size: u32) -> Result<String, String> {
    let code = QrCodeDep::with_error_correction_level(data, EcLevel::L)
        .map_err(|e| format!("could not encode qr code: {e}"));

    Ok(code
        .unwrap()
        .render::<svg::Color>()
        .min_dimensions(size, size)
        .quiet_zone(true)
        .build())
}

/// Render a QR code based on data and size
#[component]
pub fn QrCode(data: String, size: u32) -> Element {
    match generate_qr_code_svg_string(&data, size) {
        Ok(svg) => rsx! {
            div { class: "qr-code", dangerous_inner_html: "{svg}"}
        },
        Err(msg) => rsx! {
            div { class: "qr-code qr-code--error", "{msg}" }
        },
    }
}

/// A QR code that opens the app to add a specific friend.
#[component]
pub fn AddFriendQrCode(user_code: String, #[props(default = 220)] size: u32) -> Element {
    let link = format!("onlyfriends://{user_code}");

    rsx! {
        div { class: "add-friend-qr",
            QrCode { data: link.clone(), size }
        }
    }
}
