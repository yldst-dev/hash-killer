use dioxus::prelude::*;

#[component]
pub fn IconView(name: String) -> Element {
    match name.as_str() {
        "info" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "10" }
                path { d: "M12 16v-4" }
                path { d: "M12 8h.01" }
            }
        },
        "plus" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M5 12h14" }
                path { d: "M12 5v14" }
            }
        },
        "folder" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.6-.8L9.5 4A2 2 0 0 0 7.9 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
            }
        },
        "settings" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.51a2 2 0 0 1 1-1.72l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2Z" }
                circle { cx: "12", cy: "12", r: "3" }
            }
        },
        "gauge" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "m12 14 4-4" }
                path { d: "M3.34 19a10 10 0 1 1 17.32 0" }
            }
        },
        "hash" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                line { x1: "4", x2: "20", y1: "9", y2: "9" }
                line { x1: "4", x2: "20", y1: "15", y2: "15" }
                line { x1: "10", x2: "8", y1: "3", y2: "21" }
                line { x1: "16", x2: "14", y1: "3", y2: "21" }
            }
        },
        "activity" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M22 12h-4l-3 9L9 3l-3 9H2" }
            }
        },
        "file" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z" }
                path { d: "M14 2v6h6" }
                path { d: "M16 13H8" }
                path { d: "M16 17H8" }
            }
        },
        "status" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M6 3v12" }
                path { d: "M18 9v12" }
                path { d: "M6 15l6-6 6 6" }
            }
        },
        "circle-check" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "10" }
                path { d: "m9 12 2 2 4-4" }
            }
        },
        "chart" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M21 12c.552 0 1.005-.45.95-.998a10 10 0 1 0-8.953 10.949c.55.055 1.003-.398 1.003-.95v-8a1 1 0 0 1 1-1Z" }
                path { d: "M21.21 15.89A10 10 0 0 1 15 21.21" }
            }
        },
        "bar-chart" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M3 3v18h18" }
                path { d: "M7 16V9" }
                path { d: "M12 16V5" }
                path { d: "M17 16v-3" }
            }
        },
        "list" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M8 6h13" }
                path { d: "M8 12h13" }
                path { d: "M8 18h13" }
                path { d: "M3 6h.01" }
                path { d: "M3 12h.01" }
                path { d: "M3 18h.01" }
            }
        },
        "database" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                ellipse { cx: "12", cy: "5", rx: "9", ry: "3" }
                path { d: "M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" }
                path { d: "M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" }
            }
        },
        "users" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
                circle { cx: "9", cy: "7", r: "4" }
                path { d: "M22 21v-2a4 4 0 0 0-3-3.87" }
                path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
            }
        },
        "trash" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M3 6h18" }
                path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
                path { d: "M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" }
                path { d: "M10 11v6" }
                path { d: "M14 11v6" }
            }
        },
        "archive" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M21 8v13H3V8" }
                path { d: "M1 3h22v5H1z" }
                path { d: "M10 12h4" }
            }
        },
        "shield" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.68 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1Z" }
            }
        },
        "search" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                circle { cx: "11", cy: "11", r: "8" }
                path { d: "m21 21-4.3-4.3" }
            }
        },
        "timer" => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M10 2h4" }
                path { d: "M12 14v-4" }
                path { d: "M4 13a8 8 0 1 0 16 0 8 8 0 0 0-16 0" }
            }
        },
        _ => rsx! {
            svg { class: "icon", width: "18", height: "18", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "4" }
                path { d: "M12 2v2" }
                path { d: "M12 20v2" }
                path { d: "m4.93 4.93 1.41 1.41" }
                path { d: "m17.66 17.66 1.41 1.41" }
                path { d: "M2 12h2" }
                path { d: "M20 12h2" }
                path { d: "m6.34 17.66-1.41 1.41" }
                path { d: "m19.07 4.93-1.41 1.41" }
            }
        },
    }
}
