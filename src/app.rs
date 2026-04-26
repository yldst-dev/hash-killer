use crate::cache;
use crate::components::{CardTitle, MiniStat, ResultRow};
use crate::duplicate_cleaner;
use crate::hash_algorithm::HashAlgorithm;
use crate::icons::IconView;
use crate::reporting::{format_bytes, progress};
use crate::scan_mode::ScanMode;
use dioxus::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use futures_timer::Delay;
#[cfg(not(target_arch = "wasm32"))]
use futures_util::StreamExt;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(any(not(target_arch = "wasm32"), test))]
use unicode_normalization::UnicodeNormalization;

#[derive(Clone)]
struct InitialAppState {
    paths: Vec<String>,
    status: String,
    report: Option<duplicate_cleaner::CleanReport>,
    cache_limit_mb: u64,
    cache_limit_configured: bool,
    hash_algorithm: HashAlgorithm,
    hash_algorithm_configured: bool,
    scan_mode: ScanMode,
    scan_mode_configured: bool,
}

#[derive(Clone, Copy)]
struct ScanProgressState {
    progress: f64,
    completed: usize,
    total: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DuplicateRelationFilter {
    All,
    SameNameAndSize,
    SameSizeAndHash,
}

impl DuplicateRelationFilter {
    fn all() -> &'static [Self] {
        &[Self::All, Self::SameNameAndSize, Self::SameSizeAndHash]
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "전체",
            Self::SameNameAndSize => "같은 이름+용량",
            Self::SameSizeAndHash => "다른 이름+용량+해시",
        }
    }

    fn matches(self, relation: &duplicate_cleaner::DuplicateRelation) -> bool {
        match self {
            Self::All => true,
            Self::SameNameAndSize => {
                relation.kind == duplicate_cleaner::DuplicateRelationKind::SameNameAndSize
            }
            Self::SameSizeAndHash => {
                relation.kind == duplicate_cleaner::DuplicateRelationKind::SameSizeAndHash
            }
        }
    }

    fn count(self, relations: &[duplicate_cleaner::DuplicateRelation]) -> usize {
        relations
            .iter()
            .filter(|relation| self.matches(relation))
            .count()
    }

    fn button_class(self, selected: Self) -> &'static str {
        if self == selected {
            "shadcn-button shadcn-button-default relation-filter-button"
        } else {
            "shadcn-button shadcn-button-outline relation-filter-button"
        }
    }
}

