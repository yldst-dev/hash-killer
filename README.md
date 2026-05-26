# hash-killer

`hash-killer`는 선택한 폴더 안에서 중복 파일을 검사하고, 중복으로 분류된 파일을 지정한 보관 폴더로 이동하는 GPUI 기반 데스크톱 프로그램입니다.

현재 프로젝트는 Rust와 GPUI 기반으로 동작합니다. 기존 Tauri, Vue, Vite, WebView 프런트엔드와 관련 JavaScript/TypeScript 의존성은 제거되었습니다.

## 무엇을 하나요

- 여러 검사 폴더를 한 번에 선택할 수 있습니다.
- 검사 폴더별로 보관 폴더를 지정할 수 있습니다.
- BLAKE3, SHA-256, SHA-512, MD5 해시 기준을 선택할 수 있습니다.
- 빠른 일반 모드, 전체 해시 모드, 재계산 모드를 지원합니다.
- SQLite 캐시를 사용해 반복 검사 시 해시 계산을 줄입니다.
- 진행률, 처리 파일 수, 작업 로그, 결과 요약을 화면에서 확인할 수 있습니다.
- 중복 관계 로그와 실시간 작업 로그를 파일로 저장할 수 있습니다.
- 원본 파일과 보관 위치를 운영체제 파일 관리자에서 열 수 있습니다.

## 프로젝트 구조

```text
.
├── src
│   ├── cache.rs
│   ├── duplicate_cleaner.rs
│   ├── hash_algorithm.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── quarantine.rs
│   ├── reporting.rs
│   └── scan_mode.rs
├── Cargo.toml
├── Cargo.lock
├── LICENSE
└── clippy.toml
```

## 주요 파일

| 파일 | 설명 |
| --- | --- |
| `src/main.rs` | GPUI 앱 진입점, 화면 레이아웃, 버튼 이벤트, 모달, 백그라운드 검사 연결을 처리합니다. |
| `src/duplicate_cleaner.rs` | 파일 수집, 후보 선별, 해시 계산, 중복 판별, 보관 폴더 이동을 처리합니다. |
| `src/cache.rs` | SQLite 설정, 해시 캐시, 검사 스냅샷 저장을 처리합니다. |
| `src/quarantine.rs` | 검사 폴더별 보관 폴더 설정과 검증을 처리합니다. |
| `src/hash_algorithm.rs` | 지원하는 해시 알고리즘을 정의합니다. |
| `src/scan_mode.rs` | 검사 모드를 정의합니다. |
| `src/reporting.rs` | CLI와 테스트에서 쓰는 리포트 표시 보조 로직을 제공합니다. |

## 준비하기

필요한 도구는 다음과 같습니다.

- Rust stable
- macOS 실행 시 Xcode와 Xcode Command Line Tools
- macOS 실행 시 Metal Toolchain

Metal Toolchain이 없는 환경에서는 다음 명령으로 설치할 수 있습니다.

```bash
xcodebuild -downloadComponent MetalToolchain
```

## 개발 실행

GPUI 앱을 실행합니다.

```bash
cargo run
```

## 검증

```bash
cargo check
cargo test
```

GPUI 앱만 확인할 때는 다음 명령을 사용할 수 있습니다.

```bash
cargo check --bin hash-killer
```

## 앱 사용 흐름

1. `폴더 선택`으로 검사할 폴더를 추가합니다.
2. `보관 폴더`에서 각 검사 폴더에 대응하는 보관 폴더를 지정합니다.
3. 필요한 경우 검사 모드, 비교 기준, 캐시 제한을 조정합니다.
4. `검사 시작`을 누르고 확인 모달에서 시작합니다.
5. 진행 상태와 실시간 작업 로그를 확인합니다.
6. 완료 후 결과 요약과 중복 관계를 확인합니다.

검사가 완료된 뒤 새 폴더를 선택하면 이전 검사 폴더 목록은 해제되고 새 검사 작업으로 전환됩니다.

## SQLite 캐시

앱은 플랫폼별 애플리케이션 데이터 디렉터리에 `hash-killer.sqlite3` 파일을 생성할 수 있습니다. 이 DB에는 앱 설정, 검사 기록, 파일 해시 캐시가 저장됩니다.

주요 테이블은 다음과 같습니다.

| 테이블 | 내용 |
| --- | --- |
| `app_settings` | 캐시 제한, 해시 알고리즘, 검사 모드, 보관 폴더 설정 |
| `scan_runs` | 검사 상태와 결과 요약 |
| `file_hashes` | 파일 경로, 알고리즘, 크기, 수정 시각, 해시 값 |

캐시가 불필요하면 앱 하단의 `캐시 삭제` 버튼으로 삭제할 수 있습니다.

## 현재 상태

- Flutter 관련 파일은 제거되었습니다.
- Dioxus 관련 파일은 제거되었습니다.
- Tauri, Vue, Vite, WebView 관련 파일은 제거되었습니다.
- 앱 실행 경로는 GPUI 네이티브 앱으로 통일되어 있습니다.
