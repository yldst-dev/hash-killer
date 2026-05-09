# hash-killer Flutter frontend

이 디렉터리는 기존 Rust/Dioxus 루트 프로젝트와 분리된 Flutter 전환 검증 앱입니다.

## 구조

```text
lib
├── main.dart
└── src
    ├── app
    ├── core
    │   ├── design_system
    │   └── native
    └── features
        └── scan
            ├── application
            ├── data
            ├── domain
            └── presentation
```

## 경계

Dart는 화면 상태, 입력, 옵션 선택, 진행 상태, 오류 표시를 담당합니다.

Rust는 파일 수집, 해시 계산, 캐시, 보관 폴더 이동, 장시간 작업을 담당합니다.

두 계층은 `NativeScanBridge` 계약을 통해서만 연결합니다.

## 현재 상태

현재 브리지는 `StubNativeScanBridge`로 연결되어 있으며 실제 검사를 실행하지 않습니다.

다음 단계는 기존 Rust 함수 `clean_duplicate_paths_with_progress`를 별도 native bridge에서 호출하도록 연결하는 것입니다.

## 검증

```bash
dart format .
flutter analyze
flutter test
```
