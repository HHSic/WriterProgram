# 코드 지도 (2026-10-02, 창작 과정 증명 2026-10-03)

어디를 고치면 되는지, 여럿이 동시에 작업할 때 어디서 나뉘는지 정리한 문서. 파일 형식과 설계 이유는 [architecture.md](architecture.md).

원칙: **영역 하나 = 폴더나 파일 하나.** 큰 파일은 `mod.rs`(Rust)나 `index.ts`(TypeScript)가 바깥 이름을 그대로 다시 내보내므로, 파일을 나눠도 부르는 쪽 경로(`writer_core::hwpx::hwpx_bytes`, `../store`, `../api/types`)는 바뀌지 않는다.

## Rust

### `crates/core` (writer-core): 화면과 무관한 모든 것

| 모듈 | 하는 일 |
|---|---|
| `project/` | 작품 폴더: `mod.rs`(project.json 읽기·쓰기), `create.rs`(새 작품·문서 파일), `structure.rs`(부·회차 순서 바꾸기), `overview.rs`(목록과 요약), `relocate.rs`(작품 옮기기), `size.rs`(작품 크기, 오래된 자동 기록 정리, 디스크 남은 공간) |
| `doc.rs`, `store.rs` | 문서 파일(앞머리 + 본문), 원자적 저장, id, 시각 문자열, 이름 바꾸기 재시도 |
| `markup/` | 본문 표기 ↔ 블록: `parse.rs`(읽기), `write.rs`(쓰기), `mod.rs`(타입, `inline_text`) |
| `count.rs`, `layout.rs`, `indent.rs` | 글자 수·원고지 매수, 예상 쪽수, 첫 줄 들여쓰기 규칙 (화면 `app/src/editor/counts.ts`, `indent.ts`와 짝) |
| `format/` | 원고 서식: `mod.rs`(값과 검사), `heads.rs`(머리말·꼬리말·자리), `paper.rs`(용지·여백·글꼴), `presets.rs`(기본 서식, 내 서식) |
| `export.rs`, `docx/`, `hwpx/`, `xml.rs` | 내보내기. docx는 `layout.rs`(머리글·꼬리글), `styles.rs`, `body.rs`. HWPX는 `header.rs`(글자·문단 모양, 스타일), `section.rs`(본문, 머리말·꼬리말 조판 부호). `xml.rs`는 둘이 함께 쓰는 이스케이프·zip 묶기·시각 |
| `import/` | 가져오기: `text.rs`(txt·md), `word.rs`(docx), `hangul.rs`(HWPX), `page.rs`(한글 파일의 쪽 모양), 공용 `xml.rs`(XML 읽기)·`para.rs`(문단 마무리), `split.rs`(회차 나누기), `notes.rs`(빠진 자리 메모), `marked/`(교정본 읽기: 글자 모양·변경 추적·편집자 메모) |
| `corrections/` | 교정본 주고받기 ([corrections.md](corrections.md)): `sent.rs`(보낸 기록), `compare.rs`(문단·어절·글자 비교, 종류 나누기), `review.rs`(교정본과 보낸 원고 비교, 남은 곳을 지금 원고에 다시 놓기 `current_review`), `flat.rs`(회차를 글자 줄로, 자리 옮기기), `apply.rs`(받아들이기·되돌리기). 화면은 `workspace/ExportDialog.tsx`(보내기), `Exchanges.tsx`(목록), `ReviewPane.tsx`(검토 탭), `store/exchange.ts`, `lib/review.ts` |
| `cards.rs`, `notes.rs`, `search.rs`, `snapshot.rs`, `trash.rs` | 설정집, 메모, 찾기·바꾸기, 기록(창작 과정 보관이면 하루 마지막 기록을 정리하지 않음), 휴지통 |
| `journal.rs` | 창작 일지: 기기별·달별 조각 `.journal/<기기 id>/<YYYY-MM>.jsonl`(옛 `<기기 id>.jsonl`은 첫 조각), 해시 사슬, 저장 묶음(10분, `flush`·`flush_all`), 저장·기록·가져오기·교정 주고받기·시각 고정 항목(각 모듈이 `journal::note`로 남김), 확인(`verify`), 요약, 기기 설정(`Settings`, 날짜 증명 허락 포함). 화면 쪽은 `editor/journal.ts`(쓰기 묶음·붙여넣기), `store/journal.ts`(켜고 끄기·첫 알림·날짜 증명 묻기와 한 시간마다 시도), `workspace/settings/JournalField.tsx`, 명령은 `commands/journal.rs` |
| `anchor/` | 시각 고정 ([creation-proof.md](creation-proof.md) 4.2): `merkle.rs`(잎과 RFC 6962 루트), `tsp.rs`(RFC 3161 요청 만들기, 응답 확인: CMS 서명·인증서 용도·사슬), `mod.rs`(잎 모으기, 보낼 때인지 `prepare`, 응답 저장과 일지 줄 `finish`, `.journal/anchors/` 읽기). 네트워크는 앱 `commands/anchor.rs`만 |
| `proof/` | 창작 과정 증명서 (5·6장): `facts.rs`(범위, 날짜별 분량·회차별 표·요약 문장), `bundle.rs`(`증명자료.json`, 범위 밖 일지 줄은 지문만), `html.rs`(증명서 한 파일, SVG 그래프), `excerpt.rs`(초고와 지금 글 비교), `verify.rs`(검증, `examples/verify_proof.rs`가 씀). 화면은 `workspace/ProofDialog.tsx`, 명령은 `commands/proof.rs` |
| `copies/`, `changes.rs`, `places.rs`, `recent.rs` | 다른 기기: `scan.rs`(동기화 사본 찾기), `merge.rs`(project.json 사본 합치기), 바뀐 파일 알아보기, 저장 위치 찾기, 최근 작품 |

