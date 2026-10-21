# AI 도우미 연결 (MCP 서버·CLI)

`crates/mcp`의 `writer-mcp`는 Claude Code 같은 AI 도우미가 작품 폴더를 읽고 쓰게 하는 MCP 서버이자 명령줄 도구다. 앱과 같은 `writer-core`로 저장하므로 원자적 저장, 기록, 다른 곳에서 바뀐 본문 확인을 그대로 거친다. 앱은 연 작품의 폴더를 지켜보므로(architecture.md "폴더 지켜보기") 도우미가 쓴 회차는 열린 화면에 바로 나타난다.

작품 폴더를 Google Drive 같은 동기화 폴더에 두면 앱, 도우미, 다른 기기가 같은 원고를 본다.

## 설치와 연결

Windows 설치판은 `writer-mcp.exe`를 앱 옆에 함께 설치한다. 작품을 열고 **작품 설정 → 기본 → AI 도우미 연결**에서 **명령 복사**를 누르면 그 작품에 맞춘 명령이 복사된다. Claude Code를 쓰는 터미널에 붙여 넣는다.

```bash
claude mcp add --scope user writerprogram -- "<앱 폴더>\writer-mcp.exe" serve --project "G:\내 드라이브\소설\사흘째 맑음"
```

- 다른 작품으로 바꾸려면 `claude mcp remove writerprogram` 뒤에 그 작품의 명령을 붙여 넣는다.
- `--project`를 빼면 도구마다 `project` 인자로 작품 폴더를 받는다. 환경 변수 `WRITERPROGRAM_PROJECT`, 또는 지금 폴더에 `project.json`이 있으면 그 폴더를 쓴다.
- 새 작품은 `create_project` 도구로 만든다. 앱의 새 작품과 같은 폴더가 생긴다(1부와 첫 회차).
- 설치판 없이 쓸 때는 `cargo install --path crates/mcp`로 빌드한다.
- 앱은 자기 실행 파일 옆에 `writer-mcp`가 있을 때만 이 칸을 보인다(명령 `mcp_server`). 브라우저 미리보기에는 없다.

### 설치판에 넣는 방법

배포 워크플로(`.github/workflows/release.yml`)가 `writer-mcp`를 릴리스로 빌드해 `app/src-tauri/binaries/writer-mcp-x86_64-pc-windows-msvc.exe`로 두고, `tauri build --config src-tauri/tauri.mcp.conf.json`으로 사이드카(`bundle.externalBin`)에 넣는다. Tauri가 이름 끝의 대상 표기를 떼고 앱 옆에 설치한다. 기본 `tauri.conf.json`에는 넣지 않았다. 넣으면 평소 빌드와 CI도 사이드카 파일을 먼저 요구한다. `binaries/`는 git에 올리지 않는다.

## 도구

| 도구 | 하는 일 |
|---|---|
| `outline` | 부·회차(id, 제목, 상태, 글자 수, 잠금), 기획 문서, 설정 카드 목록 |
| `read_doc` | 회차 하나. 본문은 한 줄에 한 문단, 장면 나눔은 `***`. `rev`(본문 지문)를 함께 준다 |
| `add_doc` | 새 회차나 기획 문서, `text`를 주면 본문까지 |
| `write_doc` | 본문 덧붙이기(`append`)·바꾸기(`replace`) |
| `set_meta` | 제목, 시놉시스, 상태 |
| `cards`, `save_card` | 설정집 카드 읽기, 만들기·고치기 (준 항목만 바꿈) |
| `notes`, `add_note` | 메모 읽기, 작품이나 회차에 메모 남기기 |
| `check` | kfiction 원고 점검(회차 id나 글). 계약 위반·경고·참고 |
| `create_project` | 새 작품 폴더 |
| `add_part`, `rename_part`, `remove_part` | 부 만들기, 이름 바꾸기, 빈 부 지우기 |
| `move_doc` | 회차를 다른 부·자리로, 기획 문서는 기획 문서 사이에서 |
| `trash_doc`, `trash`, `restore_trash` | 휴지통으로 보내기(30일 보관), 휴지통 목록, 되살리기 |
| `search`, `replace_all` | 작품 전체나 고른 문서에서 찾기(앞뒤 문맥), 모두 바꾸기 |
| `records`, `restore_record` | 문서의 기록 목록, 기록으로 되돌리기 |
| `export` | txt, docx(Word), hwpx(한글) 내보내기. 회차마다 파일 하나도 된다 |

명령줄에서는 `writer-mcp call <도구> '<JSON 인자>' [--project <폴더>]`로 같은 도구를 부른다. 인자 자리에 `-`를 주면 표준 입력에서 JSON을 읽는다. `writer-mcp tools`는 도구 목록을 보인다.

## 원고 점검 (kfiction)

Claude Code의 kfiction hook은 Write·Edit 도구로 쓴 파일만 보므로 MCP로 쓴 회차는 지나친다. 그래서 `writer-mcp`가 같은 점검기를 직접 돌린다(`crates/mcp/src/check`, `review.rs`). Node 없이 설치판만으로 돈다.

