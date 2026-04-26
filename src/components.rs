use crate::icons::IconView;
use dioxus::prelude::*;

#[component]
pub fn CardTitle(name: String, label: String) -> Element {
    rsx! {
        div { class: "card-title",
            IconView { name }
            h2 { "{label}" }
        }
    }
}

#[component]
pub fn SettingRow(name: String, label: String, value: String) -> Element {
    rsx! {
        div { class: "setting-row",
            IconView { name }
            span { class: "setting-label", "{label}" }
            span { class: "setting-value", "{value}" }
        }
    }
}

#[component]
pub fn MiniStat(name: String, label: String, value: String) -> Element {
    rsx! {
        div { class: "mini-stat shadcn-card",
            IconView { name }
            div {
                span { "{label}" }
                strong { "{value}" }
            }
        }
    }
}

#[component]
pub fn ResultRow(name: String, tone: String, label: String, value: String) -> Element {
    let tone_class = format!("result-icon {tone}");
    let row_class = if tone == "red" {
        "result-row result-row-wide"
    } else {
        "result-row"
    };

    rsx! {
        div { class: "{row_class}",
            span { class: "{tone_class}", IconView { name } }
            span { class: "result-label", "{label}" }
            strong { "{value}" }
        }
    }
}