#[component]
pub fn App() -> Element {
    let initial = initial_app_state();
    let initial_paths = initial.paths.clone();
    let initial_status = initial.status.clone();
    let initial_report = initial.report.clone();
    let initial_cache_limit_mb = initial.cache_limit_mb;
    let initial_cache_limit_configured = initial.cache_limit_configured;
    let initial_hash_algorithm = initial.hash_algorithm;
    let initial_hash_algorithm_configured = initial.hash_algorithm_configured;
    let initial_scan_mode = initial.scan_mode;
    let initial_scan_mode_configured = initial.scan_mode_configured;

    let root_paths = use_signal(move || initial_paths.clone());
    let mut status = use_signal(move || initial_status.clone());
    let report = use_signal(move || initial_report.clone());
    let cache_limit_mb = use_signal(move || initial_cache_limit_mb);
    let cache_limit_configured = use_signal(move || initial_cache_limit_configured);
    let hash_algorithm = use_signal(move || initial_hash_algorithm);
    let hash_algorithm_configured = use_signal(move || initial_hash_algorithm_configured);
    let scan_mode = use_signal(move || initial_scan_mode);
    let scan_mode_configured = use_signal(move || initial_scan_mode_configured);
    let activity_events = use_signal(Vec::<duplicate_cleaner::ActivityEvent>::new);
    let scan_progress = use_signal(|| None::<ScanProgressState>);
    let mut cache_limit_input = use_signal(|| initial_cache_limit_mb.to_string());
    let mut running = use_signal(|| false);
    let mut cache_confirm_open = use_signal(|| false);
    let mut scan_confirm_open = use_signal(|| false);
    let mut cache_settings_open = use_signal(|| false);
    let mut scan_mode_settings_open = use_signal(|| false);
    let mut algorithm_settings_open = use_signal(|| false);
    let mut quarantine_settings_open = use_signal(|| false);
    let mut path_list_open = use_signal(|| false);
    let mut duplicate_relations_open = use_signal(|| false);
    let mut duplicate_relation_filter = use_signal(|| DuplicateRelationFilter::All);
    let mut path_remove_selection = use_signal(Vec::<String>::new);
    let mut dialog_closing = use_signal(|| false);

    let root_values = root_paths();
    let status_value = status();
    let report_value = report();
    let running_value = running();
    let cache_confirm_value = cache_confirm_open();
    let scan_confirm_value = scan_confirm_open();
    let cache_settings_value = cache_settings_open();
    let scan_mode_settings_value = scan_mode_settings_open();
    let algorithm_settings_value = algorithm_settings_open();
    let quarantine_settings_value = quarantine_settings_open();
    let path_list_value = path_list_open();
    let duplicate_relations_value = duplicate_relations_open();
    let duplicate_relation_filter_value = duplicate_relation_filter();
    let activity_values = activity_events();
    let scan_progress_value = scan_progress();
    let activity_list_class = if activity_values.is_empty() {
        "activity-list activity-list-empty"
    } else {
        "activity-list"
    };
    let dialog_class = if dialog_closing() {
        "dialog-backdrop dialog-closing"
    } else {
        "dialog-backdrop"
    };
    let path_remove_values = path_remove_selection();
    let path_remove_count = path_remove_values.len();
    let quarantine_values =
        crate::quarantine::volume_destinations(&root_values).unwrap_or_default();
    let configured_quarantine_count = quarantine_values
        .iter()
        .filter(|destination| destination.configured)
        .count();
    let quarantine_title = if quarantine_values.is_empty() {
        "미지정".to_string()
    } else if configured_quarantine_count == quarantine_values.len() {
        format!("{configured_quarantine_count}개 디스크 지정")
    } else {
        format!(
            "{configured_quarantine_count}/{} 지정",
            quarantine_values.len()
        )
    };
    let cache_limit_value = cache_limit_mb();
    let cache_limit_configured_value = cache_limit_configured();
    let scan_mode_value = scan_mode();
    let scan_mode_configured_value = scan_mode_configured();
    let scan_mode_label = scan_mode_value.label();
    let scan_mode_options = ScanMode::all()
        .iter()
        .map(|mode| {
            (
                *mode,
                mode.label().to_string(),
                mode.description().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let algorithm_value = hash_algorithm();
    let algorithm_configured_value = hash_algorithm_configured();
    let algorithm_label = algorithm_value.label();
    let algorithm_options = HashAlgorithm::all()
        .iter()
        .map(|algorithm| {
            (
                *algorithm,
                algorithm.label().to_string(),
                algorithm.description().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let cache_limit_input_value = cache_limit_input();
    let progress = report_value
        .as_ref()
        .map(|report| progress(Some(report)))
        .or_else(|| scan_progress_value.map(|progress| progress.progress))
        .unwrap_or(0.0);
    let progress_text = format!("{:.1}%", progress * 100.0);
    let progress_width = format!("width: {progress_text};");
    let progress_indicator_class = if running_value && scan_progress_value.is_none() {
        "shadcn-progress-indicator indeterminate"
    } else {
        "shadcn-progress-indicator"
    };
    let processed = report_value
        .as_ref()
        .map(|report| report.scanned_files)
        .or_else(|| scan_progress_value.map(|progress| progress.completed))
        .unwrap_or(0);
    let total = report_value
        .as_ref()
        .map(|report| report.scanned_files)
        .or_else(|| scan_progress_value.map(|progress| progress.total))
        .unwrap_or(0);
    let reclaimed = report_value
        .as_ref()
        .map(|report| format_bytes(report.reclaimed_bytes))
        .unwrap_or_else(|| "0 B".to_string());
    let elapsed = if report_value.is_some() {
        "완료"
    } else {
        "대기"
    };
    let shell_class = "app-shell light";
    let action_label = if running_value {
        "정지"
    } else {
        "검사 시작"
    };
    let has_paths = !root_values.is_empty();
    let quarantine_required = has_paths
        && quarantine_values
            .iter()
            .any(|destination| !destination.configured);
    let action_class = if running_value {
        "shadcn-button shadcn-button-destructive"
    } else if !has_paths || quarantine_required {
        "shadcn-button shadcn-button-outline is-disabled"
    } else {
        "shadcn-button shadcn-button-default"
    };
    let action_hint = if !has_paths {
        "중복 파일을 검사할 디렉터리를 선택하십시오."
    } else if running_value {
        "검사 및 중복 제거를 실행 중입니다."
    } else if quarantine_required {
        "검사 전 모든 디스크의 보관 폴더를 지정하십시오."
    } else {
        "선택한 디렉터리에서 중복 제거를 시작할 수 있습니다."
    };
    let path_title = match root_values.len() {
        0 => "선택된 경로 없음".to_string(),
        1 => root_values[0].clone(),
        count => format!("{count}개 경로 선택"),
    };
    let path_badge_label = match root_values.len() {
        0 => "선택된 경로 없음".to_string(),
        1 => compact_path_label(&root_values[0]),
        count => format!("{count}개 경로 선택"),
    };
    let scanned_files = report_value
        .as_ref()
        .map(|report| report.scanned_files)
        .unwrap_or(0)
        .to_string();
    let candidate_files = report_value
        .as_ref()
        .map(|report| report.candidate_files)
        .unwrap_or(0)
        .to_string();
    let hashed_files = report_value
        .as_ref()
        .map(|report| report.hashed_files)
        .unwrap_or(0)
        .to_string();
    let reused_hashes = report_value
        .as_ref()
        .map(|report| report.reused_hashes)
        .unwrap_or(0)
        .to_string();
    let duplicate_groups = report_value
        .as_ref()
        .map(|report| report.duplicate_groups)
        .unwrap_or(0)
        .to_string();
    let deleted_files = report_value
        .as_ref()
        .map(|report| report.deleted_files)
        .unwrap_or(0)
        .to_string();
    let duplicate_relation_values = report_value
        .as_ref()
        .map(|report| report.duplicate_relations.clone())
        .unwrap_or_default();
    let filtered_duplicate_relation_values = duplicate_relation_values
        .iter()
        .filter(|relation| duplicate_relation_filter_value.matches(relation))
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_relation_filter_options = DuplicateRelationFilter::all()
        .iter()
        .map(|filter| {
            (
                *filter,
                filter.label().to_string(),
                filter.count(&duplicate_relation_values),
            )
        })
        .collect::<Vec<_>>();
    let duplicate_relation_button_disabled = report_value.is_none();
    let duplicate_relation_button_class = if duplicate_relation_button_disabled {
        "shadcn-button shadcn-button-outline duplicate-relations-button"
    } else {
        "shadcn-button shadcn-button-default duplicate-relations-button"
    };
    let duplicate_relation_button_label = "중복 관계 보기";
    let state_badge = if running_value {
        "진행 중"
    } else if report_value.is_some() {
        "완료"
    } else {
        "대기 중"
    };
    let action_disabled = !running_value && (!has_paths || quarantine_required);
    let scan_mode_button_class = setting_button_class(scan_mode_configured_value);
    let algorithm_button_class = setting_button_class(algorithm_configured_value);
    let path_button_class = setting_button_class(has_paths);
    let cache_limit_button_class = setting_button_class(cache_limit_configured_value);
    let quarantine_button_class = setting_button_class(
        !quarantine_values.is_empty() && configured_quarantine_count == quarantine_values.len(),
    );

    use_effect(move || {
        let activity_count = activity_events().len();
        spawn(async move {
            let reset_follow = if activity_count == 0 {
                "element.dataset.followTail = '1';"
            } else {
                ""
            };
            let script = format!(
                r#"
                let element = document.getElementById('activity-list');
                if (element) {{
                    let scrollToTail = () => {{
                        element.dataset.followTail = '1';
                        if (element.autoTailTimer) {{
                            clearTimeout(element.autoTailTimer);
                            element.autoTailTimer = null;
                        }}
                        requestAnimationFrame(() => {{
                            element.scrollTop = element.scrollHeight;
                        }});
                    }};
                    let scheduleTailRestore = () => {{
                        if (element.autoTailTimer) {{
                            clearTimeout(element.autoTailTimer);
                        }}
                        element.autoTailTimer = setTimeout(scrollToTail, 5000);
                    }};
                    if (!element.dataset.scrollReady) {{
                        element.dataset.scrollReady = '1';
                        element.dataset.followTail = '1';
                        element.addEventListener('scroll', () => {{
                            let distance = element.scrollHeight - element.scrollTop - element.clientHeight;
                            if (distance <= 12) {{
                                scrollToTail();
                            }} else {{
                                element.dataset.followTail = '0';
                                scheduleTailRestore();
                            }}
                        }}, {{ passive: true }});
                    }}
                    {reset_follow}
                    if (element.dataset.followTail !== '0') {{
                        scrollToTail();
                    }} else {{
                        scheduleTailRestore();
                    }}
                }}
                "#
            );
            let _ = document::eval(&script).await;
        });
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "startup-loader" }
        main { class: "{shell_class}",
            section { class: "app-card",
                div { class: "path-card shadcn-card",
                    div { class: "path-main",
                        span { class: "path-icon", IconView { name: "folder" } }
                        div {
                            h2 { "{path_title}" }
                        }
                    }
                    button {
                        class: "drop-zone",
                        onclick: move |_| pick_folder(root_paths, status, report),
                        IconView { name: "folder" }
                        span { "폴더 선택" }
                    }
                }

                div { class: "content-grid",
                    section { class: "main-panel",
                        div { class: "shadcn-card settings-card",
                            CardTitle { name: "settings", label: "검사 설정" }
                            div { class: "setting-row setting-action-row",
                                IconView { name: "gauge" }
                                span { class: "setting-label", "검사 모드" }
                                div { class: "setting-action-group",
                                    span { class: "setting-value", "{scan_mode_label}" }
                                    button {
                                        class: "{scan_mode_button_class}",
                                        disabled: running_value,
                                        onclick: move |_| {
                                            dialog_closing.set(false);
                                            scan_mode_settings_open.set(true);
                                        },
                                        "설정"
                                    }
                                }
                            }
                            div { class: "setting-row setting-action-row",
                                IconView { name: "hash" }
                                span { class: "setting-label", "비교 기준" }
                                div { class: "setting-action-group",
                                    span { class: "setting-value", "{algorithm_label}" }
                                    button {
                                        class: "{algorithm_button_class}",
                                        disabled: running_value,
                                        onclick: move |_| {
                                            dialog_closing.set(false);
                                            algorithm_settings_open.set(true);
                                        },
                                        "설정"
                                    }
                                }
                            }
                            div { class: "setting-row setting-action-row",
                                IconView { name: "folder" }
                                span { class: "setting-label", "검사 경로" }
                                div { class: "setting-action-group",
                                    span { class: "setting-value setting-path-value", title: "{path_title}", "{path_badge_label}" }
                                    button {
                                        class: "{path_button_class}",
                                        disabled: running_value,
                                        onclick: move |_| {
                                            path_remove_selection.set(Vec::new());
                                            dialog_closing.set(false);
                                            path_list_open.set(true);
                                        },
                                        "설정"
                                    }
                                }
                            }
                            div { class: "setting-row setting-action-row",
                                IconView { name: "database" }
                                span { class: "setting-label", "캐시 제한" }
                                div { class: "setting-action-group",
                                    span { class: "setting-value", "{cache_limit_value} MB" }
                                    button {
                                        class: "{cache_limit_button_class}",
                                        disabled: running_value,
                                        onclick: move |_| {
                                            cache_limit_input.set(cache_limit_mb().to_string());
                                            dialog_closing.set(false);
                                            cache_settings_open.set(true);
                                        },
                                        "설정"
                                    }
                                }
                            }
                            div { class: "setting-row setting-action-row",
                                IconView { name: "archive" }
                                span { class: "setting-label", "보관 폴더" }
                                div { class: "setting-action-group",
                                    span { class: "setting-value", "{quarantine_title}" }
                                    button {
                                        class: "{quarantine_button_class}",
                                        disabled: running_value,
                                        onclick: move |_| {
                                            dialog_closing.set(false);
                                            quarantine_settings_open.set(true);
                                        },
                                        "설정"
                                    }
                                }
                            }
                        }

                        div { class: "shadcn-card progress-card",
                            CardTitle { name: "activity", label: "진행 상태" }
                            div { class: "progress-box",
                                div { class: "progress-head",
                                    div { class: "progress-title",
                                        if running_value {
                                            span { class: "shadcn-spinner" }
                                        }
                                        strong { {progress_text} }
                                    }
                                    span { "{processed}/{total} 파일 처리" }
                                }
                                div { class: "shadcn-progress",
                                    div {
                                        class: "{progress_indicator_class}",
                                        style: "{progress_width}",
                                    }
                                }
                            }
                            div { class: "mini-stats",
                                MiniStat { name: "file", label: "처리된 파일", value: format!("{processed} / {total}") }
                                MiniStat { name: "timer", label: "총 검사 시간", value: elapsed.to_string() }
                                MiniStat { name: "circle-check", label: "상태", value: state_badge.to_string() }
                            }
                        }
                    }

                    aside { class: "result-card shadcn-card",
                        CardTitle { name: "bar-chart", label: "결과 요약" }
                        div { class: "result-list",
                            ResultRow { name: "search", tone: "blue", label: "스캔", value: scanned_files }
                            ResultRow { name: "list", tone: "purple", label: "후보", value: candidate_files }
                            ResultRow { name: "hash", tone: "green", label: "해시", value: hashed_files }
                            ResultRow { name: "database", tone: "orange", label: "캐시", value: reused_hashes }
                            ResultRow { name: "users", tone: "yellow", label: "그룹", value: duplicate_groups }
                            ResultRow { name: "archive", tone: "emerald", label: "회수", value: reclaimed }
                            ResultRow { name: "trash", tone: "red", label: "삭제", value: deleted_files }
                        }
                        div { class: "duplicate-relations-panel",
                            button {
                                class: "{duplicate_relation_button_class}",
                                disabled: duplicate_relation_button_disabled,
                                onclick: move |_| {
                                    dialog_closing.set(false);
                                    duplicate_relation_filter.set(DuplicateRelationFilter::All);
                                    duplicate_relations_open.set(true);
                                },
                                "{duplicate_relation_button_label}"
                            }
                        }
                    }
                }

                div { class: "activity-stream shadcn-card",
                            CardTitle { name: "list", label: "실시간 작업" }
                            button {
                                class: "setting-action-button activity-export-button",
                                disabled: activity_values.is_empty(),
                                onclick: move |_| export_activity_log(activity_events, status),
                                "로그 저장"
                            }
                            div { id: "activity-list", class: "{activity_list_class}",
                                if activity_values.is_empty() {
                                    div { class: "activity-empty",
                                span { class: "activity-dot idle" }
                                div {
                                    span { class: "activity-stage", "대기 중" }
                                    span { class: "activity-detail", "검사를 시작하면 현재 처리 중인 작업과 파일이 표시됩니다." }
                                }
                            }
                        } else {
                            for event in activity_values.iter().cloned() {
                                div { class: "activity-row",
                                    span { class: "activity-dot" }
                                    div { class: "activity-copy",
                                        div { class: "activity-line",
                                            span { class: "activity-stage", "{event.stage}" }
                                            span { class: "activity-detail", "{event.detail}" }
                                        }
                                        if let Some(path) = event.path {
                                            span { class: "activity-path", title: "{path}", "{path}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                footer { class: "footer-row",
                    div { class: "footer-copy shadcn-card",
                        IconView { name: "info" }
                        p { {action_hint} }
                    }
                    button {
                        class: "shadcn-button shadcn-button-destructive cache-clear-button",
                        disabled: running_value,
                        onclick: move |_| {
                            dialog_closing.set(false);
                            cache_confirm_open.set(true);
                        },
                        "캐시 삭제"
                    }
                    button {
                        class: "{action_class}",
                        disabled: action_disabled,
                        onclick: move |_| {
                            if running() {
                                running.set(false);
                                status.set("사용자가 검사를 중지했습니다.".to_string());
                            } else {
                                dialog_closing.set(false);
                                scan_confirm_open.set(true);
                            }
                        },
                        {action_label}
                    }
                }
                div { class: "status-line", {status_value} }
            }
            if cache_confirm_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "SQLite 캐시 삭제" }
                            p { "SQLite 해시 캐시를 삭제하면 저장된 파일 해시 기록이 지워져 다음 검사에서 캐시를 재사용하지 않고 필요한 해시를 다시 계산합니다. 진행하시겠습니까?" }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| close_dialog(cache_confirm_open, dialog_closing),
                                "아니오"
                            }
                            button {
                                class: "shadcn-button shadcn-button-destructive",
                                onclick: move |_| {
                                    clear_cache(status, report);
                                    close_dialog(cache_confirm_open, dialog_closing);
                                },
                                "예"
                            }
                        }
                    }
                }
            }
            if scan_confirm_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "검사 시작" }
                            p { "검사 및 중복 제거 작업이 시작됩니다. 정말 진행하시겠습니까?" }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| close_dialog(scan_confirm_open, dialog_closing),
                                "아니오"
                            }
                            button {
                                class: "shadcn-button shadcn-button-default",
                                onclick: move |_| {
                                    close_dialog(scan_confirm_open, dialog_closing);
                                    start_scan(ScanSignals {
                                        root_paths,
                                        status,
                                        report,
                                        running,
                                        hash_algorithm,
                                        scan_mode,
                                        activity_events,
                                        scan_progress,
                                    });
                                },
                                "예"
                            }
                        }
                    }
                }
            }
            if path_list_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card path-dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "검사 경로" }
                            p { "삭제할 폴더를 선택한 뒤 선택 삭제를 누르십시오. 서로 다른 디스크의 경로도 함께 검사할 수 있습니다." }
                        }
                        div { class: "path-list",
                            if root_values.is_empty() {
                                div { class: "path-list-empty", "선택된 폴더가 없습니다." }
                            } else {
                                for path in root_values.iter().cloned() {
                                    label { class: "path-list-row",
                                        input {
                                            class: "path-checkbox",
                                            r#type: "checkbox",
                                            checked: path_remove_values.contains(&path),
                                            disabled: running_value,
                                            onchange: move |event| {
                                                toggle_path_selection(
                                                    path_remove_selection,
                                                    path.clone(),
                                                    event.checked(),
                                                );
                                            },
                                        }
                                        span { "{path}" }
                                    }
                                }
                            }
                        }
                        div { class: "dialog-actions path-dialog-actions",
                            span { class: "path-selection-count", "{path_remove_count}개 선택됨" }
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| {
                                    path_remove_selection.set(Vec::new());
                                    close_dialog(path_list_open, dialog_closing);
                                },
                                "닫기"
                            }
                            button {
                                class: "shadcn-button shadcn-button-destructive",
                                disabled: running_value || path_remove_count == 0,
                                onclick: move |_| {
                                    remove_selected_paths(
                                        root_paths,
                                        status,
                                        report,
                                        path_remove_selection,
                                        path_list_open,
                                        dialog_closing,
                                    );
                                },
                                "선택 삭제"
                            }
                        }
                    }
                }
            }
            if cache_settings_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "SQLite 캐시 제한" }
                            p { "캐시 DB가 입력한 용량을 넘으면 오래된 해시 기록부터 삭제합니다. 제한값은 MB 단위이며 16 이상으로 입력하십시오." }
                        }
                        input {
                            class: "dialog-input",
                            value: "{cache_limit_input_value}",
                            inputmode: "numeric",
                            oninput: move |event| cache_limit_input.set(event.value()),
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| close_dialog(cache_settings_open, dialog_closing),
                                "취소"
                            }
                            button {
                                class: "shadcn-button shadcn-button-default",
                                onclick: move |_| {
                                    save_cache_limit(
                                        cache_limit_mb,
                                        cache_limit_configured,
                                        cache_limit_input,
                                        cache_settings_open,
                                        dialog_closing,
                                        status,
                                    );
                                },
                                "저장"
                            }
                        }
                    }
                }
            }
            if scan_mode_settings_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "검사 모드" }
                            p { "파일을 검사 대상으로 선별하는 방식을 선택하십시오. 기본값은 빠른 일반 모드입니다." }
                        }
                        div { class: "path-list",
                            for (option, option_label, option_description) in scan_mode_options.iter().cloned() {
                                label {
                                    class: "path-list-row algorithm-list-row",
                                    onclick: move |_| {
                                        if !running() {
                                            save_scan_mode(
                                                scan_mode,
                                                scan_mode_configured,
                                                status,
                                                option,
                                            );
                                        }
                                    },
                                    input {
                                        class: "path-checkbox",
                                        r#type: "radio",
                                        name: "scan-mode",
                                        checked: option == scan_mode_value,
                                        disabled: running_value,
                                        onchange: move |event| {
                                            if event.checked() {
                                                save_scan_mode(
                                                    scan_mode,
                                                    scan_mode_configured,
                                                    status,
                                                    option,
                                                );
                                            }
                                        },
                                    }
                                    span { "{option_label}" }
                                    span { class: "path-row-muted", "{option_description}" }
                                }
                            }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| {
                                    save_scan_mode(
                                        scan_mode,
                                        scan_mode_configured,
                                        status,
                                        scan_mode_value,
                                    );
                                    close_dialog(scan_mode_settings_open, dialog_closing);
                                },
                                "닫기"
                            }
                        }
                    }
                }
            }
            if algorithm_settings_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "비교 기준" }
                            p { "파일 내용 비교에 사용할 해시 알고리즘을 선택하십시오. 기본값은 BLAKE3입니다." }
                        }
                        div { class: "path-list",
                            for (option, option_label, option_description) in algorithm_options.iter().cloned() {
                                label {
                                    class: "path-list-row algorithm-list-row",
                                    onclick: move |_| {
                                        if !running() {
                                            save_hash_algorithm(
                                                hash_algorithm,
                                                hash_algorithm_configured,
                                                status,
                                                option,
                                            );
                                        }
                                    },
                                    input {
                                        class: "path-checkbox",
                                        r#type: "radio",
                                        name: "hash-algorithm",
                                        checked: option == algorithm_value,
                                        disabled: running_value,
                                        onchange: move |event| {
                                            if event.checked() {
                                                save_hash_algorithm(
                                                    hash_algorithm,
                                                    hash_algorithm_configured,
                                                    status,
                                                    option,
                                                );
                                            }
                                        },
                                    }
                                    span { "{option_label}" }
                                    span { class: "path-row-muted", "{option_description}" }
                                }
                            }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| {
                                    save_hash_algorithm(
                                        hash_algorithm,
                                        hash_algorithm_configured,
                                        status,
                                        algorithm_value,
                                    );
                                    close_dialog(algorithm_settings_open, dialog_closing);
                                },
                                "닫기"
                            }
                        }
                    }
                }
            }
            if quarantine_settings_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card quarantine-dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "보관 폴더" }
                            p { "중복 파일은 삭제하지 않고 같은 디스크의 보관 폴더로 이동합니다. 검사 시작 전 디스크별 보관 폴더를 반드시 지정하십시오." }
                        }
                        div { class: "quarantine-list",
                            if quarantine_values.is_empty() {
                                div { class: "path-list-empty", "검사 경로를 먼저 선택하십시오." }
                            } else {
                                for destination in quarantine_values.iter().cloned() {
                                    div { class: "quarantine-row",
                                        span { class: "path-icon compact", IconView { name: "archive" } }
                                        div { class: "quarantine-copy",
                                            div { class: "quarantine-root-list",
                                                for root_path in destination.root_paths.iter() {
                                                    span { class: "quarantine-root", "{root_path}" }
                                                }
                                            }
                                            span { class: "quarantine-target", "{destination.target_path}" }
                                        }
                                        button {
                                            class: "setting-action-button quarantine-pick-button",
                                            disabled: running_value,
                                            onclick: move |_| {
                                                pick_quarantine_destination(
                                                    root_paths,
                                                    status,
                                                    destination.volume_key.clone(),
                                                );
                                            },
                                            "폴더 선택"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline",
                                onclick: move |_| {
                                    close_dialog(quarantine_settings_open, dialog_closing)
                                },
                                "닫기"
                            }
                        }
                    }
                }
            }
            if duplicate_relations_value {
                div { class: "{dialog_class}",
                    div { class: "dialog-card relation-dialog-card shadcn-card",
                        div { class: "dialog-header",
                            h2 { "중복 관계" }
                            p { "원본 파일과 보관된 중복 파일의 관계를 비교하고 파일 위치를 열 수 있습니다." }
                        }
                        div { class: "relation-filter-bar",
                            for (filter, label, count) in duplicate_relation_filter_options.iter().cloned() {
                                button {
                                    class: "{filter.button_class(duplicate_relation_filter_value)}",
                                    onclick: move |_| duplicate_relation_filter.set(filter),
                                    "{label} {count}"
                                }
                            }
                        }
                        div { class: "relation-list",
                            if duplicate_relation_values.is_empty() {
                                div { class: "path-list-empty", "검사 완료 후 중복 관계가 여기에 표시됩니다." }
                            } else if filtered_duplicate_relation_values.is_empty() {
                                div { class: "path-list-empty", "선택한 필터에 해당하는 중복 관계가 없습니다." }
                            } else {
                                for (index, relation) in filtered_duplicate_relation_values.iter().cloned().enumerate() {
                                    DuplicateRelationCard {
                                        index,
                                        relation,
                                        status,
                                    }
                                }
                            }
                        }
                        div { class: "dialog-actions",
                            button {
                                class: "shadcn-button shadcn-button-outline relation-dialog-action-button",
                                disabled: duplicate_relation_values.is_empty(),
                                onclick: move |_| export_duplicate_relations_log(report, status),
                                "로그 저장"
                            }
                            button {
                                class: "shadcn-button shadcn-button-default relation-dialog-action-button",
                                onclick: move |_| close_dialog(duplicate_relations_open, dialog_closing),
                                "닫기"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DuplicateRelationCard(
    index: usize,
    relation: duplicate_cleaner::DuplicateRelation,
    mut status: Signal<String>,
) -> Element {
    let original_path = relation.original_path.clone();
    let duplicate_path = relation.duplicate_path.clone();
    let current_duplicate_path = relation.current_duplicate_path.clone();
    let relation_number = index + 1;
    let size_label = format_bytes(relation.size);
    let hash_label = compact_hash_label(&relation.hash);

    rsx! {
        div { class: "relation-card shadcn-card",
            div { class: "relation-card-head",
                span { "관계 {relation_number}" }
                div { class: "relation-meta",
                    span { "{size_label}" }
                    span { "{hash_label}" }
                }
            }
            div { class: "relation-compare",
                div { class: "relation-file",
                    span { class: "relation-label", "원본 파일" }
                    span { class: "relation-path", title: "{original_path}", "{original_path}" }
                    button {
                        class: "setting-action-button relation-open-button",
                        onclick: move |_| reveal_file_location(original_path.clone(), status),
                        "위치 열기"
                    }
                }
                div { class: "relation-file",
                    span { class: "relation-label", "중복 파일" }
                    span { class: "relation-path", title: "{duplicate_path}", "{duplicate_path}" }
                    span { class: "relation-muted", title: "{current_duplicate_path}", "보관 위치: {current_duplicate_path}" }
                    button {
                        class: "setting-action-button relation-open-button",
                        onclick: move |_| reveal_file_location(current_duplicate_path.clone(), status),
                        "위치 열기"
                    }
                }
            }
        }
    }
}

fn initial_app_state() -> InitialAppState {
    InitialAppState {
        paths: Vec::new(),
        status: "중복 파일을 검사할 디렉터리를 선택하십시오.".to_string(),
        report: None,
        cache_limit_mb: cache::load_cache_limit_mb().unwrap_or(256),
        cache_limit_configured: cache::load_cache_limit_configured().unwrap_or(false),
        hash_algorithm: HashAlgorithm::from_id(
            &cache::load_hash_algorithm_id().unwrap_or_else(|_| "BLAKE3".to_string()),
        ),
        hash_algorithm_configured: cache::load_hash_algorithm_configured().unwrap_or(false),
        scan_mode: ScanMode::from_id(
            &cache::load_scan_mode_id().unwrap_or_else(|_| "FAST".to_string()),
        ),
        scan_mode_configured: cache::load_scan_mode_configured().unwrap_or(false),
    }
}

fn setting_button_class(configured: bool) -> &'static str {
    if configured {
        "setting-action-button is-configured"
    } else {
        "setting-action-button"
    }
}

fn compact_path_label(path: &str) -> String {
    const MAX_CHARS: usize = 14;

    let normalized = path.replace('\\', "/");

    if normalized.chars().count() <= MAX_CHARS {
        return normalized;
    }

    let prefix = normalized
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();

    format!("{prefix}...")
}

fn compact_hash_label(hash: &str) -> String {
    if hash.chars().count() <= 14 {
        return hash.to_string();
    }

    let prefix = hash.chars().take(14).collect::<String>();
    format!("{prefix}...")
}

#[cfg(not(target_arch = "wasm32"))]
fn reveal_file_location(path: String, mut status: Signal<String>) {
    match reveal_file_path(&path) {
        Ok(()) => status.set("파일 위치를 열었습니다.".to_string()),
        Err(error) => status.set(error),
    }
}

#[cfg(target_arch = "wasm32")]
fn reveal_file_location(_path: String, mut status: Signal<String>) {
    status.set("웹 미리보기에서는 파일 위치를 열 수 없습니다.".to_string());
}

#[cfg(target_os = "macos")]
fn reveal_file_path(path: &str) -> Result<(), String> {
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Finder에서 파일 위치를 열지 못했습니다.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn reveal_file_path(path: &str) -> Result<(), String> {
    let status = std::process::Command::new("explorer")
        .arg(format!("/select,{path}"))
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("탐색기에서 파일 위치를 열지 못했습니다.".to_string())
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    not(target_os = "windows")
))]
fn reveal_file_path(path: &str) -> Result<(), String> {
    let path = PathBuf::from(path);
    let open_path = if path.is_file() {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.clone())
    } else {
        path
    };
    let status = std::process::Command::new("xdg-open")
        .arg(open_path)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("파일 위치를 열지 못했습니다.".to_string())
    }
}

fn close_dialog(mut open: Signal<bool>, mut dialog_closing: Signal<bool>) {
    dialog_closing.set(true);

    spawn(async move {
        #[cfg(not(target_arch = "wasm32"))]
        Delay::new(Duration::from_millis(160)).await;
        #[cfg(target_arch = "wasm32")]
        TimeoutFuture::new(160).await;
        open.set(false);
        dialog_closing.set(false);
    });
}

fn save_scan_mode(
    mut scan_mode: Signal<ScanMode>,
    mut scan_mode_configured: Signal<bool>,
    mut status: Signal<String>,
    selected: ScanMode,
) {
    match cache::save_scan_mode_id(selected.id()) {
        Ok(()) => {
            scan_mode.set(selected);
            scan_mode_configured.set(true);
            status.set(format!("검사 모드를 {}로 저장했습니다.", selected.label()));
        }
        Err(error) => status.set(error),
    }
}

fn save_hash_algorithm(
    mut hash_algorithm: Signal<HashAlgorithm>,
    mut hash_algorithm_configured: Signal<bool>,
    mut status: Signal<String>,
    selected: HashAlgorithm,
) {
    match cache::save_hash_algorithm_id(selected.id()) {
        Ok(()) => {
            hash_algorithm.set(selected);
            hash_algorithm_configured.set(true);
            status.set(format!("비교 기준을 {}로 저장했습니다.", selected.label()));
        }
        Err(error) => status.set(error),
    }
}

fn save_cache_limit(
    mut cache_limit_mb: Signal<u64>,
    mut cache_limit_configured: Signal<bool>,
    cache_limit_input: Signal<String>,
    cache_settings_open: Signal<bool>,
    dialog_closing: Signal<bool>,
    mut status: Signal<String>,
) {
    let Ok(value) = cache_limit_input().trim().parse::<u64>() else {
        status.set("캐시 제한은 숫자로 입력하십시오.".to_string());
        return;
    };

    if value < 16 {
        status.set("캐시 제한은 16 MB 이상으로 입력하십시오.".to_string());
        return;
    }

    match cache::save_cache_limit_mb(value) {
        Ok(removed) => {
            cache_limit_mb.set(value);
            cache_limit_configured.set(true);
            close_dialog(cache_settings_open, dialog_closing);

            if removed == 0 {
                status.set(format!("SQLite 캐시 제한을 {value} MB로 저장했습니다."));
            } else {
                status.set(format!(
                    "SQLite 캐시 제한을 {value} MB로 저장하고 오래된 캐시 {removed}개를 삭제했습니다."
                ));
            }
        }
        Err(error) => status.set(error),
    }
}

fn clear_cache(
    mut status: Signal<String>,
    mut report: Signal<Option<duplicate_cleaner::CleanReport>>,
) {
    match cache::clear_cache() {
        Ok(0) => status.set("삭제할 SQLite 캐시가 없습니다.".to_string()),
        Ok(_) => {
            report.set(None);
            status.set("SQLite 캐시를 삭제했습니다.".to_string());
        }
        Err(error) => status.set(error),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn export_activity_log(
    activity_events: Signal<Vec<duplicate_cleaner::ActivityEvent>>,
    status: Signal<String>,
) {
    let events = activity_events();

    if events.is_empty() {
        let mut status = status;
        status.set("저장할 실시간 작업 로그가 없습니다.".to_string());
        return;
    }

    write_log_file(
        "hash-killer-activity-log.txt",
        format_activity_log(&events),
        status,
    );
}

#[cfg(target_arch = "wasm32")]
fn export_activity_log(
    _activity_events: Signal<Vec<duplicate_cleaner::ActivityEvent>>,
    mut status: Signal<String>,
) {
    status.set("웹 미리보기에서는 로그 파일을 저장할 수 없습니다.".to_string());
}

#[cfg(not(target_arch = "wasm32"))]
fn export_duplicate_relations_log(
    report: Signal<Option<duplicate_cleaner::CleanReport>>,
    status: Signal<String>,
) {
    let report_value = report();
    let Some(report) = report_value.as_ref() else {
        let mut status = status;
        status.set("저장할 중복 관계 로그가 없습니다.".to_string());
        return;
    };

    if report.duplicate_relations.is_empty() {
        let mut status = status;
        status.set("저장할 중복 관계 로그가 없습니다.".to_string());
        return;
    }

    write_log_file(
        "hash-killer-duplicate-relations-log.txt",
        format_duplicate_relations_log(report),
        status,
    );
}

#[cfg(target_arch = "wasm32")]
fn export_duplicate_relations_log(
    _report: Signal<Option<duplicate_cleaner::CleanReport>>,
    mut status: Signal<String>,
) {
    status.set("웹 미리보기에서는 로그 파일을 저장할 수 없습니다.".to_string());
}

#[cfg(not(target_arch = "wasm32"))]
fn write_log_file(file_name: &str, content: String, mut status: Signal<String>) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(file_name)
        .add_filter("텍스트", &["txt"])
        .save_file()
    else {
        return;
    };

    match std::fs::write(&path, normalize_log_content(&content)) {
        Ok(()) => status.set(format!("로그 파일을 저장했습니다: {}", path.display())),
        Err(error) => status.set(format!("로그 파일을 저장하지 못했습니다: {error}")),
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn normalize_log_content(content: &str) -> String {
    content.nfc().collect()
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn format_activity_log(events: &[duplicate_cleaner::ActivityEvent]) -> String {
    let mut lines = Vec::new();
    lines.push("hash-killer 실시간 작업 로그".to_string());
    lines.push(format!("총 작업: {}", events.len()));
    lines.push(String::new());

    for (index, event) in events.iter().enumerate() {
        lines.push(format!("{}. {}", index + 1, event.stage));
        lines.push(format!("상세: {}", redact_log_text(&event.detail)));

        if let Some(path) = &event.path {
            lines.push(format!("경로: {}", redact_path(path)));
        }

        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn format_duplicate_relations_log(report: &duplicate_cleaner::CleanReport) -> String {
    let same_name_count = report
        .duplicate_relations
        .iter()
        .filter(|relation| {
            relation.kind == duplicate_cleaner::DuplicateRelationKind::SameNameAndSize
        })
        .count();
    let same_size_hash_count = report
        .duplicate_relations
        .len()
        .saturating_sub(same_name_count);
    let mut lines = Vec::new();
    lines.push("hash-killer 중복 관계 로그".to_string());
    lines.push(format!("전체 관계: {}", report.duplicate_relations.len()));
    lines.push(format!("같은 이름+용량: {same_name_count}"));
    lines.push(format!("다른 이름+용량+해시: {same_size_hash_count}"));
    lines.push(format!(
        "회수 용량: {}",
        format_bytes(report.reclaimed_bytes)
    ));
    lines.push(String::new());

    for (index, relation) in report.duplicate_relations.iter().enumerate() {
        lines.push(format!("관계 {}", index + 1));
        lines.push(format!(
            "분류: {}",
            duplicate_relation_kind_label(relation.kind)
        ));
        lines.push(format!(
            "원본 파일: {}",
            redact_path(&relation.original_path)
        ));
        lines.push(format!(
            "중복 파일: {}",
            redact_path(&relation.duplicate_path)
        ));
        lines.push(format!(
            "보관 위치: {}",
            redact_path(&relation.current_duplicate_path)
        ));
        lines.push(format!("용량: {}", format_bytes(relation.size)));
        lines.push(format!("해시: {}", compact_hash_label(&relation.hash)));
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn redact_log_text(value: &str) -> String {
    if value.contains('/') || value.contains('\\') {
        redact_path(value)
    } else {
        value.to_string()
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn redact_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let file_name = Path::new(&normalized)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            normalized
                .rsplit('/')
                .find(|part| !part.is_empty())
                .map(str::to_string)
        });

    match file_name {
        Some(file_name) => format!(".../{file_name}"),
        None => "[경로 마스킹됨]".to_string(),
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn duplicate_relation_kind_label(kind: duplicate_cleaner::DuplicateRelationKind) -> &'static str {
    match kind {
        duplicate_cleaner::DuplicateRelationKind::SameNameAndSize => "같은 이름+용량",
        duplicate_cleaner::DuplicateRelationKind::SameSizeAndHash => "다른 이름+용량+해시",
    }
}

fn toggle_path_selection(
    mut path_remove_selection: Signal<Vec<String>>,
    path: String,
    selected: bool,
) {
    let mut selected_paths = path_remove_selection();

    if selected {
        if !selected_paths.contains(&path) {
            selected_paths.push(path);
        }
    } else {
        selected_paths.retain(|selected_path| selected_path != &path);
    }

    path_remove_selection.set(selected_paths);
}

#[derive(Clone, Copy)]
struct ScanSignals {
    root_paths: Signal<Vec<String>>,
    status: Signal<String>,
    report: Signal<Option<duplicate_cleaner::CleanReport>>,
    running: Signal<bool>,
    hash_algorithm: Signal<HashAlgorithm>,
    scan_mode: Signal<ScanMode>,
    activity_events: Signal<Vec<duplicate_cleaner::ActivityEvent>>,
    scan_progress: Signal<Option<ScanProgressState>>,
}

fn remove_selected_paths(
    mut root_paths: Signal<Vec<String>>,
    mut status: Signal<String>,
    mut report: Signal<Option<duplicate_cleaner::CleanReport>>,
    mut path_remove_selection: Signal<Vec<String>>,
    path_list_open: Signal<bool>,
    dialog_closing: Signal<bool>,
) {
    let selected_paths = path_remove_selection();

    if selected_paths.is_empty() {
        return;
    }

    let mut paths = root_paths();
    let previous_count = paths.len();
    paths.retain(|path| !selected_paths.contains(path));
    let removed_count = previous_count.saturating_sub(paths.len());
    let count = paths.len();

    if removed_count == 0 {
        path_remove_selection.set(Vec::new());
        return;
    }

    root_paths.set(paths);
    report.set(None);
    path_remove_selection.set(Vec::new());

    if count == 0 {
        close_dialog(path_list_open, dialog_closing);
        status.set("중복 파일을 검사할 디렉터리를 선택하십시오.".to_string());
    } else {
        status.set(format!(
            "{removed_count}개 디렉터리를 제거했습니다. {count}개 디렉터리가 검사 목록에 남아 있습니다."
        ));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_quarantine_destination(
    root_paths: Signal<Vec<String>>,
    mut status: Signal<String>,
    volume_key: String,
) {
    let roots = root_paths();

    if roots.is_empty() {
        status.set("검사 경로를 먼저 선택하십시오.".to_string());
        return;
    }

    let destinations = match crate::quarantine::volume_destinations(&roots) {
        Ok(destinations) => destinations,
        Err(error) => {
            status.set(error);
            return;
        }
    };
    let Some(destination) = destinations
        .into_iter()
        .find(|destination| destination.volume_key == volume_key)
    else {
        status.set("해당 디스크의 검사 경로를 찾을 수 없습니다.".to_string());
        return;
    };
    let start_directory = if destination.configured {
        PathBuf::from(destination.target_path)
    } else {
        PathBuf::from(destination.root_path)
    };
    let dialog = if start_directory.exists() {
        rfd::FileDialog::new().set_directory(start_directory)
    } else {
        rfd::FileDialog::new()
    };

    if let Some(folder) = dialog.pick_folder() {
        match crate::quarantine::save_destination(&volume_key, &folder) {
            Ok(()) => status.set("보관 폴더를 저장했습니다.".to_string()),
            Err(error) => status.set(error),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn pick_quarantine_destination(
    _root_paths: Signal<Vec<String>>,
    mut status: Signal<String>,
    _volume_key: String,
) {
    status.set("웹 미리보기에서는 보관 폴더를 선택할 수 없습니다.".to_string());
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_folder(
    mut root_paths: Signal<Vec<String>>,
    mut status: Signal<String>,
    mut report: Signal<Option<duplicate_cleaner::CleanReport>>,
) {
    if let Some(folders) = rfd::FileDialog::new().pick_folders() {
        let mut paths = root_paths();
        let previous_count = paths.len();

        for path in folders
            .into_iter()
            .map(|folder| folder.display().to_string())
        {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }

        let count = paths.len();
        root_paths.set(paths);
        if count == previous_count {
            status.set("이미 등록된 디렉터리입니다.".to_string());
        } else {
            status.set(format!(
                "{count}개 디렉터리를 선택했습니다. 보관 폴더를 지정한 뒤 검사를 시작할 수 있습니다."
            ));
        }
        report.set(None);
    }
}

#[cfg(target_arch = "wasm32")]
fn pick_folder(
    mut root_paths: Signal<Vec<String>>,
    mut status: Signal<String>,
    mut report: Signal<Option<duplicate_cleaner::CleanReport>>,
) {
    root_paths.set(Vec::new());
    status.set("웹 미리보기에서는 직접 경로를 입력하십시오.".to_string());
    report.set(None);
}

fn reset_completed_scan_settings(mut root_paths: Signal<Vec<String>>, mut status: Signal<String>) {
    root_paths.set(Vec::new());

    match crate::quarantine::clear_destinations() {
        Ok(_) => {
            status.set("완료되었습니다. 검사 경로와 보관 폴더 설정을 초기화했습니다.".to_string())
        }
        Err(error) => status.set(format!(
            "완료되었지만 보관 폴더 설정을 초기화하지 못했습니다: {error}"
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn start_scan(signals: ScanSignals) {
    let ScanSignals {
        root_paths,
        mut status,
        mut report,
        mut running,
        hash_algorithm,
        scan_mode,
        mut activity_events,
        mut scan_progress,
    } = signals;
    let roots = root_paths();

    if roots.is_empty() {
        status.set("중복 파일을 검사할 디렉터리를 선택하십시오.".to_string());
        return;
    }

    match crate::quarantine::volume_destinations(&roots) {
        Ok(destinations)
            if destinations
                .iter()
                .all(|destination| destination.configured) => {}
        Ok(_) => {
            status.set("모든 디스크의 보관 폴더를 먼저 지정하십시오.".to_string());
            return;
        }
        Err(error) => {
            status.set(error);
            return;
        }
    }

    running.set(true);
    report.set(None);
    activity_events.set(Vec::new());
    scan_progress.set(None);
    status.set("검사 및 중복 제거를 실행 중입니다.".to_string());

    let (sender, receiver) = futures_channel::oneshot::channel();
    let (activity_sender, mut activity_receiver) =
        futures_channel::mpsc::unbounded::<duplicate_cleaner::ActivityEvent>();
    let algorithm = hash_algorithm();
    let mode = scan_mode();

    std::thread::spawn(move || {
        let result = duplicate_cleaner::clean_duplicate_paths_with_progress(
            roots.into_iter().map(PathBuf::from).collect(),
            algorithm,
            mode,
            move |event| {
                let _ = activity_sender.unbounded_send(event);
            },
        );
        let _ = sender.send(result);
    });

    spawn(async move {
        while let Some(event) = activity_receiver.next().await {
            push_activity_event(activity_events, scan_progress, event);
        }
    });

    spawn(async move {
        let result = receiver
            .await
            .unwrap_or_else(|_| Err("검사 작업을 완료하지 못했습니다.".to_string()));

        running.set(false);

        match result {
            Ok(clean_report) => {
                push_activity_event(
                    activity_events,
                    scan_progress,
                    duplicate_cleaner::ActivityEvent::new("완료", "검사가 완료되었습니다.", None),
                );
                reset_completed_scan_settings(root_paths, status);
                report.set(Some(clean_report));
            }
            Err(error) => {
                push_activity_event(
                    activity_events,
                    scan_progress,
                    duplicate_cleaner::ActivityEvent::new("오류", error.clone(), None),
                );
                status.set(error);
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn start_scan(signals: ScanSignals) {
    let ScanSignals {
        root_paths,
        mut status,
        mut report,
        mut running,
        hash_algorithm,
        scan_mode,
        mut activity_events,
        mut scan_progress,
    } = signals;
    let roots = root_paths();

    if roots.is_empty() {
        status.set("중복 파일을 검사할 디렉터리를 선택하십시오.".to_string());
        return;
    }

    running.set(true);
    report.set(None);
    activity_events.set(Vec::new());
    scan_progress.set(None);
    status.set("검사 및 중복 제거를 실행 중입니다.".to_string());

    spawn(async move {
        let result = duplicate_cleaner::clean_duplicate_paths_with_progress(
            roots.into_iter().map(PathBuf::from).collect(),
            hash_algorithm(),
            scan_mode(),
            |event| push_activity_event(activity_events, scan_progress, event),
        );
        running.set(false);

        match result {
            Ok(clean_report) => {
                push_activity_event(
                    activity_events,
                    scan_progress,
                    duplicate_cleaner::ActivityEvent::new("완료", "검사가 완료되었습니다.", None),
                );
                reset_completed_scan_settings(root_paths, status);
                report.set(Some(clean_report));
            }
            Err(error) => {
                push_activity_event(
                    activity_events,
                    scan_progress,
                    duplicate_cleaner::ActivityEvent::new("오류", error.clone(), None),
                );
                status.set(error);
            }
        }
    });
}

fn push_activity_event(
    mut activity_events: Signal<Vec<duplicate_cleaner::ActivityEvent>>,
    mut scan_progress: Signal<Option<ScanProgressState>>,
    event: duplicate_cleaner::ActivityEvent,
) {
    if let (Some(progress), Some(completed), Some(total)) =
        (event.progress, event.completed, event.total)
    {
        scan_progress.set(Some(ScanProgressState {
            progress,
            completed,
            total,
        }));
    }

    let mut events = activity_events();
    events.push(event);

    if events.len() > 200 {
        let remove_count = events.len() - 200;
        events.drain(0..remove_count);
    }

    activity_events.set(events);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_log_redacts_paths() {
        let events = vec![duplicate_cleaner::ActivityEvent::new(
            "해시 계산",
            "/Users/example/Documents/private/file.txt",
            Some("/Users/example/Documents/private/file.txt".to_string()),
        )];

        let log = format_activity_log(&events);

        assert!(log.contains(".../file.txt"));
        assert!(!log.contains("/Users/example"));
        assert!(!log.contains("private"));
    }

    #[test]
    fn duplicate_relation_log_redacts_paths_and_hash() {
        let report = duplicate_cleaner::CleanReport {
            reclaimed_bytes: 10,
            duplicate_relations: vec![duplicate_cleaner::DuplicateRelation {
                original_path: "/Users/example/Documents/private/original.txt".to_string(),
                duplicate_path: "/Users/example/Documents/private/duplicate.txt".to_string(),
                current_duplicate_path: "/Users/example/Archive/private/duplicate.txt".to_string(),
                size: 10,
                hash: "0123456789abcdef0123456789abcdef".to_string(),
                kind: duplicate_cleaner::DuplicateRelationKind::SameSizeAndHash,
            }],
            ..duplicate_cleaner::CleanReport::default()
        };

        let log = format_duplicate_relations_log(&report);

        assert!(log.contains(".../original.txt"));
        assert!(log.contains(".../duplicate.txt"));
        assert!(log.contains("0123456789abcd..."));
        assert!(!log.contains("/Users/example"));
        assert!(!log.contains("private"));
        assert!(!log.contains("0123456789abcdef0123456789abcdef"));
    }
}