### 나머지 crate

| 위치 | 하는 일 |
|---|---|
| `crates/sync` (writer-sync) | 드라이브와 맞추기: `engine.rs`(이 기기·드라이브·지난번 셋을 비교), `base.rs`(지난번 모습), `remote.rs`(드라이브가 해 줘야 할 일), `folder.rs`(흉내 드라이브), `providers/`(`config.rs` 앱 등록·주소, `session.rs` 로그인 세션과 오류 말(공간 부족 포함), 드라이브별 파일과 남은 공간 읽기), `oauth.rs`(PKCE 로그인), `http.rs`(ureq 감싸기), `secrets.rs`(자격 증명), `accounts.rs`(연결·맞추는 작품 목록) |
| `crates/ai` (writer-ai) | 자기 API 키로 쓰는 AI ([architecture.md](architecture.md) "AI 점검"): `provider.rs`(세 회사, 기본 모델, 주소·`WRITER_AI_<회사>_ENDPOINT`), `client.rs`(회사별 요청·답·오류), `mask.rs`(이름 가리기와 조사 맞추기), `tasks.rs`(회차 요약·설정 모순 점검에 보낼 글, 답 읽기, 인용 찾기), `settings.rs`(`ai.json`). 시험 `tests/flow.rs`, 흉내 서버 `tests/support/` |
| `crates/revise` (writer-revise) | 퇴고 점검 시제품: `rules/`에 점검 종류별 파일(`repetition.rs` 반복, `rhythm.rs` 문장 리듬, `expression.rs` 표현, `dialogue.rs` 대사, `names.rs` 이름) |
| `app/src-tauri/src` (writer-app) | Tauri 명령: `commands/`에 영역별 파일(project, doc, output, search, cards, notes, journal, anchor, proof, exchange, sync_folder, ai: 키는 `drives/keyring.rs`로 자격 증명 저장소에), `drives/`(keyring, connect, projects), `browser.rs`, `watch.rs`. 공용은 `state.rs`(AppState), `paths.rs`(설정 파일 위치), `error.rs`(`Res`, `fail`) |

