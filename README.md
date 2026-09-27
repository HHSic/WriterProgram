# WriterProgram

한글(HWP)의 빈자리를 채우는, 한국어 소설 작가를 위한 설치형 글쓰기 앱. 웹소설 연재 작가와 출판 장편 작가를 함께 겨냥한다. 원고는 내 컴퓨터의 작품 폴더에 텍스트 파일로 남는다.

- 스택: Tauri 2 + React/TypeScript + Tiptap + Rust
- 상태: v0.1 구현 중 ([구현 현황](docs/mvp-scope.md#구현-현황-2026-09-27))

## 폴더

| 위치 | 내용 |
|---|---|
| `app/` | 데스크톱 앱 (화면: React + Tiptap, `app/src-tauri`: Tauri) |
| `crates/core` | 작품 폴더 형식, 저장, 기록, 분량, 내보내기 |
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
