#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use gpui::{
    actions, div, point, prelude::*, px, relative, rgb, rgba, size, svg, App, Application,
    AssetSource, Bounds, ClickEvent, Context, CursorStyle, Div, FocusHandle, IntoElement,
    KeyBinding, KeyDownEvent, MouseButton, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Timer, TitlebarOptions, Window, WindowBounds,
    WindowControlArea, WindowOptions,
};
use hash_killer::duplicate_cleaner::{ActivityEvent, CleanReport, DuplicateRelation};
use hash_killer::hash_algorithm::HashAlgorithm;
use hash_killer::scan_mode::ScanMode;
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

actions!(hash_killer, [QuitApp, CloseAppWindow]);

#[cfg(target_os = "macos")]
const APP_ICON_PNG: &[u8] = include_bytes!("../resources/hashkiller.png");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    None,
    CacheConfirm,
    ScanConfirm,
    Md5Confirm,
    ScanMode,
    Algorithm,
    Paths,
    CacheLimit,
    Quarantine,
    Relations,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RelationFilter {
    All,
    SameNameAndSize,
    SameSizeAndHash,
}

enum ScanEvent {
    Activity(ActivityEvent),
    Completed(CleanReport),
    Failed(String),
    Cancelled,
}

struct ActiveScan {
    cancelled: Arc<AtomicBool>,
    receiver: mpsc::Receiver<ScanEvent>,
}

struct HashKillerApp {
    paths: Vec<String>,
    cache_limit_mb: u64,
    cache_limit_input: String,
    cache_limit_focus: FocusHandle,
    cache_limit_configured: bool,
    algorithm: HashAlgorithm,
    algorithm_configured: bool,
    scan_mode: ScanMode,
    scan_mode_configured: bool,
    destinations: Vec<hash_killer::quarantine::VolumeDestination>,
    report: Option<CleanReport>,
    activity_events: Vec<ActivityEvent>,
    activity_log_events: Vec<ActivityEvent>,
    activity_scroll: ScrollHandle,
    scan_progress: Option<(f64, usize, usize)>,
    scan_started_at: Option<Instant>,
    last_scan_elapsed: Option<Duration>,
    status_message: String,
    error_message: String,
    running: bool,
    pending: bool,
    pending_shutdown: bool,
    close_handler_registered: bool,
    dialog: Dialog,
    selected_paths: Vec<String>,
    relation_filter: RelationFilter,
    active_scan: Option<ActiveScan>,
}

