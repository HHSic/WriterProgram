# AI 도우미 연결 (MCP 서버·CLI)

`crates/mcp`의 `writer-mcp`는 Claude Code 같은 AI 도우미가 작품 폴더를 읽고 쓰게 하는 MCP 서버이자 명령줄 도구다. 앱과 같은 `writer-core`로 저장하므로 원자적 저장, 기록, 다른 곳에서 바뀐 본문 확인을 그대로 거친다. 앱은 연 작품의 폴더를 지켜보므로(architecture.md "폴더 지켜보기") 도우미가 쓴 회차는 열린 화면에 바로 나타난다.

작품 폴더를 Google Drive 같은 동기화 폴더에 두면 앱, 도우미, 다른 기기가 같은 원고를 본다.

## 설치와 연결

```bash
cargo install --path crates/mcp          # writer-mcp 실행 파일
claude mcp add writerprogram -- writer-mcp serve --project "G:/내 드라이브/소설/사흘째 맑음"
```

- `--project`를 빼면 도구마다 `project` 인자로 작품 폴더를 받는다. 환경 변수 `WRITERPROGRAM_PROJECT`, 또는 지금 폴더에 `project.json`이 있으면 그 폴더를 쓴다.
- 새 작품은 `create_project` 도구로 만든다. 앱의 새 작품과 같은 폴더가 생긴다(1부와 첫 회차).
- Windows 설치판에는 아직 들어 있지 않다. Rust로 직접 빌드한다.

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
| `create_project` | 새 작품 폴더 |

명령줄에서는 `writer-mcp call <도구> '<JSON 인자>' [--project <폴더>]`로 같은 도구를 부른다. 인자 자리에 `-`를 주면 표준 입력에서 JSON을 읽는다. `writer-mcp tools`는 도구 목록을 보인다.

## 원고를 지키는 규칙

- **고치기 전 기록**: `write_doc`은 바꾸기 직전 본문을 `manual` 기록(이름 "Claude가 고치기 전")으로 남긴다. 가장 최근 기록과 같으면 다시 남기지 않는다. 작가는 앱의 기록에서 언제든 되돌린다. 손으로 보관한 기록과 같은 종류라 자동으로 지워지지 않는다.
- **다른 곳에서 바뀐 본문**: `base_rev`를 주면 `read_doc` 뒤에 앱이나 다른 기기에서 본문이 바뀐 경우 쓰지 않는다. 저장하는 순간에도 `save_body`의 지문 확인을 거쳐, 그사이 바뀌면 쓰려던 글을 `this-device` 기록으로 남기고 멈춘다.
- **잠근 회차**(완료 회차 잠금)는 고치지 않는다.
- **서식**: 본문은 평문으로 오가므로 굵게·방점·메모 구간·문단 여백을 실어 나르지 못한다. 서식이 있는 회차를 `replace`로 바꾸려 하면 거절하고, 작가가 확인한 뒤 `allow_plain: true`로만 바꾼다. `append`는 기존 서식을 그대로 둔다.
- **언제 쓰나**: 서버가 `initialize`에서 도우미에게 주는 안내(`instructions`)에 "작가가 넣어 달라거나 쓰라고 했을 때만 본문을 쓴다"를 넣었다. 후보와 제안은 채팅이나 메모로 남긴다([context-sentences.md](context-sentences.md) "후보를 내고 작가가 고른다").
- 도우미가 쓴 저장은 창작 일지에 남지 않는다. 일지는 앱이 기기 id를 정했을 때만 쓰고(`journal::set_device`), `writer-mcp`는 정하지 않는다. 일지는 AI 사용 여부를 다루지 않으므로([creation-proof.md](creation-proof.md)) 도우미의 저장을 따로 표시하지도 않는다.

## 한계

- 줄바꿈만 있는 문단 안 줄(Shift+Enter)은 `read_doc`에서 줄로 나뉘어 보이고, 그대로 `replace`하면 문단으로 나뉜다.
- 회차 옮기기, 부 만들기, 휴지통, 찾기/바꾸기, 내보내기는 아직 도구가 없다. 앱에서 한다.
- 시험: `cargo test -p writer-mcp` (작품을 만들어 쓰고 읽기, 기록, 지문 충돌, 잠금, 서식, 카드·메모, JSON-RPC 응답).