- **저장 전**: `write_doc`, `add_doc`으로 회차에 보낸 글에 계약 위반(어시스턴트 말투, 마크다운)이 있으면 저장하지 않고 고칠 줄을 돌려준다. 저장한 뒤 고치게 하면 도우미가 덧붙이기를 다시 해 글이 겹칠 수 있어서 저장 전에 막는다. hook처럼 문서마다 30분 안에 두 번까지 거절하고, 세 번째는 저장하되 답의 `check.unfixed_contract`에 남은 위반을 적는다. 깨끗한 글을 한 번 쓰면 다시 센다. 거절 횟수는 서버가 켜져 있는 동안만 기억한다.
- **저장 뒤**: 답의 `check`에 그 회차 전체의 경고(최대 20개와 전체 수)와 참고 수를 담는다. 문체 경고는 막지 않는다. 문체 수치를 0으로 몰면 문장과 개성이 깎이므로 작가가 고른다.
- 기획 문서는 점검하지 않는다.
- **JS 점검기와 같은 판정**: Rust 점검기는 `plugins/kfiction/lib`를 옮긴 것이다. `plugins/kfiction/scripts/lint-cases.mjs`가 JS 점검기로 만든 사례(`test/fixtures/lint-cases.json`, 규칙 16개 모두와 대사 86줄의 말 단계)를 두 쪽 시험이 모두 확인한다. 규칙을 바꾸면 스크립트를 다시 돌리고 Rust 쪽도 고친다. 한쪽만 고치면 시험이 실패한다.

## 원고를 지키는 규칙

- **고치기 전 기록**: `write_doc`은 바꾸기 직전 본문을 `manual` 기록(이름 "Claude가 고치기 전")으로 남긴다. 가장 최근 기록과 같으면 다시 남기지 않는다. 작가는 앱의 기록에서 언제든 되돌린다. 손으로 보관한 기록과 같은 종류라 자동으로 지워지지 않는다.
- **다른 곳에서 바뀐 본문**: `base_rev`를 주면 `read_doc` 뒤에 앱이나 다른 기기에서 본문이 바뀐 경우 쓰지 않는다. 저장하는 순간에도 `save_body`의 지문 확인을 거쳐, 그사이 바뀌면 쓰려던 글을 `this-device` 기록으로 남기고 멈춘다.
- **잠근 회차**(완료 회차 잠금)는 고치지 않는다. 모두 바꾸기와 기록으로 되돌리기도 건너뛴다.
- **모두 바꾸기**는 앱과 같은 `search::replace_all`이다. 서식을 그대로 두고, 바뀌는 문서마다 `before-replace` 기록을 남긴다. 낱말을 그대로 바꾸므로 이름을 바꾸면 뒤의 조사(은/는, 이/가)가 어긋날 수 있다. 도구 설명에 바꾼 뒤 다시 찾아보라고 적었다.
- **되돌리기**: `restore_record`는 지금 본문을 `before-restore` 기록으로 남긴 뒤 되돌린다. 휴지통으로 보낸 문서는 30일 안에 `restore_trash`로 되살린다.
- **내보내기**는 이미 있는 파일이나 비어 있지 않은 폴더에 쓰지 않는다. 작가가 확인했을 때만 `overwrite: true`. 경로는 전체 경로로만 받는다.
- **서식**: 본문은 평문으로 오가므로 굵게·방점·메모 구간·문단 여백을 실어 나르지 못한다. 서식이 있는 회차를 `replace`로 바꾸려 하면 거절하고, 작가가 확인한 뒤 `allow_plain: true`로만 바꾼다. `append`는 기존 서식을 그대로 둔다.
- **언제 쓰나**: 서버가 `initialize`에서 도우미에게 주는 안내(`instructions`)에 "작가가 넣어 달라거나 쓰라고 했을 때만 본문을 쓴다"를 넣었다. 후보와 제안은 채팅이나 메모로 남긴다([context-sentences.md](context-sentences.md) "후보를 내고 작가가 고른다").
- 도우미가 쓴 저장은 창작 일지에 남지 않는다. 일지는 앱이 기기 id를 정했을 때만 쓰고(`journal::set_device`), `writer-mcp`는 정하지 않는다. 일지는 AI 사용 여부를 다루지 않으므로([creation-proof.md](creation-proof.md)) 도우미의 저장을 따로 표시하지도 않는다.

## 한계

- 줄바꿈만 있는 문단 안 줄(Shift+Enter)은 `read_doc`에서 줄로 나뉘어 보이고, 그대로 `replace`하면 문단으로 나뉜다.
- 휴지통 비우기, 문서 합치기·나누기, 교정본 주고받기, 원고 서식 바꾸기는 도구가 없다. 앱에서 한다.
- 시험: `cargo test -p writer-mcp` (점검기 사례 대조, 저장 전 거절과 세 번째 통과, 저장 뒤 경고, 쓰고 읽기, 기록, 지문 충돌, 잠금, 서식, 카드·메모, 부와 옮기기, 휴지통, 찾기·바꾸기와 되돌리기, txt·docx·hwpx 내보내기, JSON-RPC 응답), 화면은 `AiHelperField.test.tsx`.
