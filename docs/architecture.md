# 코드 구조와 파일 형식 (2026-09-27)

v0.1 구현의 뼈대. 화면 설계는 [screens.md](screens.md), 영역별 데이터는 [layout-data.md](layout-data.md).

## 코드 구성

| 위치 | 내용 |
|---|---|
| `crates/core` (writer-core) | 작품 폴더 형식, 원고 파일 읽기·쓰기, 원자적 저장, 글자 수·원고지 매수, 기록(스냅샷), 휴지통, 텍스트 내보내기. 화면·Tauri에 의존하지 않아 모바일에서도 그대로 쓴다. |
| `crates/revise` (writer-revise) | 퇴고 점검 엔진 시제품 ([revise-engine.md](revise-engine.md)). v0.2에 앱과 연결. |
| `app/src-tauri` (writer-app) | Tauri 명령. core 함수를 얇게 감싸고, 오류는 화면에 쓸 짧은 이유로 바꾼다. |
| `app/src/api` | Rust 명령 호출(`tauri.ts`), 브라우저 미리보기용 메모리 저장소(`mock.ts`) |
| `app/src/editor` | 편집기 스키마, 자동 저장, 글자 수, 장면 개요 |
| `app/src/screens`, `app/src/workspace` | 시작 화면, 새 작품, 작업 화면(왼쪽 탐색 · 가운데 원고 · 오른쪽 맥락) |
| `app/src/store.ts` | 화면 상태와 동작 (zustand) |

## 작품 폴더

```text
작품 폴더/
  project.json                     작품 정보, 부와 회차 순서
  manuscript/<id>.md               회차·장 (파일 하나에 하나)
  planning/<id>.md                 기획 문서
  .snapshots/<문서 id>/<시각>.<종류>.md   기록
  .trash/<항목 id>/                휴지통 (문서 파일 + item.json)
```

- **부는 `project.json`에만 있다.** 회차를 다른 부로 옮겨도 파일은 움직이지 않는다. 폴더 동기화에서 이동·이름 바꾸기 충돌을 줄이기 위해서다 (layout-data.md의 "폴더 + 순서"를 이렇게 정함).
- **파일 이름은 id다.** 제목과 순서는 파일 이름에 넣지 않는다. 제목을 바꾸거나 순서를 바꿔도 파일 이름이 바뀌지 않는다.
- **열 때 맞추기.** `project.json`에 없는 원고 파일(다른 기기에서 동기화로 들어온 파일 등)은 마지막 부 끝에 붙이고, 파일이 사라진 항목은 구조에서 뺀다.
- 휴지통은 30일, 자동 기록은 90일 지나면 작품을 열 때 정리한다. 직접 보관한 기록은 지우지 않는다.

### project.json

```json
{
  "app": "WriterProgram",
  "format": 1,
  "id": "k7q2m9x4t1ab",
  "title": "달빛 서점의 마지막 손님",
  "kind": "webnovel",
  "penName": "",
  "created": "2026-09-27T01:00:00.000Z",
  "goal": { "perDoc": 5000, "countSpaces": true, "daily": null },
  "sceneBreak": "◆",
  "preset": "webnovel",
  "parts": [{ "id": "p1…", "title": "1부", "docs": ["d1…", "d2…"] }],
  "planning": ["d9…"]
}
```

- `kind`: `webnovel`(웹소설) / `print`(출판 장편).
- `sceneBreak`: 화면과 내보내기에 쓰는 장면 나눔 기호. 파일 안에서는 늘 `***`.
- 이 버전이 모르는 키는 그대로 남긴다. `format`이 앱보다 새로우면 열지 않고 업데이트를 안내한다.

## 문서 파일 (.md)

아무 텍스트 편집기로 열어도 읽히는 마크다운 호환 텍스트. 앞머리(front matter) 값은 JSON 문자열로 쓴다 (YAML로도 읽힌다).

```text
---
id: "k7q2m9x4t1ab"
title: "비에 젖은 손님"
synopsis: "폐점 직전 찾아온 손님이 대여 카드를 내민다."
status: "draft"
target: 5000
created: "2026-09-27T01:00:00.000Z"
---

셔터를 반쯤 내렸을 때 종이 울렸다.

“영업, 끝났나요?”

***

남자가 봉투에서 꺼낸 것은 **책이 아니라** 대여 카드였다.
```

### 본문 표기 규칙 (`crates/core/src/markup.rs`)

