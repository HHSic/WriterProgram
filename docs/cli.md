# 터미널 명령 (writer)

`writer`는 AI 도우미 MCP 서버([mcp.md](mcp.md))와 같은 도구를 셸 명령으로 부른다. Claude Code처럼 터미널에서 일하는 도우미는 MCP 연결 없이 `writer help`만 읽고 작품을 다룰 수 있고, 사람도 그대로 쓴다. 같은 `crates/mcp` 라이브러리에서 실행 파일 두 개를 만든다.

| 실행 파일 | 쓰임 |
|---|---|
| `writer-mcp` | MCP 서버(`serve`)와 JSON 인자 명령(`call <도구> '<JSON>'`) |
| `writer` | 사람과 도우미가 치는 명령 (`src/cli.rs`) |

명령은 도구 목록(`tools::list`)에서 만든다. 도구가 늘면 명령도 같이 는다. 점검도 같다. 지침을 읽기 전에는 회차를 쓸 수 없고, 회차 본문에는 맥락 메모가 필요하고, 저장 전 계약 위반은 거절되고, 저장 뒤 경고가 답에 온다.

## 쓰는 법

```bash
writer help                       # 명령 목록과 규칙
writer help write                 # 명령 하나의 인자
writer guide                      # kfiction 작문 지침 (쓰기 전에 한 번)
writer outline                    # 부·회차 목록
writer read <회차 id>              # 본문과 rev
printf '…' | writer write <회차 id> append --base-rev <rev> --pov 하진 --tense past
writer add "3화 · 시장" --pov 하진 --tense past --speech "하진>상인:해요체" --file 3화.md
writer check <회차 id>             # 또는 writer check < 초고.md (작품 폴더 없이)
writer save-reading <회차 id> --args-file 해석.json
```

- **명령 이름**: 자주 쓰는 도구는 짧은 이름이 있다(`read`=read_doc, `add`=add_doc, `write`=write_doc, `meta`=set_meta, `card`=save_card, `note`=add_note, `new`=create_project). 나머지는 도구 이름의 `_`를 `-`로(`replace-all`, `save-reading`).
- **인자**: 도구의 필수 인자(글자·수인 것)는 차례대로 그냥 쓴다. 모든 인자는 `--이름 값`(`_`는 `-`)으로도 된다. 예/아니오 인자는 이름만(`--allow-plain`). 여러 값은 되풀이한다(`--new-facts 가 --new-facts 나`). 객체·배열은 JSON(`--fields '{"나이": "29"}'`). `--args '<JSON>'`, `--args -`(표준 입력), `--args-file`은 인자를 한꺼번에 준다.
- **본문**: `add`, `write`, `check`의 본문은 `--file 경로`, `--text 글`, 또는 파이프·리다이렉트(`< 장면.md`). 표준 입력이 터미널이거나 비어 있으면(`/dev/null`) 기다리지 않는다.
- **맥락 메모**: `--pov 시점인물 --tense past|present`, 대사가 있으면 `--speech 화자>청자:말단계[:호칭]`을 쌍마다. `--context '<JSON>'`도 된다.
- **작품 폴더**: `--project`, 환경 변수 `WRITERPROGRAM_PROJECT`, 아니면 지금 폴더와 그 위에서 `project.json`을 찾는다. 작품 폴더 안에서 치면 따로 줄 필요가 없다.
- **출력**: 글은 그대로, 나머지는 보기 좋은 JSON. `--json`은 언제나 JSON.
- **끝 코드**: 0 됨, 1 도구가 거절하거나 실패(이유는 stderr), 2 명령을 알아듣지 못함.

## 지침 읽기

MCP 서버는 연결 하나 동안 `guide`를 읽었는지 기억한다. `writer`는 부를 때마다 새 프로세스라, `writer guide`를 읽은 때를 상태 폴더(임시 폴더의 `writer-mcp/guide-read`, 또는 `WRITER_MCP_STATE_DIR`)에 적고 12시간 동안 읽은 것으로 친다. 기기 하나에 하나라 그 사이에는 다른 도우미 세션도 읽은 것으로 본다. 거절 횟수(`refusals.json`)도 같은 폴더에 있다.

`writer-mcp call`은 예전처럼 지침을 묻지 않는다(사람이 JSON으로 부르는 낮은 단계 명령).

## PATH 등록

터미널에서 `writer`라고만 쳐서 부르려면 설치 폴더가 PATH에 있어야 한다.

- **설치할 때**: Windows 설치판이 처음 설치할 때 "터미널에서 writer 명령으로 작품을 읽고 쓸 수 있게 할까요?"라고 묻는다(`app/src-tauri/windows/hooks.nsh`). 예를 고르면 설치 폴더를 사용자 PATH(HKCU\Environment, 관리자 권한 없음)에 넣는다. 업데이트(`/UPDATE`)와 아무도 보지 않는 설치(조용히, 수동)는 묻지 않고 그대로 둔다. 지울 때 뺀다.
- **앱에서**: 작품 설정 → 기본 → AI 도우미 연결의 "터미널 명령 (writer)"에서 등록하거나 뺀다(명령 `cli_command`, `cli_path_set`). 예전 버전을 쓰다 업데이트한 사람은 여기서 켠다.
- **명령으로**: `writer path status | add | remove`.
- 등록한 뒤 새로 연 터미널부터 찾는다. 열려 있던 터미널은 다시 연다.
- PATH를 고치는 일은 `writer.exe`가 한다(`crates/mcp/src/path.rs`). NSIS 문자열은 1024자에서 잘려서 긴 PATH를 설치기 안에서 고치면 잘린 채 저장될 수 있다. 다른 항목은 쓰인 그대로(`%USERPROFILE%\…` 포함) 두고 이 폴더만 넣고 뺀다. 대소문자, `/`와 `\`, 끝의 `\`가 달라도 같은 폴더로 본다.
- Windows 밖에서는 PATH를 고치지 않는다. `writer path add`가 셸 설정 파일에 넣을 줄을 알려 준다.

## 설치판에 넣는 방법

배포 워크플로가 `cargo build --release -p writer-mcp --bins`로 두 실행 파일을 만들어 `app/src-tauri/binaries/`에 대상 표기를 붙여 두고, `tauri.mcp.conf.json`이 둘 다 사이드카(`bundle.externalBin`)로, `windows/hooks.nsh`를 설치기 훅(`bundle.windows.nsis.installerHooks`)으로 넣는다. 앱 실행 파일은 `WriterProgram.exe`라 `writer.exe`와 겹치지 않는다.

설치판 없이 쓸 때는 `cargo install --path crates/mcp`로 `writer`와 `writer-mcp`를 함께 깐다.

## 한계

- Windows 레지스트리 코드는 Windows 대상으로 컴파일과 clippy만 확인했다. 설치기 훅은 Tauri 2.12 NSIS 템플릿의 변수(`$UpdateMode`, `$PassiveMode`)와 훅 자리에 맞췄다.
- 지침 읽음은 기기 단위다. 세션마다 묻지 않는다.
