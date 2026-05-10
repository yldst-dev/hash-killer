# hash-killer

`hash-killer`는 선택한 폴더 안에서 중복 파일을 검사하고, 중복으로 분류된 파일을 지정한 보관 폴더로 이동하는 데스크톱 프로그램입니다.

현재 프로젝트는 Tauri, Vue, Rust 기반으로 동작합니다. 기존 Flutter와 Dioxus 프런트엔드는 제거되었으며, 실행 가능한 앱은 `apps/desktop`의 Tauri 앱 하나입니다.

## 무엇을 하나요

- 여러 검사 폴더를 한 번에 선택할 수 있습니다.
- 검사 폴더별로 보관 폴더를 지정할 수 있습니다.
- BLAKE3, SHA-256, SHA-512, MD5 해시 기준을 선택할 수 있습니다.
- 빠른 일반 모드, 전체 해시 모드, 재계산 모드를 지원합니다.
- SQLite 캐시를 사용해 반복 검사 시 해시 계산을 줄입니다.
- 진행률, 처리 파일 수, 작업 로그, 결과 요약을 화면에서 확인할 수 있습니다.
- 중복 관계 로그와 실시간 작업 로그를 파일로 저장할 수 있습니다.
- macOS는 `.app`, Windows는 포터블 `.exe` 산출물을 목표로 빌드합니다.

## 프로젝트 구조

```text
.
├── apps
│   └── desktop
│       ├── src
│       │   ├── App.vue
│       │   ├── main.ts
│       │   ├── styles.css
│       │   └── lib
│       └── src-tauri
│           ├── src
│           ├── capabilities
│           ├── icons
│           └── tauri.conf.json
├── src
│   ├── cache.rs
│   ├── duplicate_cleaner.rs
│   ├── hash_algorithm.rs
│   ├── lib.rs
│   ├── quarantine.rs
│   ├── reporting.rs
│   └── scan_mode.rs
├── Cargo.toml
├── Cargo.lock
└── clippy.toml
```

## 주요 디렉터리

| 경로 | 설명 |
| --- | --- |
| `apps/desktop` | Tauri/Vue 데스크톱 앱입니다. 화면, Tauri 설정, 앱 빌드 스크립트가 있습니다. |
| `apps/desktop/src` | Vue UI와 프런트엔드 로직입니다. |
| `apps/desktop/src-tauri` | Tauri Rust 진입점과 네이티브 명령입니다. |
| `src` | 중복 검사, 해시 계산, 캐시, 보관 폴더 처리 등 핵심 Rust 로직입니다. |

## 주요 파일

| 파일 | 설명 |
| --- | --- |
| `apps/desktop/src/App.vue` | 메인 화면, 모달, 상태 표시, 사용자 동작을 처리합니다. |
| `apps/desktop/src/lib/native.ts` | 프런트엔드에서 Tauri 명령과 이벤트를 호출합니다. |
| `apps/desktop/src/lib/format.ts` | 경로, 용량, 로그, 진행률 표시 형식을 담당합니다. |
| `apps/desktop/src/lib/lockdown.ts` | 우클릭, 새로고침, 텍스트 선택 등 웹뷰 특유 동작을 차단합니다. |
| `apps/desktop/src-tauri/src/lib.rs` | Tauri 명령, 폴더 선택, 검사 실행, 로그 저장, 파일 열기를 처리합니다. |
| `src/duplicate_cleaner.rs` | 파일 수집, 후보 선별, 해시 계산, 중복 판별, 보관 폴더 이동을 처리합니다. |
| `src/cache.rs` | SQLite 설정, 해시 캐시, 검사 스냅샷 저장을 처리합니다. |
| `src/quarantine.rs` | 검사 폴더별 보관 폴더 설정과 검증을 처리합니다. |
| `src/hash_algorithm.rs` | 지원하는 해시 알고리즘을 정의합니다. |
| `src/scan_mode.rs` | 검사 모드를 정의합니다. |

## 준비하기

필요한 도구는 다음과 같습니다.

- Rust stable
- Node.js 24 이상
- npm
- macOS 앱 빌드 시 Xcode Command Line Tools
- Windows 포터블 `.exe` 빌드 시 Windows 환경

처음 받은 뒤에는 데스크톱 앱 의존성을 설치합니다.

```bash
cd apps/desktop
npm ci
```

## 개발 실행

Tauri 개발 모드로 실행합니다.

```bash
cd apps/desktop
npm run tauri dev
```

프런트엔드만 확인해야 할 때는 Vite 개발 서버를 실행할 수 있습니다.

```bash
cd apps/desktop
npm run dev
```

## 검증

루트 Rust core 크레이트를 확인합니다.

```bash
cargo check
cargo test
```

프런트엔드 타입 검사와 번들을 확인합니다.

```bash
cd apps/desktop
npm run typecheck
npm run build
```

Tauri Rust 크레이트를 확인합니다.

```bash
cd apps/desktop/src-tauri
cargo check
```

## 빌드

macOS `.app` 빌드:

```bash
cd apps/desktop
npm run build:mac
```

빌드 결과는 보통 다음 위치에 생성됩니다.

```text
apps/desktop/src-tauri/target/release/bundle/macos/hash-killer.app
```

Windows 포터블 `.exe` 빌드:

```bash
cd apps/desktop
npm run build:windows:portable
```

Windows 환경에서 실행하면 보통 다음 위치에 실행 파일이 생성됩니다.

```text
apps/desktop/src-tauri/target/release/hash-killer-desktop.exe
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

앱은 실행 위치 기준으로 `hash-killer.sqlite3` 파일을 생성할 수 있습니다. 이 DB에는 앱 설정, 검사 기록, 파일 해시 캐시가 저장됩니다.

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
- GitHub Actions 워크플로 파일은 제거되었습니다.
- 앱 실행 경로는 Tauri/Vue 데스크톱 앱으로 통일되어 있습니다.
