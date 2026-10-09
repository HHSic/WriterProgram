# WriterProgram

한글(HWP)의 빈자리를 채우는, 한국어 소설 작가를 위한 설치형 글쓰기 앱. 웹소설 연재 작가와 출판 장편 작가를 함께 겨냥한다. 원고는 내 컴퓨터의 작품 폴더에 텍스트 파일로 남는다.

- 스택: Tauri 2 + React/TypeScript + Tiptap + Rust
- 상태: v0.1 구현 중 ([구현 현황](docs/mvp-scope.md#구현-현황-2026-09-27))

작품 전체를 한 파일로 옮기려면 작업 화면의 **파일 → 작품 파일로 저장 (.wpm)…**을 쓴다.
원고·기획·설정집·메모·서식·기록·휴지통·창작 일지가 함께 들어간다.
시작 화면의 **작품 파일 열기…**로 다시 열며, 이후 자동 저장과 Ctrl+S도 같은 `.wpm`을 갱신한다.
설치판은 `.wpm` 파일 연결과 실행 시 파일 열기를 지원한다. 브라우저 미리보기에서는 파일 저장·열기를 지원하지 않는다.

**문자표 → 텍스트 이모티콘**에서 표정을 골라 원고나 입력칸에 넣을 수 있다 (Ctrl+F10).
기본 목록은 ASCII, 한글 자모와 전통적인 한국어 문자 범위의 일반 기호로 구성했다.
**유니코드 표정·유니코드 기호**에는 `(ง •̀_•́)ง`, `ʕ•ᴥ•ʔ`, `☺` 같은 표현도 제공한다.
목록 전체의 HWPX 내보내기·다시 가져오기에서 결합 문자와 공백까지 보존하는지 검사한다.
한글 프로그램의 실제 표시는 버전·글꼴에 따라 달라질 수 있으며, 확장 유니코드 문자는 네모로 보이거나 결합 기호의 위치가 달라질 수 있다.

작업 화면의 **공유**에서 연결된 Google Drive·OneDrive·Dropbox에 선택한 원고의 읽기용 문서 사본을 올리고 링크를 복사한다.
앱에서 드라이브 계정을 연결한 사용자만 생성·종료할 수 있다. [링크 공유](docs/sharing.md)에 권한과 사용 방법을 정리했다.

Windows 설치판은 [GitHub Releases](https://github.com/HHSic/WriterProgram/releases/latest)에서 받는다.
설치된 앱은 하루 한 번 새 버전을 확인하고, 사용자가 업데이트를 선택하면 저장·서명 검증 후 설치한다.
정식 버전 태그를 올리면 설치 파일과 업데이트 파일을 검증한 뒤 자동 공개한다. [배포와 자동 업데이트](docs/release.md).

작업 화면의 **책 미리보기**에서 앱 안의 원고를 책처럼 읽는다. **휴대폰**은 한 쪽, **PC**는 넓은 화면에서 두 쪽으로 보여 주며,
**자동**은 실제 화면 폭에 맞춘다. 목차로 회차를 고르고 버튼·방향키·터치 밀기로 쪽을 넘긴다. 글자 크기와 배경을 바꿔도 원고 서식은 바뀌지 않는다.
현재 휴대폰 선택은 앱 내부의 화면 미리보기이며 Android·iOS 설치 파일 배포를 추가하는 기능은 아니다.

## 폴더

| 위치 | 내용 |
|---|---|
| `app/` | 데스크톱 앱 (화면: React + Tiptap, `app/src-tauri`: Tauri) |
| `crates/core` | 작품 폴더 형식, 저장, 기록, 분량, 설정집, 메모, 찾기/바꾸기, 원고 서식, 내보내기 |
| `crates/revise` | 퇴고 점검 엔진 시제품 |
| `docs/` | 설계 문서 |

## 실행

필요한 것: Node.js 20 이상, Rust (stable), Windows에서는 WebView2 (Windows 11에 기본 포함).

```bash
cd app
npm install
npm run tauri dev
```

화면만 빠르게 볼 때는 `npm run dev` 후 브라우저에서 http://localhost:1420 을 연다. 이때는 예시 작품이 든 메모리 저장소를 쓰고 파일에는 아무것도 쓰지 않는다.

테스트:

```bash
cargo test
cd app
npm test
```

## 문서

- [시장 조사](docs/market-research.md), [MVP 범위](docs/mvp-scope.md), [운영 모델](docs/business-model.md)
- [화면 설계](docs/screens.md), [영역과 데이터](docs/layout-data.md), [화면 용어](docs/terminology.md)
- [코드 구조와 파일 형식](docs/architecture.md)
- [퇴고 점검 엔진](docs/revise-engine.md), [맞춤법 검사기 라이선스](docs/spellcheck-license.md)