## TypeScript (`app/src`)

| 위치 | 하는 일 |
|---|---|
| `api/tauri.ts` | Rust 명령 호출. 명령 이름 하나에 함수 하나 |
| `api/types/` | Rust와 주고받는 타입, 영역별 파일. `backend.ts`가 명령 목록(`Backend`) |
| `api/mock/` | 브라우저 미리보기용 가짜 백엔드(`npm run dev`일 때만, 배포 빌드에는 안 들어감). 영역별 파일 |
| `store/` | 화면 상태(`state.ts`)와 동작. 영역별 파일(tabs, project, docs, cards, notes, devices, drives, web, exchange, ai …), 화면은 `store/index.ts`에서 가져다 씀 |
| AI 점검 | `workspace/AiTab.tsx`(오른쪽 탭: 보낼 내용 보기 → 보내기 → 결과), `workspace/AiDialog.tsx`(AI 연결 설정), `store/ai.ts`(설정, `askAi`, 시놉시스에 넣기), `lib/aiText.ts`(보낼 양·쓴 양 문장), `api/mock/ai.ts`(흉내 답) |
| `editor/` | Tiptap 확장, 자동 저장 세션, 글자 수, 찾기·메모·설정집 강조, 서식 버튼(`markButtons.tsx`), 소리 내어 읽기(`sentences.ts` 문장 나누기, `readAloud.ts` 읽을 범위·강조·speechSynthesis; 상태는 `store/reading.ts`, 막대는 `workspace/ReadingBar.tsx`) |
| `workspace/` | 작업 화면. 대화상자는 파일 하나에 하나(`DialogHost.tsx`가 고름), 작품 설정은 `settings/`(작품 크기는 `ProjectSizeField.tsx`, 창작 일지·날짜 증명·창작 과정 보관은 `JournalField.tsx`), 창작 과정 증명서는 `ProofDialog.tsx`. 가운데 탭 종류(문서, 설정 카드, 메모함, 개요 표, 웹, 교정본 검토 `ReviewPane.tsx`)는 `lib/tabs.ts`의 `Target`과 `Panes.tsx`의 `TabContent` |
| `lib/` | 순수 도움 함수(탭 계산, 색, 날짜 글, 지연 저장 `useDebouncedSave`, 교정본 검토의 고른 것과 본문 조각 `review.ts`) |
| `styles/` | 영역별 CSS. `index.css`가 순서대로 불러오고, **순서가 우선순위**라 새 파일은 맞는 자리에 끼운다 |

## 자주 하는 작업

**Rust 명령 하나 추가**
1. core에 함수 (`crates/core/src/<영역>`)
2. `app/src-tauri/src/commands/<영역>.rs`에 `#[tauri::command]`
3. `app/src-tauri/src/lib.rs` 명령 목록에 전체 경로로 한 줄
4. `api/types/backend.ts`에 메서드, `api/tauri.ts`에 호출, `api/mock/<영역>.ts`에 흉내

**화면 동작 하나 추가**: `store/<영역>.ts`에 함수, 화면에서 쓰면 `store/index.ts`에 이름 추가.

**원고 서식 항목 하나 추가**: `format/mod.rs`(값·검사·`#[serde(default)]`) → `hwpx/section.rs`·`docx/layout.rs`(쓰기) → `import/page.rs`(한글 파일에서 읽기, 해당되면) → `api/types/format.ts` → `workspace/settings/`(화면) → `api/mock/presets.ts`.

## 여럿이 동시에 작업할 때

- 영역 파일이 겹치지 않게 나눈다. 예: 한 사람은 `import/`·`ImportDialog.tsx`, 다른 사람은 `drives/`·`store/drives.ts`.
- 함께 고치는 곳은 목록 파일뿐이다: `lib.rs`의 명령 목록, `store/index.ts`, `api/types/index.ts`, `api/types/backend.ts`, `styles/index.css`. 한 줄씩 더하는 곳이라 충돌이 나도 풀기 쉽다.
- 큰 일은 브랜치나 git worktree를 따로 써서 하고, 합친 뒤 아래 검사를 다 돌린다.