impl HashKillerApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            paths: Vec::new(),
            cache_limit_mb: 256,
            cache_limit_input: "256".to_string(),
            cache_limit_focus: cx.focus_handle(),
            cache_limit_configured: false,
            algorithm: HashAlgorithm::Blake3,
            algorithm_configured: false,
            scan_mode: ScanMode::Fast,
            scan_mode_configured: false,
            destinations: Vec::new(),
            report: None,
            activity_events: Vec::new(),
            activity_log_events: Vec::new(),
            activity_scroll: ScrollHandle::new(),
            scan_progress: None,
            scan_started_at: None,
            last_scan_elapsed: None,
            status_message: "중복 파일을 검사할 디렉터리를 선택하십시오.".to_string(),
            error_message: String::new(),
            running: false,
            pending: false,
            pending_shutdown: false,
            close_handler_registered: false,
            dialog: Dialog::None,
            selected_paths: Vec::new(),
            relation_filter: RelationFilter::All,
            active_scan: None,
        };
        app.load_settings();
        app
    }

    fn load_settings(&mut self) {
        match (
            hash_killer::cache::load_cache_limit_mb(),
            hash_killer::cache::load_cache_limit_configured(),
            hash_killer::cache::load_hash_algorithm_id(),
            hash_killer::cache::load_hash_algorithm_configured(),
            hash_killer::cache::load_scan_mode_id(),
            hash_killer::cache::load_scan_mode_configured(),
        ) {
            (
                Ok(limit),
                Ok(limit_configured),
                Ok(algorithm),
                Ok(algorithm_configured),
                Ok(scan_mode),
                Ok(scan_mode_configured),
            ) => {
                self.cache_limit_mb = limit;
                self.cache_limit_input = limit.to_string();
                self.cache_limit_configured = limit_configured;
                self.algorithm = HashAlgorithm::from_id(&algorithm);
                self.algorithm_configured = algorithm_configured;
                self.scan_mode = ScanMode::from_id(&scan_mode);
                self.scan_mode_configured = scan_mode_configured;
            }
            _ => {
                self.set_failure("설정을 불러오지 못했습니다.");
            }
        }
    }

    fn path_title(&self) -> String {
        match self.paths.len() {
            0 => "선택된 경로 없음".to_string(),
            1 => self.paths[0].clone(),
            count => format!("{count}개 경로 선택"),
        }
    }

    fn path_badge(&self) -> String {
        match self.paths.len() {
            0 => "선택된 경로 없음".to_string(),
            1 => compact_path(&self.paths[0], 24),
            count => format!("{count}개 경로 선택"),
        }
    }

    fn configured_destination_count(&self) -> usize {
        self.destinations
            .iter()
            .filter(|destination| destination.configured)
            .count()
    }

    fn quarantine_configured(&self) -> bool {
        !self.destinations.is_empty()
            && self.configured_destination_count() == self.destinations.len()
    }

    fn quarantine_title(&self) -> String {
        if self.destinations.is_empty() {
            return "미지정".to_string();
        }
        let configured = self.configured_destination_count();
        if configured == self.destinations.len() {
            format!("{configured}개 폴더 지정")
        } else {
            format!("{configured}/{} 지정", self.destinations.len())
        }
    }

    fn scan_block_reason(&self) -> Option<&'static str> {
        if self.paths.is_empty() {
            Some("중복 파일을 검사할 디렉터리를 선택하십시오.")
        } else if !self.scan_mode_configured {
            Some("검사 모드를 설정하십시오.")
        } else if !self.algorithm_configured {
            Some("비교 기준을 설정하십시오.")
        } else if !self.cache_limit_configured {
            Some("캐시 제한을 설정하십시오.")
        } else if !self.quarantine_configured() {
            Some("보관 폴더를 설정하십시오.")
        } else {
            None
        }
    }

    fn can_start_scan(&self) -> bool {
        self.scan_block_reason().is_none()
    }

    fn progress_value(&self) -> f64 {
        if self.report.is_some() && !self.running {
            return 1.0;
        }
        self.scan_progress
            .map(|progress| progress.0)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    }

    fn processed_total(&self) -> (usize, usize) {
        if let Some(report) = &self.report {
            return (report.scanned_files, report.scanned_files);
        }
        self.scan_progress
            .map(|progress| (progress.1, progress.2))
            .unwrap_or((0, 0))
    }

    fn duplicate_relations(&self) -> &[DuplicateRelation] {
        self.report
            .as_ref()
            .map(|report| report.duplicate_relations.as_slice())
            .unwrap_or(&[])
    }

    fn filtered_relations(&self) -> Vec<&DuplicateRelation> {
        self.duplicate_relations()
            .iter()
            .filter(|relation| match self.relation_filter {
                RelationFilter::All => true,
                RelationFilter::SameNameAndSize => is_same_name_relation(&relation.kind),
                RelationFilter::SameSizeAndHash => !is_same_name_relation(&relation.kind),
            })
            .collect()
    }

    fn relation_filter_count(&self, filter: RelationFilter) -> usize {
        self.duplicate_relations()
            .iter()
            .filter(|relation| match filter {
                RelationFilter::All => true,
                RelationFilter::SameNameAndSize => is_same_name_relation(&relation.kind),
                RelationFilter::SameSizeAndHash => !is_same_name_relation(&relation.kind),
            })
            .count()
    }

    fn status_label(&self) -> &'static str {
        if self.running {
            "진행 중"
        } else if self.report.is_some() {
            "완료"
        } else if self.error_message.is_empty() {
            "대기 중"
        } else {
            "오류"
        }
    }

    fn elapsed_label(&self) -> String {
        if let Some(started_at) = self.scan_started_at {
            return format_duration(started_at.elapsed());
        }
        self.last_scan_elapsed
            .map(format_duration)
            .unwrap_or_else(|| "-".to_string())
    }

    fn action_hint(&self) -> String {
        if self.running {
            "검사 및 중복 제거를 실행 중입니다.".to_string()
        } else if let Some(reason) = self.scan_block_reason() {
            reason.to_string()
        } else {
            "선택한 디렉터리에서 중복 제거를 시작할 수 있습니다.".to_string()
        }
    }

    fn choose_folders(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        let folders = rfd::FileDialog::new().pick_folders().unwrap_or_default();
        if folders.is_empty() {
            return;
        }
        let completed = self.report.is_some();
        let previous_count = if completed { 0 } else { self.paths.len() };
        let mut unique = if completed {
            Vec::new()
        } else {
            self.paths.clone()
        };
        for folder in folders {
            let path = folder.display().to_string();
            if !unique.iter().any(|existing| existing == &path) {
                unique.push(path);
            }
        }
        let added_count = unique.len().saturating_sub(previous_count);
        if added_count == 0 {
            self.status_message =
                "선택한 디렉터리는 이미 검사 목록에 포함되어 있습니다.".to_string();
            self.refresh_destinations();
            cx.notify();
            return;
        }
        self.paths = unique;
        self.report = None;
        self.status_message = if completed {
            format!(
                "{}개 디렉터리를 새 검사 목록으로 선택했습니다.",
                self.paths.len()
            )
        } else {
            format!("{added_count}개 디렉터리를 추가했습니다. 현재 {}개 디렉터리가 검사 목록에 있습니다.", self.paths.len())
        };
        self.refresh_destinations();
        cx.notify();
    }

    fn refresh_destinations(&mut self) {
        self.destinations = if self.paths.is_empty() {
            Vec::new()
        } else {
            hash_killer::quarantine::volume_destinations(&self.paths).unwrap_or_default()
        };
    }

    fn update_scan_mode(&mut self, mode: ScanMode, cx: &mut Context<Self>) {
        match hash_killer::cache::save_scan_mode_id(mode.id()) {
            Ok(()) => {
                self.scan_mode = mode;
                self.scan_mode_configured = true;
                self.status_message = format!("검사 모드를 {}로 저장했습니다.", mode.label());
            }
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn update_algorithm(&mut self, algorithm: HashAlgorithm, cx: &mut Context<Self>) {
        if algorithm.requires_warning() && self.algorithm != algorithm {
            self.dialog = Dialog::Md5Confirm;
            cx.notify();
            return;
        }
        self.save_algorithm(algorithm, cx);
    }

    fn save_algorithm(&mut self, algorithm: HashAlgorithm, cx: &mut Context<Self>) {
        match hash_killer::cache::save_hash_algorithm_id(algorithm.id()) {
            Ok(()) => {
                self.algorithm = algorithm;
                self.algorithm_configured = true;
                self.status_message = format!("비교 기준을 {}로 저장했습니다.", algorithm.label());
                self.dialog = Dialog::None;
            }
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn update_cache_limit(&mut self, value: u64, cx: &mut Context<Self>) {
        match hash_killer::cache::save_cache_limit_mb(value) {
            Ok(pruned) => {
                self.cache_limit_mb = value;
                self.cache_limit_input = value.to_string();
                self.cache_limit_configured = true;
                self.status_message = if pruned > 0 {
                    format!("SQLite 캐시 제한을 {value} MB로 저장하고 오래된 해시 {pruned}개를 정리했습니다.")
                } else {
                    format!("SQLite 캐시 제한을 {value} MB로 저장했습니다.")
                };
                self.dialog = Dialog::None;
            }
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn save_cache_limit_input(&mut self, cx: &mut Context<Self>) {
        let trimmed = self.cache_limit_input.trim();
        match trimmed.parse::<u64>().ok().filter(|value| *value > 0) {
            Some(value) => self.update_cache_limit(value, cx),
            None => {
                self.set_failure("캐시 제한은 1 이상의 숫자로 입력하십시오.");
                cx.notify();
            }
        }
    }

    fn handle_cache_limit_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.secondary() && keystroke.key == "a" {
            self.cache_limit_input.clear();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                self.cache_limit_input.pop();
                cx.stop_propagation();
                cx.notify();
            }
            "delete" => {
                self.cache_limit_input.clear();
                cx.stop_propagation();
                cx.notify();
            }
            "enter" => {
                self.save_cache_limit_input(cx);
                cx.stop_propagation();
            }
            _ => {
                if keystroke.modifiers.modified() {
                    return;
                }
                let Some(chars) = keystroke.key_char.as_ref() else {
                    return;
                };
                let digits = chars
                    .chars()
                    .filter(|ch| ch.is_ascii_digit())
                    .collect::<String>();
                if digits.is_empty() {
                    return;
                }
                self.cache_limit_input.push_str(&digits);
                if self.cache_limit_input.len() > 9 {
                    self.cache_limit_input.truncate(9);
                }
                while self.cache_limit_input.len() > 1 && self.cache_limit_input.starts_with('0') {
                    self.cache_limit_input.remove(0);
                }
                cx.stop_propagation();
                cx.notify();
            }
        }
    }

    fn choose_quarantine(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        let Some(destination) = self.destinations.get(index).cloned() else {
            return;
        };
        let dialog = destination
            .configured
            .then(|| PathBuf::from(&destination.target_path))
            .filter(|path| path.exists())
            .map(|path| rfd::FileDialog::new().set_directory(path))
            .unwrap_or_else(|| {
                rfd::FileDialog::new().set_directory(PathBuf::from(&destination.root_path))
            });
        let Some(folder) = dialog.pick_folder() else {
            return;
        };
        match hash_killer::quarantine::save_destination(&destination.volume_key, &folder) {
            Ok(()) => {
                self.refresh_destinations();
                self.status_message = "보관 폴더를 저장했습니다.".to_string();
            }
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn clear_cache(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        match hash_killer::cache::clear_cache() {
            Ok(removed) => {
                self.report = None;
                self.status_message = if removed > 0 {
                    format!("SQLite 캐시 파일 {removed}개를 삭제했습니다.")
                } else {
                    "삭제할 SQLite 캐시 파일이 없습니다.".to_string()
                };
                self.dialog = Dialog::None;
            }
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn start_scan(&mut self, cx: &mut Context<Self>) {
        if self.running {
            self.cancel_scan(cx);
            return;
        }
        if let Some(reason) = self.scan_block_reason() {
            self.status_message = reason.to_string();
            cx.notify();
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        let roots = self.paths.iter().map(PathBuf::from).collect::<Vec<_>>();
        let algorithm = self.algorithm;
        let scan_mode = self.scan_mode;
        let thread_cancelled = cancelled.clone();
        let result_cancelled = cancelled.clone();
        std::thread::spawn(move || {
            let progress_sender = sender.clone();
            let result =
                hash_killer::duplicate_cleaner::clean_duplicate_paths_with_progress_and_cancel(
                    roots,
                    algorithm,
                    scan_mode,
                    move |event| {
                        let _ = progress_sender.send(ScanEvent::Activity(event));
                    },
                    move || thread_cancelled.load(Ordering::Relaxed),
                );
            match result {
                Ok(report) => {
                    let _ = sender.send(ScanEvent::Completed(report));
                }
                Err(_) if result_cancelled.load(Ordering::Relaxed) => {
                    let _ = sender.send(ScanEvent::Cancelled);
                }
                Err(message) => {
                    let _ = sender.send(ScanEvent::Failed(message));
                }
            }
        });
        self.running = true;
        self.pending = false;
        self.dialog = Dialog::None;
        self.report = None;
        self.error_message.clear();
        self.activity_events.clear();
        self.activity_log_events.clear();
        self.activity_scroll.set_offset(point(px(0.), px(0.)));
        self.scan_progress = None;
        self.scan_started_at = Some(Instant::now());
        self.last_scan_elapsed = None;
        self.status_message = "검사 및 중복 제거를 실행 중입니다.".to_string();
        self.active_scan = Some(ActiveScan {
            cancelled,
            receiver,
        });
        self.watch_scan(cx);
        cx.notify();
    }

    fn cancel_scan(&mut self, cx: &mut Context<Self>) {
        if let Some(scan) = &self.active_scan {
            scan.cancelled.store(true, Ordering::Relaxed);
            self.status_message = "검사 중지를 요청했습니다.".to_string();
        } else {
            self.running = false;
            self.status_message = "사용자가 검사를 중지했습니다.".to_string();
        }
        cx.notify();
    }

    fn watch_scan(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |weak, cx| loop {
            Timer::after(Duration::from_millis(80)).await;
            let keep_running = weak
                .update(cx, |app, cx| {
                    app.drain_scan_events(cx);
                    cx.notify();
                    app.running
                })
                .unwrap_or(false);
            if !keep_running {
                break;
            }
        })
        .detach();
    }

    fn drain_scan_events(&mut self, cx: &mut Context<Self>) {
        let Some(scan) = self.active_scan.as_ref() else {
            return;
        };
        let mut events = Vec::new();
        while let Ok(event) = scan.receiver.try_recv() {
            events.push(event);
        }
        for event in events {
            match event {
                ScanEvent::Activity(activity) => self.push_activity(activity),
                ScanEvent::Completed(report) => {
                    self.last_scan_elapsed = self
                        .scan_started_at
                        .take()
                        .map(|started_at| started_at.elapsed());
                    self.push_activity(ActivityEvent::with_progress(
                        "완료",
                        "검사가 완료되었습니다.",
                        None,
                        1.0,
                        report.scanned_files,
                        report.scanned_files,
                    ));
                    self.report = Some(report);
                    self.running = false;
                    self.active_scan = None;
                    self.status_message = "검사가 완료되었습니다.".to_string();
                    if self.pending_shutdown {
                        cx.quit();
                    }
                }
                ScanEvent::Failed(message) => {
                    self.last_scan_elapsed = self
                        .scan_started_at
                        .take()
                        .map(|started_at| started_at.elapsed());
                    self.push_activity(ActivityEvent::new("오류", message.clone(), None));
                    self.running = false;
                    self.active_scan = None;
                    self.error_message = message.clone();
                    self.status_message = message;
                    if self.pending_shutdown {
                        cx.quit();
                    }
                }
                ScanEvent::Cancelled => {
                    self.last_scan_elapsed = self
                        .scan_started_at
                        .take()
                        .map(|started_at| started_at.elapsed());
                    self.push_activity(ActivityEvent::new(
                        "중지",
                        "사용자가 검사를 중지했습니다.",
                        None,
                    ));
                    self.running = false;
                    self.active_scan = None;
                    self.status_message = "사용자가 검사를 중지했습니다.".to_string();
                    if self.pending_shutdown {
                        cx.quit();
                    }
                }
            }
        }
    }

    fn push_activity(&mut self, event: ActivityEvent) {
        if let (Some(progress), Some(completed), Some(total)) =
            (event.progress, event.completed, event.total)
        {
            self.scan_progress = Some((progress, completed, total));
        }
        self.activity_log_events.push(event);
        let keep_from = self.activity_log_events.len().saturating_sub(200);
        self.activity_events = self.activity_log_events[keep_from..].to_vec();
        self.activity_scroll.scroll_to_bottom();
    }

    fn remove_selected_paths(&mut self, cx: &mut Context<Self>) {
        if self.selected_paths.is_empty() || self.running {
            return;
        }
        self.paths
            .retain(|path| !self.selected_paths.iter().any(|selected| selected == path));
        self.selected_paths.clear();
        self.report = None;
        self.status_message = if self.paths.is_empty() {
            "중복 파일을 검사할 디렉터리를 선택하십시오.".to_string()
        } else {
            format!(
                "{}개 디렉터리가 검사 목록에 남아 있습니다.",
                self.paths.len()
            )
        };
        self.refresh_destinations();
        if self.paths.is_empty() {
            self.dialog = Dialog::None;
        }
        cx.notify();
    }

    fn toggle_selected_path(&mut self, path: String, cx: &mut Context<Self>) {
        if self.selected_paths.iter().any(|selected| selected == &path) {
            self.selected_paths.retain(|selected| selected != &path);
        } else {
            self.selected_paths.push(path);
        }
        cx.notify();
    }

    fn save_activity_log(&mut self, cx: &mut Context<Self>) {
        if self.activity_log_events.is_empty() {
            self.status_message = "저장할 실시간 작업 로그가 없습니다.".to_string();
            cx.notify();
            return;
        }
        self.save_text(
            "hash-killer-activity.log",
            format_activity_log(&self.activity_log_events),
            "실시간 작업 로그를 저장했습니다.",
            cx,
        );
    }

    fn save_relations_log(&mut self, cx: &mut Context<Self>) {
        if self.duplicate_relations().is_empty() {
            self.status_message = "저장할 중복 관계 로그가 없습니다.".to_string();
            cx.notify();
            return;
        }
        self.save_text(
            "hash-killer-duplicates.log",
            format_duplicate_relations_log(self.duplicate_relations()),
            "중복 관계 로그를 저장했습니다.",
            cx,
        );
    }

    fn save_text(
        &mut self,
        suggested_name: &str,
        contents: String,
        success_message: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(suggested_name)
            .save_file()
        else {
            return;
        };
        match std::fs::write(path, contents) {
            Ok(()) => self.status_message = success_message.to_string(),
            Err(error) => self.set_failure(error.to_string()),
        }
        cx.notify();
    }

    fn open_file(&mut self, path: String, cx: &mut Context<Self>) {
        match reveal_file_path(PathBuf::from(path)) {
            Ok(()) => self.status_message = "파일 위치 열기를 요청했습니다.".to_string(),
            Err(error) => self.set_failure(error),
        }
        cx.notify();
    }

    fn open_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        if self.running && dialog == Dialog::CacheConfirm {
            return;
        }
        if dialog == Dialog::Paths {
            self.selected_paths.clear();
        }
        if dialog == Dialog::Relations {
            self.relation_filter = RelationFilter::All;
        }
        if dialog == Dialog::CacheLimit {
            self.cache_limit_input = self.cache_limit_mb.to_string();
        }
        self.dialog = dialog;
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = Dialog::None;
        cx.notify();
    }

    fn request_shutdown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running {
            self.pending_shutdown = true;
            if let Some(scan) = &self.active_scan {
                scan.cancelled.store(true, Ordering::Relaxed);
            }
            self.status_message = "진행 중인 검사를 중지한 뒤 종료합니다.".to_string();
            cx.notify();
            return;
        }

        window.remove_window();
        cx.quit();
    }

    fn handle_window_should_close(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.running {
            self.pending_shutdown = true;
            if let Some(scan) = &self.active_scan {
                scan.cancelled.store(true, Ordering::Relaxed);
            }
            self.status_message = "진행 중인 검사를 중지한 뒤 종료합니다.".to_string();
            cx.notify();
            return false;
        }

        true
    }

    fn set_failure(&mut self, error: impl Into<String>) {
        let message = error.into();
        self.error_message = message.clone();
        self.status_message = message;
    }

    fn root_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .bg(color::bg_app())
            .text_color(color::text_primary())
            .text_size(px(12.))
            .on_action(cx.listener(|view, _: &QuitApp, window, cx| {
                view.request_shutdown(window, cx);
            }))
            .on_action(cx.listener(|view, _: &CloseAppWindow, window, cx| {
                view.request_shutdown(window, cx);
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(self.window_titlebar(window))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .px(px(13.))
                            .pb(px(15.))
                            .child(self.top_panel(cx))
                            .child(self.dashboard(cx))
                            .child(self.action_bar(cx)),
                    ),
            )
            .when(self.dialog != Dialog::None, |root| {
                root.child(self.modal_overlay(cx))
            })
    }

    fn window_titlebar(&self, window: &mut Window) -> impl IntoElement {
        let maximize_label = if window.is_maximized() { "▢" } else { "□" };

        div()
            .id("window-titlebar")
            .h(px(42.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_end()
            .when(cfg!(target_os = "windows"), |bar| {
                bar.window_control_area(WindowControlArea::Drag).child(
                    div()
                        .h_full()
                        .flex()
                        .items_center()
                        .child(window_control_button(
                            "window-minimize",
                            "_",
                            WindowControlArea::Min,
                            false,
                        ))
                        .child(window_control_button(
                            "window-maximize",
                            maximize_label,
                            WindowControlArea::Max,
                            false,
                        ))
                        .child(window_control_button(
                            "window-close",
                            "X",
                            WindowControlArea::Close,
                            true,
                        )),
                )
            })
    }

    fn top_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel()
            .h(px(68.))
            .flex()
            .items_center()
            .justify_between()
            .px(px(14.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(13.))
                    .child(icon_box("DIR", px(36.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(self.path_title()),
                            )
                            .child(
                                div()
                                    .text_color(color::text_secondary())
                                    .child("검사할 폴더를 선택해주세요."),
                            ),
                    ),
            )
            .child(button(
                "choose-folders",
                "폴더 선택",
                ButtonKind::Folder,
                self.running,
                cx.listener(|view, _, _, cx| view.choose_folders(cx)),
            ))
    }

    fn dashboard(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .h(px(240.))
                            .child(self.settings_panel(cx))
                            .child(self.progress_panel()),
                    )
                    .child(self.activity_panel(cx)),
            )
            .child(self.summary_panel(cx))
    }

    fn section_header(
        &self,
        icon: &'static str,
        title: &'static str,
        extra: Option<impl IntoElement>,
    ) -> Div {
        let header = div()
            .h(px(36.))
            .flex()
            .items_center()
            .justify_between()
            .px(px(14.))
            .bg(color::bg_header())
            .border_b_1()
            .border_color(color::border_default())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon_box(icon, px(22.)))
                    .child(div().font_weight(gpui::FontWeight::BOLD).child(title)),
            );
        if let Some(extra) = extra {
            header.child(extra)
        } else {
            header
        }
    }

    fn settings_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel()
            .flex_1()
            .min_w(px(271.5))
            .overflow_hidden()
            .child(self.section_header("CFG", "검사 설정", None::<Div>))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(self.setting_row(
                        "MODE",
                        "검사 모드",
                        self.scan_mode.label(),
                        !self.scan_mode_configured,
                        Dialog::ScanMode,
                        cx,
                    ))
                    .child(self.setting_row(
                        "HASH",
                        "비교 기준",
                        self.algorithm.label(),
                        !self.algorithm_configured,
                        Dialog::Algorithm,
                        cx,
                    ))
                    .child(self.setting_row(
                        "PATH",
                        "검사 경로",
                        self.path_badge(),
                        self.paths.is_empty(),
                        Dialog::Paths,
                        cx,
                    ))
                    .child(self.setting_row(
                        "DB",
                        "캐시 제한",
                        format!("{} MB", self.cache_limit_mb),
                        !self.cache_limit_configured,
                        Dialog::CacheLimit,
                        cx,
                    ))
                    .child(self.setting_row(
                        "KEEP",
                        "보관 폴더",
                        self.quarantine_title(),
                        !self.quarantine_configured(),
                        Dialog::Quarantine,
                        cx,
                    )),
            )
    }

    fn setting_row(
        &self,
        icon: &'static str,
        label: &'static str,
        value: impl Into<String>,
        unconfigured: bool,
        dialog: Dialog,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let disabled = self.running;
        div()
            .h(px(34.))
            .flex()
            .items_center()
            .gap_2()
            .px(px(11.))
            .border_b_1()
            .border_color(color::border_subtle())
            .child(icon_box(icon, px(24.)))
            .child(
                div()
                    .w(px(62.))
                    .text_color(color::text_secondary())
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_align(gpui::TextAlign::Right)
                    .child(value.into()),
            )
            .child(button(
                format!("setting-{label}"),
                "설정",
                if unconfigured {
                    ButtonKind::Configured
                } else {
                    ButtonKind::Small
                },
                disabled,
                cx.listener(move |view, _, _, cx| view.open_dialog(dialog, cx)),
            ))
    }

    fn progress_panel(&self) -> impl IntoElement {
        let (processed, total) = self.processed_total();
        let progress = self.progress_value();
        let elapsed = self.elapsed_label();
        let visible_progress = progress.max(if self.running { 0.16 } else { 0.015 }) as f32;
        panel()
            .flex_1()
            .min_w(px(278.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.section_header("RUN", "진행 상태", None::<Div>))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .px(px(22.))
                    .py(px(14.))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .flex()
                            .items_center()
                            .child(progress_meta(
                                "FILE",
                                "처리된 파일",
                                format!("{processed} / {total}"),
                                true,
                            ))
                            .child(progress_meta("TIME", "총 검사 시간", elapsed, true))
                            .child(progress_meta("STAT", "상태", self.status_label(), false)),
                    )
                    .child(
                        div().mt(px(14.)).mb(px(4.)).child(
                            div()
                                .h(px(6.))
                                .w_full()
                                .rounded_full()
                                .overflow_hidden()
                                .bg(color::bg_active())
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(visible_progress))
                                        .rounded_full()
                                        .bg(color::accent()),
                                ),
                        ),
                    ),
            )
    }

    fn activity_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.section_header(
                "LOG",
                "실시간 작업",
                Some(button(
                    "save-activity",
                    "로그 저장",
                    ButtonKind::Small,
                    self.activity_log_events.is_empty(),
                    cx.listener(|view, _, _, cx| view.save_activity_log(cx)),
                )),
            ))
            .child(
                div()
                    .id("activity-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .track_scroll(&self.activity_scroll)
                    .when(self.activity_events.is_empty(), |list| {
                        list.child(empty_activity_state())
                    })
                    .children(
                        self.activity_events
                            .iter()
                            .enumerate()
                            .map(|(index, event)| {
                                div()
                                    .id(("activity", index))
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .px_4()
                                    .py_2()
                                    .border_b_1()
                                    .border_color(color::border_subtle())
                                    .child(dot().mt_1())
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.))
                                            .child(
                                                div().child(format!(
                                                    "{} {}",
                                                    event.stage, event.detail
                                                )),
                                            )
                                            .when_some(event.path.clone(), |content, path| {
                                                content.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(color::text_secondary())
                                                        .child(path),
                                                )
                                            }),
                                    )
                            }),
                    ),
            )
    }

    fn summary_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let report = self.report.clone().unwrap_or_default();
        panel()
            .w(px(235.))
            .min_w(px(235.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.section_header("SUM", "결과 요약", None::<Div>))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .child(stat_tile(
                        "SCAN",
                        "스캔",
                        report.scanned_files.to_string(),
                        false,
                    ))
                    .child(stat_tile(
                        "CAND",
                        "후보",
                        report.candidate_files.to_string(),
                        false,
                    ))
                    .child(stat_tile(
                        "HASH",
                        "해시",
                        report.hashed_files.to_string(),
                        false,
                    ))
                    .child(stat_tile(
                        "DB",
                        "캐시",
                        report.reused_hashes.to_string(),
                        false,
                    ))
                    .child(stat_tile(
                        "GRP",
                        "그룹",
                        report.duplicate_groups.to_string(),
                        false,
                    ))
                    .child(stat_tile(
                        "MOVE",
                        "분류",
                        format!(
                            "{}개 / {}",
                            report.deleted_files,
                            format_bytes(report.reclaimed_bytes)
                        ),
                        true,
                    ))
                    .child(div().mt_auto().p(px(14.)).child(button(
                        "relations",
                        "중복 관계 보기",
                        ButtonKind::Wide,
                        self.duplicate_relations().is_empty(),
                        cx.listener(|view, _, _, cx| view.open_dialog(Dialog::Relations, cx)),
                    ))),
            )
    }

    fn action_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel()
            .h(px(59.))
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(13.))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_color(color::text_secondary())
                    .child(
                        svg()
                            .path(ICON_INFO_PATH)
                            .w(px(18.))
                            .h(px(18.))
                            .text_color(color::text_secondary()),
                    )
                    .child(self.action_hint()),
            )
            .child(button(
                "clear-cache",
                "캐시 삭제",
                ButtonKind::Danger,
                self.running,
                cx.listener(|view, _, _, cx| view.open_dialog(Dialog::CacheConfirm, cx)),
            ))
            .child(button(
                "run-stop",
                if self.running {
                    "검사 중지"
                } else {
                    "검사 시작"
                },
                if self.running {
                    ButtonKind::Stop
                } else {
                    ButtonKind::Run
                },
                self.pending || (!self.running && !self.can_start_scan()),
                cx.listener(|view, _, _, cx| {
                    if view.running {
                        view.cancel_scan(cx);
                    } else {
                        view.open_dialog(Dialog::ScanConfirm, cx);
                    }
                }),
            ))
    }

    fn modal_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .child(div().absolute().inset_0().bg(rgba(0x0f172a1f)))
            .child(self.modal_card(cx))
    }

    fn modal_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.dialog {
            Dialog::CacheConfirm => self.cache_confirm_dialog(cx).into_any_element(),
            Dialog::ScanConfirm => self.scan_confirm_dialog(cx).into_any_element(),
            Dialog::Md5Confirm => self.md5_confirm_dialog(cx).into_any_element(),
            Dialog::ScanMode => self.scan_mode_dialog(cx).into_any_element(),
            Dialog::Algorithm => self.algorithm_dialog(cx).into_any_element(),
            Dialog::Paths => self.paths_dialog(cx).into_any_element(),
            Dialog::CacheLimit => self.cache_limit_dialog(cx).into_any_element(),
            Dialog::Quarantine => self.quarantine_dialog(cx).into_any_element(),
            Dialog::Relations => self.relations_dialog(cx).into_any_element(),
            Dialog::None => div().into_any_element(),
        }
    }

    fn dialog_shell(&self, title: &'static str) -> Div {
        panel()
            .w(px(540.))
            .max_h(px(640.))
            .p_6()
            .gap_3()
            .flex()
            .flex_col()
            .child(div().text_size(px(15.)).child(title))
    }

    fn dialog_actions(
        &self,
        primary: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .mt_4()
            .flex()
            .justify_end()
            .gap_2()
            .child(button(
                "dialog-close",
                "닫기",
                ButtonKind::Dialog,
                false,
                cx.listener(|view, _, _, cx| view.close_dialog(cx)),
            ))
            .child(primary)
    }

    fn settings_confirm_action(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().mt_4().flex().justify_end().child(button(
            "settings-confirm",
            "확인",
            ButtonKind::Run,
            false,
            cx.listener(|view, _, _, cx| view.close_dialog(cx)),
        ))
    }

    fn cache_confirm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.dialog_shell("SQLite 캐시 삭제")
            .child(
                div().text_color(color::text_secondary()).child(
                    "SQLite 해시 캐시를 삭제하면 다음 검사에서 필요한 해시를 다시 계산합니다.",
                ),
            )
            .child(self.dialog_actions(
                button(
                    "confirm-cache-delete",
                    "삭제",
                    ButtonKind::Danger,
                    false,
                    cx.listener(|view, _, _, cx| view.clear_cache(cx)),
                ),
                cx,
            ))
    }

    fn scan_confirm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(color::border_subtle())
            .bg(color::bg_app())
            .flex()
            .flex_col()
            .gap_2();
        list = list.child(confirm_row("검사 경로", self.path_title()));
        list = list.child(confirm_row("검사 모드", self.scan_mode.label()));
        list = list.child(confirm_row("비교 기준", self.algorithm.label()));
        self.dialog_shell("검사 시작")
            .child(
                div()
                    .text_color(color::text_secondary())
                    .child("선택한 디렉터리에서 중복 파일 검사를 시작하시겠습니까?"),
            )
            .child(list)
            .child(self.dialog_actions(
                button(
                    "confirm-start",
                    "시작",
                    ButtonKind::Run,
                    !self.can_start_scan(),
                    cx.listener(|view, _, _, cx| view.start_scan(cx)),
                ),
                cx,
            ))
    }

    fn md5_confirm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.dialog_shell("MD5 사용 확인")
            .child(
                div()
                    .text_color(color::text_secondary())
                    .child("MD5는 충돌에 취약하므로 신뢰할 수 없는 파일의 중복 제거 기준으로 권장하지 않습니다."),
            )
            .child(
                div()
                    .text_color(color::danger())
                    .child("레거시 MD5 목록과 맞춰야 하는 경우에만 사용하십시오."),
            )
            .child(self.dialog_actions(
                button(
                    "confirm-md5",
                    "MD5 사용",
                    ButtonKind::Danger,
                    false,
                    cx.listener(|view, _, _, cx| view.save_algorithm(HashAlgorithm::Md5, cx)),
                ),
                cx,
            ))
    }

    fn scan_mode_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shell = self.dialog_shell("검사 모드");
        for mode in HashKillerScanModes::all() {
            let selected = self.scan_mode == mode;
            shell = shell.child(option_row(
                format!("mode-{}", mode.id()),
                mode.label(),
                mode.description(),
                selected,
                cx.listener(move |view, _, _, cx| view.update_scan_mode(mode, cx)),
            ));
        }
        shell.child(self.settings_confirm_action(cx))
    }

    fn algorithm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shell = self.dialog_shell("비교 기준");
        for algorithm in HashAlgorithm::all().iter().copied() {
            let selected = self.algorithm == algorithm;
            shell = shell.child(option_row(
                format!("algorithm-{}", algorithm.id()),
                algorithm.label(),
                algorithm.description(),
                selected,
                cx.listener(move |view, _, _, cx| view.update_algorithm(algorithm, cx)),
            ));
        }
        shell.child(self.settings_confirm_action(cx))
    }

    fn paths_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shell = self.dialog_shell("검사 경로");
        if self.paths.is_empty() {
            shell = shell.child(
                div()
                    .text_color(color::text_secondary())
                    .child("선택된 폴더가 없습니다."),
            );
        }
        for (index, path) in self.paths.iter().enumerate() {
            let path_value = path.clone();
            let selected = self.selected_paths.iter().any(|selected| selected == path);
            shell = shell.child(
                div()
                    .id(("path", index))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .rounded_sm()
                    .border_1()
                    .border_color(if selected {
                        color::accent()
                    } else {
                        color::border_subtle()
                    })
                    .bg(color::bg_app())
                    .child(if selected { "[x]" } else { "[ ]" })
                    .child(div().flex_1().min_w(px(0.)).child(path.clone()))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.toggle_selected_path(path_value.clone(), cx)
                    })),
            );
        }
        shell.child(
            div()
                .mt_4()
                .flex()
                .justify_end()
                .gap_2()
                .child(button(
                    "paths-close",
                    "닫기",
                    ButtonKind::Dialog,
                    false,
                    cx.listener(|view, _, _, cx| view.close_dialog(cx)),
                ))
                .child(button(
                    "remove-paths",
                    "선택 삭제",
                    ButtonKind::Danger,
                    self.selected_paths.is_empty(),
                    cx.listener(|view, _, _, cx| view.remove_selected_paths(cx)),
                )),
        )
    }

    fn cache_limit_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.dialog_shell("SQLite 캐시 제한")
            .child(
                div()
                    .h(px(42.))
                    .px_3()
                    .rounded_md()
                    .border_1()
                    .border_color(color::border_default())
                    .bg(color::bg_app())
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("cache-limit-input")
                            .track_focus(&self.cache_limit_focus)
                            .cursor(CursorStyle::IBeam)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|view, _, window, _| {
                                    window.focus(&view.cache_limit_focus)
                                }),
                            )
                            .on_key_down(cx.listener(|view, event, window, cx| {
                                view.handle_cache_limit_key(event, window, cx)
                            }))
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(16.))
                            .text_color(color::text_primary())
                            .child(self.cache_limit_input.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(color::text_secondary())
                            .child("MB"),
                    ),
            )
            .child(self.dialog_actions(
                button(
                    "save-cache-limit",
                    "저장",
                    ButtonKind::Run,
                    false,
                    cx.listener(|view, _, _, cx| view.save_cache_limit_input(cx)),
                ),
                cx,
            ))
    }

    fn quarantine_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shell = self.dialog_shell("보관 폴더");
        if self.destinations.is_empty() {
            shell = shell.child(
                div()
                    .text_color(color::text_secondary())
                    .child("검사 경로를 먼저 선택하십시오."),
            );
        }
        for (index, destination) in self.destinations.iter().enumerate() {
            shell = shell.child(
                div()
                    .id(("quarantine", index))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .p_3()
                    .rounded_sm()
                    .border_1()
                    .border_color(color::border_subtle())
                    .bg(color::bg_app())
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(destination.root_paths.join(", "))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(color::text_secondary())
                                    .child(destination.target_path.clone()),
                            ),
                    )
                    .child(button(
                        format!("choose-quarantine-{index}"),
                        "폴더 선택",
                        if destination.configured {
                            ButtonKind::Small
                        } else {
                            ButtonKind::Configured
                        },
                        false,
                        cx.listener(move |view, _, _, cx| view.choose_quarantine(index, cx)),
                    )),
            );
        }
        shell.child(self.settings_confirm_action(cx))
    }

    fn relations_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut shell = self.dialog_shell("중복 관계").w(px(760.));
        shell = shell.child(
            div()
                .flex()
                .gap_2()
                .mb_2()
                .child(filter_button(
                    "전체",
                    self.relation_filter_count(RelationFilter::All),
                    self.relation_filter == RelationFilter::All,
                    RelationFilter::All,
                    cx,
                ))
                .child(filter_button(
                    "같은 이름+용량",
                    self.relation_filter_count(RelationFilter::SameNameAndSize),
                    self.relation_filter == RelationFilter::SameNameAndSize,
                    RelationFilter::SameNameAndSize,
                    cx,
                ))
                .child(filter_button(
                    "다른 이름+용량+해시",
                    self.relation_filter_count(RelationFilter::SameSizeAndHash),
                    self.relation_filter == RelationFilter::SameSizeAndHash,
                    RelationFilter::SameSizeAndHash,
                    cx,
                )),
        );
        let relations = self.filtered_relations();
        if relations.is_empty() {
            shell = shell.child(
                div()
                    .text_color(color::text_secondary())
                    .child("선택한 필터에 해당하는 중복 관계가 없습니다."),
            );
        }
        let mut list = div()
            .id("relations-scroll")
            .flex()
            .flex_col()
            .gap_2()
            .max_h(px(380.))
            .overflow_y_scroll();
        for (index, relation) in relations.into_iter().enumerate() {
            let original = relation.original_path.clone();
            let current = relation.current_duplicate_path.clone();
            list = list.child(
                div()
                    .id(("relation", index))
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(color::border_default())
                    .bg(color::bg_app())
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .mb_3()
                            .child(format!("관계 {}", index + 1))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(color::text_secondary())
                                    .child(format!(
                                        "{} · {} · {}",
                                        relation_kind_label(&relation.kind),
                                        format_bytes(relation.size),
                                        compact_hash(&relation.hash)
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(relation_path_box(
                                "원본 파일",
                                relation.original_path.clone(),
                                original,
                                cx,
                            ))
                            .child(relation_path_box(
                                "보관 위치",
                                relation.current_duplicate_path.clone(),
                                current,
                                cx,
                            )),
                    ),
            );
        }
        shell.child(list).child(
            div()
                .mt_4()
                .flex()
                .justify_end()
                .gap_2()
                .child(button(
                    "save-relations",
                    "로그 저장",
                    ButtonKind::Small,
                    self.duplicate_relations().is_empty(),
                    cx.listener(|view, _, _, cx| view.save_relations_log(cx)),
                ))
                .child(button(
                    "relations-close",
                    "닫기",
                    ButtonKind::Dialog,
                    false,
                    cx.listener(|view, _, _, cx| view.close_dialog(cx)),
                )),
        )
    }
}