| 원고 | 파일 |
|---|---|
| 문단 | 빈 줄 하나로 나눈다 |
| 문단 안 줄바꿈 (Shift+Enter) | 그냥 줄바꿈. 문단 안의 빈 줄은 `\` 한 글자 |
| 일부러 둔 빈 문단 | `&nbsp;` |
| 장면 나눔 | `***` 한 줄 |
| 굵게 · 기울임 · 취소선 | `**굵게**` · `*기울임*` · `~~취소선~~` |
| 밑줄 · 방점 | `<u>밑줄</u>` · `<span class="dot">방점</span>` |
| 메모가 붙은 구간 (v0.1 메모 기능용) | `<mark data-memo="id">구간</mark>` |

- `\`는 ASCII 문장부호를 문자 그대로 쓰게 한다. 본문의 `\`와 `*`는 늘 `\\`, `\*`로 쓰고, `~`와 `<`는 표기로 읽힐 수 있을 때만 `\`를 붙인다. 그래서 `<상태창>`, `그래~` 같은 웹소설 표현은 파일에서도 그대로 보인다.
- 손으로 고친 파일도 읽는다: 빈 줄이 여러 개면 문단 나눔 하나로, 닫히지 않은 표기는 그 문단 끝에서 닫힌 것으로 본다.
- 편집기(Tiptap)는 본문을 ProseMirror JSON으로 주고받고, Rust가 이 표기로 바꿔 쓴다. 표기 규칙은 Rust 한 곳에만 있다.

## 저장

- **원자적 저장**: 임시 파일에 쓰고 디스크에 확정한 뒤 이름을 바꿔 덮는다. 파일은 늘 이전 내용이거나 새 내용이다. Windows에서 동기화 프로그램·백신이 파일을 잠깐 잡고 있으면 몇 번 다시 시도한다.
- **자동 저장**: 입력이 0.8초 멈추면 저장하고, 계속 입력 중이어도 5초마다 저장한다. 같은 문서의 저장은 차례대로 한다. Ctrl+S는 바로 저장.
- **본문과 앞머리를 따로 저장**: 본문 저장은 디스크에 있는 제목·상태를 그대로 두고 본문만 바꾼다. 제목·시놉시스·상태는 따로 저장한다. 편집기가 오래된 제목으로 덮어쓰는 일이 없다.
- **저장 실패**: 상단에 "저장하지 못함 · 다시 시도"와 이유(디스크 공간 부족 등)를 보인다. 저장되지 않은 글이 있으면 다른 회차로 넘어가거나 창을 닫지 않는다.

## 기록 (스냅샷)

- 파일: `.snapshots/<문서 id>/20260927-101500-123.auto.md` (UTC 시각 + 종류). 앞머리에 `snapshotKind`, `snapshotName`, `snapshotAt`.
- 종류와 화면 이름: `auto` 자동 기록, `manual` 직접 보관, `before-replace` 바꾸기 전, `before-restore` 되돌리기 전, `before-revise` 퇴고 전.
- 자동 기록: 본문을 저장할 때 마지막 기록이 10분보다 오래됐고 내용이 다르면, 바뀌기 직전 원고를 남긴다.
- 이 때로 되돌리기: 지금 원고를 `before-restore`로 남긴 뒤 본문을 되돌린다. 제목·상태는 그대로.

## 분량 계산 (`crates/core/src/count.rs`, `app/src/editor/counts.ts`)

- 공백 포함 / 제외: 유니코드 글자 단위. 줄바꿈은 세지 않는다. 공백은 명시한 목록(스페이스, 탭, 전각 공백, NBSP 등).
- 원고지 매수 (200자 원고지, 20칸 × 10줄):
  - 문단 첫 칸은 비운다. 문단 안 줄바꿈 뒤 줄은 들여쓰지 않는다.
  - 아라비아 숫자와 영문 소문자는 두 자가 한 칸.
  - 마침표·쉼표 뒤에는 빈칸을 두지 않는다.
  - 줄 첫머리에 오는 공백은 쓰지 않고, 닫는 문장부호는 앞 줄 끝에 붙인다.
  - 장면 나눔은 한 줄. 회차마다 새 장에서 시작하므로 작품 전체 매수는 회차별 매수의 합.
- Rust와 TypeScript 두 구현은 `crates/core/tests/fixtures/counts.json`의 같은 사례로 검사한다.

## 개발

```bash
cargo test                              # core 테스트
cd app
npm install
npm test                                # 글자 수 (TypeScript 쪽)
npm run dev                             # 브라우저 미리보기 (메모리 저장소, 예시 작품)
npm run tauri dev                       # 데스크톱 앱
npx tauri build --debug --no-bundle     # 설치 파일 없이 실행 파일만
```