## 배포

설치 파일, 자동 업데이트, 서명 키, 새 버전 내는 순서는 [release.md](release.md). 업데이트 명령은 `commands/update.rs`, 화면은 `store/update.ts`와 보기 설정의 버전 칸.

## 검사

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt -p writer-core -p writer-sync -p writer-ai -p writer-app --check
npm --prefix app run typecheck
npm --prefix app test
```

`cargo fmt --all`은 쓰지 않는다. `crates/revise`의 기존 파일은 손으로 맞춘 모양이라 통째로 바뀐다.

## 라이브러리를 쓴 곳과 일부러 직접 짠 곳

이미 쓰는 것: quick-xml(XML 읽기·엔티티), zip, similar(교정본 비교와 증명서 초고 비교의 diff), chrono(시각 글), encoding_rs(EUC-KR 등), sha2, regex, ureq(HTTP: 드라이브, 시각 인증 요청), base64(로그인, 증명 자료의 토큰), getrandom(로그인), notify-debouncer-mini(폴더 감시), keyring-core, lindera(형태소), Tiptap, zustand. 시각 인증 응답 읽기와 서명 확인은 RustCrypto 한 계열(der 0.7): `x509-tsp`(TimeStampReq·TSTInfo), `cms`(SignedData), `x509-cert`(인증서), `rsa`·`p256`·`p384`(서명 확인). 직접 짠 DER로는 인증서 사슬과 CMS 서명 속성을 바르게 다루기 어렵고, 같은 코드가 나중에 WebAssembly 검증 페이지에서도 돈다.

직접 짠 채로 둔 것과 이유:

| 코드 | 이유 |
|---|---|
| `core/xml.rs` 이스케이프 | quick-xml은 XML이 금지한 글자를 지우지 않고 줄바꿈을 다르게 써서 파일이 달라진다 |
| `import/xml.rs` 문자 참조 | quick-xml은 `&#X41;`(대문자 X) 같은 참조를 거부해, 한글 파일의 글이 빠질 수 있다 |
| `store::new_id` | 12자 id가 파일 이름이라 형식을 바꿀 수 없다 |
| 문서 앞머리 읽기·쓰기 | 모르는 키를 순서 그대로 남겨야 한다(YAML 라이브러리는 파일을 다시 씀) |
| `count.rs` 빈칸 목록, `editor/counts.ts` | Rust와 화면 글자 수가 같아야 해서 같은 목록을 둔다 |
| `sync/oauth.rs` | 140줄로 작고 RFC 시험값으로 검사한다. oauth2 crate는 의존성이 늘고 ureq 3을 따로 붙여야 한다 |
| keyring 나눠 담기 (`drives/keyring.rs`) | Windows 자격 증명 2560바이트 한도를 넘는 토큰을 나누는 crate가 없다 |
| `crates/ai/src/client.rs` (AI 회사 SDK 없이) | Rust 공식 SDK가 없는 회사가 있고, 세 회사 요청·답이 필드 몇 개라 ureq로 직접 짜고 흉내 서버로 시험한다 |
| `lib/diff.ts` | 문단 짝짓기가 앱에 맞춰져 있고, 글자 비교만 jsdiff로 바꿔도 줄어드는 양이 작다 |
| 끌어서 옮기기 (Sidebar, 탭, 개요 표) | 마우스와 길게 누르기 메뉴가 엮여 있어 dnd-kit로 바꾸면 위험이 크다 |

나중에 할 일: `trash.rs`·`copies/`의 파일 이름 바꾸기에는 재시도가 없다(동기화 프로그램이 파일을 잡고 있으면 실패할 수 있음). `commands/`는 파일 작업을 async 안에서 바로 하고 `drives/`는 `spawn_blocking`을 쓴다. 둘을 한쪽으로 맞출지 정해야 한다.