impl Render for HashKillerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.close_handler_registered {
            let entity = cx.entity();
            window.on_window_should_close(cx, move |window, cx| {
                entity.update(cx, |view, cx| view.handle_window_should_close(window, cx))
            });
            self.close_handler_registered = true;
        }
        self.root_shell(window, cx)
    }
}

struct HashKillerScanModes;

impl HashKillerScanModes {
    fn all() -> [ScanMode; 3] {
        [ScanMode::Fast, ScanMode::FullHash, ScanMode::Rehash]
    }
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Small,
    Dialog,
    Folder,
    Run,
    Stop,
    Danger,
    Configured,
    Wide,
}

fn panel() -> Div {
    div()
        .bg(color::bg_panel())
        .border_1()
        .border_color(color::border_default())
        .rounded_lg()
}

fn button(
    id: impl Into<String>,
    label: impl Into<String>,
    kind: ButtonKind,
    disabled: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<Div> {
    let label = label.into();
    let id = id.into();
    let icon = button_icon_path(&id, &label);
    let icon_color = if disabled && matches!(kind, ButtonKind::Run) {
        color::text_tertiary()
    } else {
        color::button_icon(kind)
    };
    let base = div()
        .id(SharedString::from(id))
        .h(px(28.))
        .min_w(px(44.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.))
        .rounded_md()
        .border_1()
        .border_color(color::border_default())
        .bg(color::bg_panel())
        .text_color(color::text_primary())
        .text_size(px(12.))
        .cursor_pointer()
        .when_some(icon, |button, icon| {
            button.child(
                svg()
                    .path(icon)
                    .w(px(12.))
                    .h(px(12.))
                    .text_color(icon_color),
            )
        })
        .child(label)
        .when(disabled, |button| button.cursor(CursorStyle::Arrow))
        .when(disabled && !matches!(kind, ButtonKind::Run), |button| {
            button.opacity(0.5)
        })
        .on_click(move |event, window, cx| {
            if !disabled {
                listener(event, window, cx);
            }
        });
    match kind {
        ButtonKind::Small => base,
        ButtonKind::Dialog => base.h(px(34.)).min_w(px(95.)).px(px(15.)).rounded_lg(),
        ButtonKind::Folder => base
            .h(px(32.))
            .min_w(px(96.))
            .px(px(16.))
            .rounded_lg()
            .border_color(rgb(0xbfdbfe))
            .bg(rgb(0xeff6ff))
            .text_color(rgb(0x1d4ed8)),
        ButtonKind::Run => base
            .h(px(34.))
            .min_w(px(96.))
            .px(px(16.))
            .rounded_lg()
            .border_color(if disabled {
                color::border_default()
            } else {
                color::accent()
            })
            .bg(if disabled {
                color::bg_active()
            } else {
                color::accent()
            })
            .text_color(if disabled {
                color::text_tertiary()
            } else {
                rgb(0xffffff)
            }),
        ButtonKind::Stop => base
            .h(px(34.))
            .min_w(px(96.))
            .px(px(16.))
            .rounded_lg()
            .border_color(color::danger())
            .bg(color::danger())
            .text_color(rgb(0xffffff)),
        ButtonKind::Danger => base
            .h(px(34.))
            .min_w(px(95.))
            .px(px(15.))
            .rounded_lg()
            .text_color(color::danger()),
        ButtonKind::Configured => base
            .border_color(color::accent_dark())
            .bg(color::accent_dark())
            .text_color(rgb(0xffffff)),
        ButtonKind::Wide => base.w_full().h(px(30.)).justify_center().px(px(14.)),
    }
}

fn window_control_button(
    id: &'static str,
    label: &'static str,
    area: WindowControlArea,
    close: bool,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .w(px(46.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(color::text_secondary())
        .window_control_area(area)
        .hover(|button| {
            if close {
                button.bg(color::danger()).text_color(rgb(0xffffff))
            } else {
                button.bg(color::bg_hover())
            }
        })
        .child(div().text_size(px(15.)).line_height(px(15.)).child(label))
}

fn icon_box(label: &'static str, size: gpui::Pixels) -> Div {
    let icon_size = if size == px(46.0) {
        px(28.0)
    } else if size == px(36.0) {
        px(22.0)
    } else if size == px(34.0) {
        px(20.0)
    } else if size == px(28.0) {
        px(17.0)
    } else if size == px(26.0) {
        px(15.0)
    } else if size == px(24.0) {
        px(14.0)
    } else if size == px(22.0) {
        px(13.0)
    } else {
        px(14.0)
    };
    let color = icon_color(label);
    div()
        .w(size)
        .h(size)
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(if label == "DIR" {
            color::blue_border()
        } else {
            color::border_subtle()
        })
        .bg(if label == "DIR" {
            color::blue_soft()
        } else {
            color::bg_hover()
        })
        .text_color(color)
        .child(
            svg()
                .path(icon_path(label))
                .w(icon_size)
                .h(icon_size)
                .text_color(color),
        )
}

fn icon_color(label: &str) -> gpui::Rgba {
    if label == "DIR" {
        color::blue()
    } else {
        color::text_secondary()
    }
}

fn icon_path(label: &str) -> &'static str {
    match label {
        "DIR" | "PATH" => ICON_FOLDER_PATH,
        "CFG" | "MODE" => ICON_SETTINGS_PATH,
        "RUN" | "STAT" => ICON_ACTIVITY_PATH,
        "SUM" => ICON_CHART_PATH,
        "HASH" => ICON_HASH_PATH,
        "DB" => ICON_DATABASE_PATH,
        "KEEP" => ICON_ARCHIVE_PATH,
        "LOG" => ICON_LIST_PATH,
        "FILE" | "SCAN" => ICON_FILE_SEARCH_PATH,
        "TIME" => ICON_CLOCK_PATH,
        "CAND" => ICON_FILTER_PATH,
        "GRP" => ICON_GROUP_PATH,
        "MOVE" => ICON_MOVE_PATH,
        _ => ICON_CIRCLE_PATH,
    }
}

fn button_icon_path(id: &str, label: &str) -> Option<&'static str> {
    if id.starts_with("filter-") {
        return Some(ICON_FILTER_PATH);
    }
    match id {
        "choose-folders" => Some(ICON_FOLDER_PLUS_PATH),
        "save-activity" | "save-relations" => Some(ICON_DOWNLOAD_PATH),
        "save-cache-limit" => Some(ICON_SAVE_PATH),
        "clear-cache" | "confirm-cache-delete" | "remove-paths" => Some(ICON_TRASH_PATH),
        "confirm-start" => Some(ICON_PLAY_PATH),
        "run-stop" if label == "검사 중지" => Some(ICON_STOP_PATH),
        "run-stop" => Some(ICON_PLAY_PATH),
        "dialog-close" | "paths-close" | "relations-close" => Some(ICON_X_PATH),
        "relations" => Some(ICON_LINK_PATH),
        "open-file" => Some(ICON_EXTERNAL_PATH),
        _ => None,
    }
}

fn progress_meta(
    icon: &'static str,
    label: &'static str,
    value: impl Into<String>,
    divider: bool,
) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .when(divider, |meta| {
            meta.border_r_1().border_color(color::border_subtle())
        })
        .text_align(gpui::TextAlign::Center)
        .child(icon_box(icon, px(28.)))
        .child(
            div()
                .text_size(px(12.))
                .text_color(color::text_secondary())
                .child(label),
        )
        .child(
            div()
                .text_size(px(18.))
                .font_weight(gpui::FontWeight::BOLD)
                .child(value.into()),
        )
}

