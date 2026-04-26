# hash-killer

`hash-killer`는 선택한 디렉터리에서 중복 파일을 찾아 같은 디스크의 보관 폴더로 이동하는 Dioxus 기반 데스크톱 애플리케이션입니다. 파일 내용 비교에는 해시를 사용하며, SQLite 캐시를 통해 반복 검사 시 기존 해시 결과를 재사용합니다.

## 주요 기능

- 여러 디렉터리 동시 검사
- BLAKE3, SHA-256, SHA-512, MD5 해시 비교
- 빠른 일반 모드, 전체 해시 모드, 재계산 모드 지원
- SQLite 기반 해시 캐시와 실행 결과 스냅샷 저장
- 디스크별 보관 폴더 지정
- 중복 파일 관계 확인과 작업 로그 저장
- CLI 경로 인자 기반 검사 지원
- 웹 미리보기 빌드 지원

## 기술 스택

- Rust 2021
- Dioxus 0.7.6
- Dioxus Desktop
- SQLite, rusqlite
- jwalk
- rayon
- blake3, sha2, md-5
- rfd
- same-file

## 프로젝트 구조

```text
.
├── assets
│   └── main.css
├── src
│   ├── app.rs
│   ├── cache.rs
│   ├── components.rs
│   ├── duplicate_cleaner.rs
│   ├── hash_algorithm.rs
│   ├── icons.rs
│   ├── main.rs
│   ├── quarantine.rs
│   ├── reporting.rs
│   └── scan_mode.rs
├── Cargo.toml
├── Cargo.lock
├── Dioxus.toml
└── clippy.toml
```

## 주요 파일

| 파일 | 역할 |
| --- | --- |
| `src/main.rs` | 앱 진입점입니다. CLI 인자가 있으면 검사 작업을 실행하고, 없으면 데스크톱 앱을 실행합니다. |
| `src/app.rs` | Dioxus UI, 상태 관리, 경로 선택, 검사 실행, 설정 저장, 로그 내보내기를 담당합니다. |
| `src/duplicate_cleaner.rs` | 파일 수집, 후보 선별, 해시 계산, 캐시 재사용, 중복 그룹 판별, 보관 폴더 이동을 처리합니다. |
| `src/cache.rs` | SQLite DB 연결, 스키마 준비, 앱 설정, 해시 캐시, 실행 스냅샷, 캐시 정리를 담당합니다. |
| `src/quarantine.rs` | 디스크별 보관 폴더 저장, 검증, 검사 대상 제외 경로 생성을 담당합니다. |
| `src/hash_algorithm.rs` | BLAKE3, SHA-256, SHA-512, MD5 해시 알고리즘을 정의합니다. |
| `src/scan_mode.rs` | 빠른 일반 모드, 전체 해시 모드, 재계산 모드를 정의합니다. |
| `src/reporting.rs` | CLI 리포트 출력, 바이트 단위 포맷, 진행률 계산을 담당합니다. |
| `src/components.rs` | 공통 UI 컴포넌트를 정의합니다. |
| `src/icons.rs` | 앱에서 사용하는 아이콘 컴포넌트를 정의합니다. |
| `assets/main.css` | 전체 앱 스타일을 정의합니다. |

## 사전 준비

Rust와 Dioxus CLI가 필요합니다.

```bash
cargo install dioxus-cli
```

## 실행

데스크톱 개발 서버를 실행합니다.

```bash
dx serve --platform desktop
```

Cargo로 데스크톱 앱을 실행합니다.

```bash
cargo run
```

웹 미리보기를 실행합니다.

```bash
dx serve --platform web --port 8080 --no-default-features --features web
```

웹 미리보기에서는 실제 파일 시스템 검사, SQLite 캐시 삭제, 로그 저장처럼 로컬 파일 시스템 권한이 필요한 기능이 제한됩니다.

## CLI 사용

경로 인자를 전달하면 GUI 없이 검사를 실행합니다.

```bash
cargo run -- /path/to/directory
```

여러 경로를 함께 전달할 수 있습니다.

```bash
cargo run -- /path/to/first /path/to/second
```

CLI 실행도 보관 폴더 설정을 사용합니다. 검사 대상 디스크별로 같은 디스크의 보관 폴더가 설정되어 있지 않으면 검사가 중단됩니다.

## 검사 모드

| 모드 | 설명 |
| --- | --- |
| 빠른 일반 모드 | 같은 용량의 파일만 해시 대상으로 선별하고 SQLite 캐시를 재사용합니다. |
| 전체 해시 모드 | 모든 파일을 해시 대상으로 포함합니다. |
| 재계산 모드 | 캐시를 사용하지 않고 중복 후보의 해시를 다시 계산합니다. |

## 해시 알고리즘

| 알고리즘 | 용도 |
| --- | --- |
| BLAKE3 | 기본값이며 일반 검사에 적합합니다. |
| SHA-256 | 범용 호환성이 높은 256비트 해시입니다. |
| SHA-512 | 긴 다이제스트가 필요한 검사에 사용할 수 있습니다. |
| MD5 | 레거시 비교용이며 보안 용도로 사용하지 않습니다. |

## SQLite 캐시

앱은 현재 작업 디렉터리에 `hash-killer.sqlite3` 파일을 생성합니다. 이 DB에는 앱 설정, 마지막 검사 스냅샷, 파일 해시 캐시가 저장됩니다.

캐시 제한 기본값은 `256 MB`이며, 설정 가능한 최소값은 `16 MB`입니다. 캐시 용량이 제한을 넘으면 오래된 해시 기록부터 정리합니다.

생성되는 주요 테이블은 다음과 같습니다.

| 테이블 | 내용 |
| --- | --- |
| `app_settings` | 캐시 제한, 해시 알고리즘, 검사 모드, 디스크별 보관 폴더 설정 |
| `scan_runs` | 마지막 검사 상태와 요약 결과 |
| `file_hashes` | 파일 경로, 알고리즘, 크기, 수정 시각, 해시 값 |

## 중복 파일 처리 방식

1. 선택한 경로의 파일을 수집합니다.
2. 검사 모드에 따라 중복 후보를 선별합니다.
3. SQLite 캐시에서 기존 해시를 조회합니다.
4. 캐시에 없는 파일의 해시를 계산합니다.
5. 파일 크기와 해시가 같은 그룹을 중복 그룹으로 분류합니다.
6. 각 그룹에서 경로 정렬 기준 첫 번째 파일을 보존합니다.
7. 나머지 파일을 같은 디스크에 지정된 보관 폴더로 이동합니다.

보관 폴더는 원본 파일과 같은 디스크에 있어야 합니다. 보관 폴더 내부 파일은 검사 대상에서 제외됩니다.

## 검증

정적 확인을 실행합니다.

```bash
cargo check
```

테스트를 실행합니다.

```bash
cargo test
```

릴리스 빌드를 실행합니다.

```bash
cargo build --release
```
