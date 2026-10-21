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

**기기 간 맞추기 → 드라이브 연결 마법사**는 Google Drive·OneDrive·Dropbox의 앱 등록 안내, 등록 정보 입력·저장, 브라우저 로그인과 연결 확인을 제공한다.
JSON 파일을 직접 만들지 않아도 된다. 이미 앱이 등록된 드라이브는 바로 로그인한다. [드라이브 연결 안내](docs/drive-apps.md).

Windows 설치판은 [GitHub Releases](https://github.com/HHSic/WriterProgram/releases/latest)에서 받는다.
설치된 앱은 하루 한 번 새 버전을 확인하고, 사용자가 업데이트를 선택하면 저장·서명 검증 후 설치한다.
정식 버전 태그를 올리면 설치 파일과 업데이트 파일을 검증한 뒤 자동 공개한다. [배포와 자동 업데이트](docs/release.md).

**계정·설정 동기화**에서 Google·Kakao·이메일 앱 계정·등록된 SAML SSO로 로그인하면 화면과 책 미리보기 설정을 자동 적용한다.
Supabase Free 플랜으로 시작할 수 있으며 SAML SSO는 Pro 이상이다. 운영자는 프로젝트와 DB·로그인 제공자를 먼저 등록한다. [계정 서버 설정](docs/accounts.md).

작업 화면의 **책 미리보기**에서 앱 안의 원고를 책처럼 읽는다. **휴대폰**은 한 쪽, **PC**는 넓은 화면에서 두 쪽으로 보여 주며,
**자동**은 실제 화면 폭에 맞춘다. 목차로 회차를 고르고 버튼·방향키·터치 밀기로 쪽을 넘긴다. 글자 크기와 배경을 바꿔도 원고 서식은 바뀌지 않는다.
현재 휴대폰 선택은 앱 내부의 화면 미리보기이며 Android·iOS 설치 파일 배포를 추가하는 기능은 아니다.

## 폴더

| 위치 | 내용 |
|---|---|
| `app/` | 데스크톱 앱 (화면: React + Tiptap, `app/src-tauri`: Tauri) |
| `crates/core` | 작품 폴더 형식, 저장, 기록, 분량, 설정집, 메모, 찾기/바꾸기, 원고 서식, 내보내기 |
| `crates/revise` | 퇴고 점검 엔진 시제품 |
| `crates/mcp` | AI 도우미가 작품 폴더를 읽고 쓰는 MCP 서버·CLI ([AI 도우미 연결](docs/mcp.md)) |
| `crates/evals`, `evals/` | 맥락 문장 후보 평가 실행기(`context-eval`)와 평가셋 |
| `plugins/kfiction` | 한국어 소설 작문 플러그인 (Claude Code, Codex) |
| `docs/` | 설계 문서 |

## 한국어 소설 작문 플러그인

채팅에서 원고에 들어갈 문장·대사 후보를 맥락(관계, 말 단계, 장면)에 맞게 내고, 원고를 쓰면 점검기가 알려 주는 플러그인. Claude Code에서는 이렇게 설치한다.

```
/plugin marketplace add HHSic/WriterProgram
/plugin install kfiction@writerprogram
```

Claude Code 2.1.275 이후에서는 `/plugin install kfiction --marketplace HHSic/WriterProgram` 한 줄로도 된다. Codex 설치와 쓰는 법은 [plugins/kfiction](plugins/kfiction/README.md).

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
- [맥락 문장 후보: 과제와 평가셋](docs/context-sentences.md), [한국어 소설 생성 프롬프트 연구](reports/한국어%20소설%20생성%20프롬프트%20연구.md)

한글 가져오기 범위는 [한글 원고 가져오기](docs/import-hancom.md), 편집 기능은 [페이지·서식·저장](docs/editor-pages.md)을 참고한다.