fn stat_tile(
    icon: &'static str,
    label: &'static str,
    value: impl Into<String>,
    danger: bool,
) -> impl IntoElement {
    div()
        .h(px(42.))
        .flex()
        .items_center()
        .gap_3()
        .px_5()
        .border_b_1()
        .border_color(color::border_subtle())
        .child(icon_box(icon, px(26.)))
        .child(
            div()
                .flex_1()
                .text_size(px(14.))
                .text_color(if danger {
                    color::danger_hover()
                } else {
                    color::text_secondary()
                })
                .child(label),
        )
        .child(
            div()
                .text_size(px(15.))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(if danger {
                    color::danger()
                } else {
                    color::text_primary()
                })
                .child(value.into()),
        )
}

fn dot() -> Div {
    div()
        .w(px(8.))
        .h(px(8.))
        .rounded_full()
        .bg(color::text_tertiary())
        .flex_none()
}

fn empty_activity_state() -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_align(gpui::TextAlign::Center)
        .child(
            div()
                .relative()
                .w(px(82.))
                .h(px(64.))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(ICON_CLIPBOARD_PATH)
                        .w(px(46.))
                        .h(px(46.))
                        .text_color(color::blue_faint()),
                )
                .child(
                    svg()
                        .path(ICON_SPARKLE_PATH)
                        .absolute()
                        .left(px(10.))
                        .top(px(7.))
                        .w(px(8.))
                        .h(px(8.))
                        .text_color(color::blue_faint()),
                )
                .child(
                    svg()
                        .path(ICON_SPARKLE_PATH)
                        .absolute()
                        .right(px(8.))
                        .bottom(px(12.))
                        .w(px(7.))
                        .h(px(7.))
                        .text_color(color::blue_faint()),
                ),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::BOLD)
                .child("대기 중"),
        )
        .child(
            div()
                .text_color(color::text_tertiary())
                .child("검사를 시작하면 현재 처리 중인 작업과 파일이 표시됩니다."),
        )
}

fn option_row(
    id: impl Into<String>,
    label: &'static str,
    description: &'static str,
    selected: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .cursor_pointer()
        .flex()
        .flex_col()
        .items_center()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            color::text_primary()
        } else {
            color::border_default()
        })
        .bg(if selected {
            color::bg_hover()
        } else {
            color::bg_app()
        })
        .child(div().text_size(px(14.)).child(label))
        .child(
            div()
                .text_size(px(12.))
                .text_color(color::text_secondary())
                .child(description),
        )
        .on_click(listener)
}

fn confirm_row(label: &'static str, value: impl Into<String>) -> impl IntoElement {
    div()
        .flex()
        .gap_3()
        .child(
            div()
                .w(px(88.))
                .text_size(px(12.))
                .text_color(color::text_secondary())
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .text_size(px(12.))
                .child(value.into()),
        )
}

fn filter_button(
    label: &'static str,
    count: usize,
    selected: bool,
    filter: RelationFilter,
    cx: &mut Context<HashKillerApp>,
) -> impl IntoElement {
    let text = format!("{label} {count}");
    button(
        format!("filter-{label}"),
        text,
        if selected {
            ButtonKind::Configured
        } else {
            ButtonKind::Small
        },
        false,
        cx.listener(move |view, _, _, cx| {
            view.relation_filter = filter;
            cx.notify();
        }),
    )
}

fn relation_path_box(
    label: &'static str,
    path: String,
    open_path: String,
    cx: &mut Context<HashKillerApp>,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(color::text_secondary())
                .child(label),
        )
        .child(div().text_size(px(12.)).child(path))
        .child(button(
            "open-file",
            "위치 열기",
            ButtonKind::Small,
            false,
            cx.listener(move |view, _, _, cx| view.open_file(open_path.clone(), cx)),
        ))
}

fn compact_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    let normalized = path.replace('\\', "/");
    let tail = normalized
        .split('/')
        .rfind(|part| !part.is_empty())
        .unwrap_or(&normalized);
    if tail.chars().count() + 4 <= max {
        format!(".../{tail}")
    } else {
        let take = max.saturating_sub(3).max(1);
        format!("{}...", tail.chars().take(take).collect::<String>())
    }
}

fn compact_hash(hash: &str) -> String {
    if hash.len() <= 14 {
        hash.to_string()
    } else {
        format!("{}...", &hash[..14])
    }
}

fn format_bytes(value: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = value as f64;
    let mut index = 0;
    while size >= 1024.0 && index < units.len() - 1 {
        size /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{} {}", size.round() as u64, units[index])
    } else {
        format!("{:.1} {}", size, units[index])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{hours}:{:02}:{:02}", minutes % 60, seconds % 60)
    } else {
        format!("{minutes}:{:02}", seconds % 60)
    }
}

fn is_same_name_relation(kind: &hash_killer::duplicate_cleaner::DuplicateRelationKind) -> bool {
    matches!(
        kind,
        hash_killer::duplicate_cleaner::DuplicateRelationKind::SameNameAndSize
    )
}

fn relation_kind_label(
    kind: &hash_killer::duplicate_cleaner::DuplicateRelationKind,
) -> &'static str {
    if is_same_name_relation(kind) {
        "같은 이름+용량"
    } else {
        "다른 이름+용량+해시"
    }
}

fn format_activity_log(events: &[ActivityEvent]) -> String {
    events
        .iter()
        .map(|event| {
            let progress = event
                .progress
                .map(|progress| format!("\t{:.1}%", progress * 100.0))
                .unwrap_or_default();
            let path = event
                .path
                .as_ref()
                .map(|path| format!("\t{path}"))
                .unwrap_or_default();
            format!("{}\t{}{}{}", event.stage, event.detail, progress, path)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_duplicate_relations_log(relations: &[DuplicateRelation]) -> String {
    relations
        .iter()
        .map(|relation| {
            [
                relation_kind_label(&relation.kind).to_string(),
                relation.size.to_string(),
                relation.hash.clone(),
                relation.original_path.clone(),
                relation.current_duplicate_path.clone(),
            ]
            .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_os = "macos")]
fn set_application_icon() {
    use cocoa::appkit::{NSApp, NSApplication, NSImage};
    use cocoa::base::nil;
    use cocoa::foundation::NSData;
    use std::ffi::c_void;

    unsafe {
        let data = NSData::dataWithBytes_length_(
            nil,
            APP_ICON_PNG.as_ptr().cast::<c_void>(),
            APP_ICON_PNG.len() as _,
        );
        let image = NSImage::initWithData_(NSImage::alloc(nil), data);
        if image != nil {
            NSApp().setApplicationIconImage_(image);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn set_application_icon() {}

fn reveal_file_path(path: PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

mod color {
    use gpui::{rgb, Rgba};

    pub fn bg_app() -> Rgba {
        rgb(0xFBFCFE)
    }

    pub fn bg_panel() -> Rgba {
        rgb(0xFFFFFF)
    }

    pub fn bg_hover() -> Rgba {
        rgb(0xF5F7FA)
    }

    pub fn bg_active() -> Rgba {
        rgb(0xE6EAF0)
    }

    pub fn bg_header() -> Rgba {
        rgb(0xFFFFFF)
    }

    pub fn border_subtle() -> Rgba {
        rgb(0xEEF1F5)
    }

    pub fn border_default() -> Rgba {
        rgb(0xE1E6EF)
    }

    pub fn text_primary() -> Rgba {
        rgb(0x111827)
    }

    pub fn text_secondary() -> Rgba {
        rgb(0x687385)
    }

    pub fn text_tertiary() -> Rgba {
        rgb(0x9AA4B2)
    }

    pub fn accent() -> Rgba {
        rgb(0x2563EB)
    }

    pub fn accent_dark() -> Rgba {
        rgb(0x0F172A)
    }

    pub fn blue() -> Rgba {
        rgb(0x2563EB)
    }

    pub fn blue_soft() -> Rgba {
        rgb(0xEFF6FF)
    }

    pub fn blue_border() -> Rgba {
        rgb(0xD8E7FF)
    }

    pub fn blue_faint() -> Rgba {
        rgb(0xBFD7FF)
    }

    pub fn danger() -> Rgba {
        rgb(0xDC2626)
    }

    pub fn danger_hover() -> Rgba {
        rgb(0xB91C1C)
    }

    pub fn button_icon(kind: super::ButtonKind) -> Rgba {
        match kind {
            super::ButtonKind::Run | super::ButtonKind::Stop | super::ButtonKind::Configured => {
                rgb(0xFFFFFF)
            }
            super::ButtonKind::Danger => danger(),
            super::ButtonKind::Folder => rgb(0x1D4ED8),
            _ => text_secondary(),
        }
    }
}

struct IconAssets;

impl AssetSource for IconAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(icon_svg(path).map(|source| Cow::Borrowed(source.as_bytes())))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

fn icon_svg(path: &str) -> Option<&'static str> {
    match path {
        ICON_FOLDER_PATH => Some(ICON_FOLDER),
        ICON_FOLDER_PLUS_PATH => Some(ICON_FOLDER_PLUS),
        ICON_SETTINGS_PATH => Some(ICON_SETTINGS),
        ICON_ACTIVITY_PATH => Some(ICON_ACTIVITY),
        ICON_CHART_PATH => Some(ICON_CHART),
        ICON_HASH_PATH => Some(ICON_HASH),
        ICON_DATABASE_PATH => Some(ICON_DATABASE),
        ICON_ARCHIVE_PATH => Some(ICON_ARCHIVE),
        ICON_LIST_PATH => Some(ICON_LIST),
        ICON_FILE_SEARCH_PATH => Some(ICON_FILE_SEARCH),
        ICON_CLOCK_PATH => Some(ICON_CLOCK),
        ICON_FILTER_PATH => Some(ICON_FILTER),
        ICON_GROUP_PATH => Some(ICON_GROUP),
        ICON_MOVE_PATH => Some(ICON_MOVE),
        ICON_CIRCLE_PATH => Some(ICON_CIRCLE),
        ICON_SAVE_PATH => Some(ICON_SAVE),
        ICON_DOWNLOAD_PATH => Some(ICON_DOWNLOAD),
        ICON_TRASH_PATH => Some(ICON_TRASH),
        ICON_PLAY_PATH => Some(ICON_PLAY),
        ICON_STOP_PATH => Some(ICON_STOP),
        ICON_X_PATH => Some(ICON_X),
        ICON_LINK_PATH => Some(ICON_LINK),
        ICON_EXTERNAL_PATH => Some(ICON_EXTERNAL),
        ICON_INFO_PATH => Some(ICON_INFO),
        ICON_CLIPBOARD_PATH => Some(ICON_CLIPBOARD),
        ICON_SPARKLE_PATH => Some(ICON_SPARKLE),
        ICON_WINDOW_MINIMIZE_PATH => Some(ICON_WINDOW_MINIMIZE),
        ICON_WINDOW_MAXIMIZE_PATH => Some(ICON_WINDOW_MAXIMIZE),
        ICON_WINDOW_RESTORE_PATH => Some(ICON_WINDOW_RESTORE),
        _ => None,
    }
}

const ICON_FOLDER_PATH: &str = "icons/folder.svg";
const ICON_FOLDER_PLUS_PATH: &str = "icons/folder-plus.svg";
const ICON_SETTINGS_PATH: &str = "icons/settings.svg";
const ICON_ACTIVITY_PATH: &str = "icons/activity.svg";
const ICON_CHART_PATH: &str = "icons/chart.svg";
const ICON_HASH_PATH: &str = "icons/hash.svg";
const ICON_DATABASE_PATH: &str = "icons/database.svg";
const ICON_ARCHIVE_PATH: &str = "icons/archive.svg";
const ICON_LIST_PATH: &str = "icons/list.svg";
const ICON_FILE_SEARCH_PATH: &str = "icons/file-search.svg";
const ICON_CLOCK_PATH: &str = "icons/clock.svg";
const ICON_FILTER_PATH: &str = "icons/filter.svg";
const ICON_GROUP_PATH: &str = "icons/group.svg";
const ICON_MOVE_PATH: &str = "icons/move.svg";
const ICON_CIRCLE_PATH: &str = "icons/circle.svg";
const ICON_SAVE_PATH: &str = "icons/save.svg";
const ICON_DOWNLOAD_PATH: &str = "icons/download.svg";
const ICON_TRASH_PATH: &str = "icons/trash.svg";
const ICON_PLAY_PATH: &str = "icons/play.svg";
const ICON_STOP_PATH: &str = "icons/stop.svg";
const ICON_X_PATH: &str = "icons/x.svg";
const ICON_LINK_PATH: &str = "icons/link.svg";
const ICON_EXTERNAL_PATH: &str = "icons/external.svg";
const ICON_INFO_PATH: &str = "icons/info.svg";
const ICON_CLIPBOARD_PATH: &str = "icons/clipboard.svg";
const ICON_SPARKLE_PATH: &str = "icons/sparkle.svg";
const ICON_WINDOW_MINIMIZE_PATH: &str = "icons/window-minimize.svg";
const ICON_WINDOW_MAXIMIZE_PATH: &str = "icons/window-maximize.svg";
const ICON_WINDOW_RESTORE_PATH: &str = "icons/window-restore.svg";

const ICON_FOLDER: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6.5A2.5 2.5 0 0 1 5.5 4H9l2 2h7.5A2.5 2.5 0 0 1 21 8.5v8A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z"/><path d="M3 9h18"/></svg>"#;
const ICON_FOLDER_PLUS: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6.5A2.5 2.5 0 0 1 5.5 4H9l2 2h7.5A2.5 2.5 0 0 1 21 8.5v8A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z"/><path d="M12 11v5"/><path d="M9.5 13.5h5"/></svg>"#;
const ICON_SETTINGS: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h10"/><path d="M18 7h2"/><path d="M16 5v4"/><path d="M4 12h3"/><path d="M11 12h9"/><path d="M9 10v4"/><path d="M4 17h12"/><path d="M20 17h0"/><path d="M18 15v4"/></svg>"#;
const ICON_ACTIVITY: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12h4l2-6 4 12 2-6h6"/></svg>"#;
const ICON_CHART: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19V5"/><path d="M4 19h16"/><path d="M8 16v-5"/><path d="M12 16V8"/><path d="M16 16v-3"/></svg>"#;
const ICON_HASH: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 9h14"/><path d="M4 15h14"/><path d="M10 3 8 21"/><path d="M16 3l-2 18"/></svg>"#;
const ICON_DATABASE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="7" ry="3"/><path d="M5 5v6c0 1.7 3.1 3 7 3s7-1.3 7-3V5"/><path d="M5 11v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"/></svg>"#;
const ICON_ARCHIVE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16"/><path d="M5 7v12h14V7"/><path d="M7 3h10l3 4H4z"/><path d="M10 12h4"/></svg>"#;
const ICON_LIST: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 6h12"/><path d="M8 12h12"/><path d="M8 18h12"/><path d="M4 6h.01"/><path d="M4 12h.01"/><path d="M4 18h.01"/></svg>"#;
const ICON_FILE_SEARCH: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3h8l4 4v13H6z"/><path d="M14 3v5h5"/><circle cx="11" cy="14" r="2.5"/><path d="m13 16 2 2"/></svg>"#;
const ICON_CLOCK: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="8"/><path d="M12 8v5l3 2"/></svg>"#;
const ICON_FILTER: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 5h16l-6 7v5l-4 2v-7z"/></svg>"#;
const ICON_GROUP: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3"/><path d="M3 19c.6-3 2.8-5 6-5s5.4 2 6 5"/><circle cx="17" cy="9" r="2"/><path d="M15 14c2.5.2 4.2 1.8 5 5"/></svg>"#;
const ICON_MOVE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m13 6 6 6-6 6"/><path d="M5 5v14"/></svg>"#;
const ICON_CIRCLE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="7"/></svg>"#;
const ICON_SAVE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 3h12l2 2v16H5z"/><path d="M8 3v6h8V3"/><path d="M8 17h8"/></svg>"#;
const ICON_DOWNLOAD: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v10"/><path d="m8 10 4 4 4-4"/><path d="M5 20h14"/></svg>"#;
const ICON_TRASH: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M6 7l1 14h10l1-14"/><path d="M9 7V4h6v3"/></svg>"#;
const ICON_PLAY: &str =
    r#"<svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>"#;
const ICON_STOP: &str =
    r#"<svg viewBox="0 0 24 24" fill="currentColor"><path d="M7 7h10v10H7z"/></svg>"#;
const ICON_X: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12"/><path d="M18 6 6 18"/></svg>"#;
const ICON_LINK: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1"/></svg>"#;
const ICON_EXTERNAL: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 4h6v6"/><path d="m10 14 10-10"/><path d="M20 14v5H5V4h5"/></svg>"#;
const ICON_INFO: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 8h.01"/></svg>"#;
const ICON_CLIPBOARD: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 4h6l1 2h3v15H5V6h3z"/><path d="M9 4h6v4H9z"/><path d="M9 12h1"/><path d="M13 12h3"/><path d="M9 16h1"/><path d="M13 16h3"/></svg>"#;
const ICON_SPARKLE: &str = r#"<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 2 14.4 9.6 22 12l-7.6 2.4L12 22l-2.4-7.6L2 12l7.6-2.4z"/></svg>"#;
const ICON_WINDOW_MINIMIZE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 18h12"/></svg>"#;
const ICON_WINDOW_MAXIMIZE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M7 7h10v10H7z"/></svg>"#;
const ICON_WINDOW_RESTORE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M8 8h8v8H8z"/><path d="M11 5h8v8"/></svg>"#;

fn main() {
    Application::new()
        .with_assets(IconAssets)
        .run(|cx: &mut App| {
            set_application_icon();
            cx.bind_keys([
                KeyBinding::new("cmd-q", QuitApp, None),
                KeyBinding::new("cmd-w", CloseAppWindow, None),
            ]);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let window_size = size(px(950.0), px(650.0));
            let bounds = Bounds::centered(None, window_size, cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(window_size),
                    is_resizable: true,
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(16.0), px(17.0))),
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(HashKillerApp::new),
            )
            .unwrap();
            cx.activate(true);
        });
}
