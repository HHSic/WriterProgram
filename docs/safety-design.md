# 원고 안전과 한글 입력 보완 설계 (2026-10-03, S1~S7·K1·K2 구현, K1은 실제 입력기 확인 남음)

[feature-gap-report.md](feature-gap-report.md) 3.1·3.2·5장에서 찾은 문제를 고치는 설계. 원칙은 하나다: **어떤 실패에도 작가의 글은 남고, 무엇이 안 됐는지 작가가 안다.** 코드 위치는 2026-10-03 기준.

| 번호 | 문제 | 심각도 | 크기 | 상태 |
|---|---|---|---|---|
| S1 | 정리 작업 하나가 실패하면 작품이 열리지 않음 | 높음 | 작음 | 완료 |
| S2 | 동기화 중 잠깐 못 읽은 파일을 지운 것으로 보고 드라이브에서 지움 | 높음 | 작음~중간 | 완료 |
| S3 | 창작 일지가 저장마다 커지고 매분 통째로 다시 올라감 | 높음 | 중간 | 완료 |
| S4 | 읽을 수 없는 회차가 알림 없이 사라짐 | 중간~높음 | 중간 | 완료 |
| S5 | project.json이 깨지면 복구할 길이 없음 | 중간 | 중간 | 완료 |
| S6 | 크게 지운 글이 기록에 남지 않을 수 있음 | 중간 | 작음 | 완료 |
| S7 | 저장 실패 뒤 다시 시도 없음, 실패 표시 덮어씀, 고친 값 버림 | 중간 | 중간 | 완료 |
| K1 | 한글 조합 중 꾸밈 다시 계산, 다른 기기 글 불러오기 | 높음 | 중간 | 완료 (사람 확인 남음, [ime-checklist.md](ime-checklist.md)) |
| K2 | 대화상자 Esc·바깥 누르기·포커스 | 중간 | 작음 | 완료 |

## S1. 정리 작업은 실패해도 작품을 연다

**지금**: `project::open`(`crates/core/src/project/overview.rs` 111행~)이 `trash::purge(...)?`와 `snapshot::prune(...)?`를 `?`로 부른다. 동기화 프로그램이나 백신이 파일 하나를 잡고 있으면 지우기가 실패하고, 그 오류가 작품 열기 전체를 막는다.

**설계**
- `purge`와 `prune`이 오류를 위로 올리지 않고 `Cleanup { removed: usize, failed: Vec<PathBuf> }`를 돌려준다. 못 지운 항목은 건너뛰고 다음 항목으로 간다. 다음에 작품을 열 때 다시 시도된다.
- `store.rs`에 `remove_file_retry`, `remove_dir_all_retry`를 둔다. `rename_retry`와 같은 방식(Windows 공유 위반 때 짧게 몇 번 다시)이다. 휴지통 이름 바꾸기(`trash.rs`)도 `rename_retry`로 바꾼다.
- `open`에서 정리는 맨 마지막에 하고, 결과는 화면에 띄우지 않고 로그에만 남긴다(작가가 할 일이 없음).
- `copies::reconcile`, `merge_project_copies`, `repair`의 오류는 지금처럼 올린다. 이것들은 원고를 지키는 단계라 실패를 숨기면 안 된다. 대신 오류 문장을 작가 말로 바꾼다(S5와 같이).

**시험**: 읽기 전용 속성이나 열린 핸들로 지울 수 없는 기록·휴지통 항목이 있어도 `open`이 성공하고, 다른 항목은 지워진다.

**구현 (2026-10-03, 완료)**
- `store::Cleanup`, `store::remove_file_retry`·`remove_dir_all_retry`(이미 없는 파일은 지운 것으로 침). `trash::purge`·`snapshot::prune`이 `Cleanup`을 돌려주고, 휴지통 옮기기·되살리기·영구 삭제와 `snapshot::remove_all`도 재시도한다.
- 정한 것: 휴지통 항목 폴더를 지울 때 `item.json`을 맨 나중에 지운다. 문서 파일만 잡혀 있을 때 `item.json`이 먼저 지워지면 항목이 목록에서 사라지고(읽을 수 없는 항목) 다음 정리에서도 빠지기 때문이다.
- 원고를 지키는 단계의 오류 문장: "다른 기기의 작품 구조 파일을 되살리지 못함", "동기화 프로그램이 남긴 사본을 정리하지 못함", "다른 기기의 작품 구조를 합치지 못함", "작품 구조를 원고 파일과 맞추지 못함" 뒤에 이유와 파일 이름. 화면에는 "작품을 열지 못함 · …"로 나온다. `project.json` 자체를 읽지 못하는 경우는 S5에서 다룬다.
- 시험: `crates/core/tests/project.rs`의 `open_succeeds_when_cleanup_cannot_remove_a_file`, `clearing_reports_what_it_could_not_remove`, `steps_that_guard_the_manuscript_still_stop_opening_in_plain_words`. 파일을 잡는 흉내는 Windows에서 공유 없이 연 핸들, 다른 곳에서는 쓸 수 없는 폴더.

## S2. 동기화: 모르는 파일은 건드리지 않는다

**지금**: `crates/sync/src/engine.rs`의 `scan`이 `metadata()`나 `read()`에 실패하면 그 파일을 `continue`로 건너뛴다. 그러면 base에는 있고 이 기기 목록에는 없는 상태가 되어 "이 기기에서 지움"으로 판단하고 `remote.remove`를 부른다. 디렉터리 항목 오류(`filter_map(|e| e.ok())`)도 조용히 빠진다.

**설계**
- `scan`이 `here`와 함께 `unknown: BTreeSet<String>`(경로)과 `unknown_dirs: Vec<String>`(항목을 다 읽지 못한 폴더)을 채운다.
  - metadata·read 실패 → 그 경로를 `unknown`에.
  - 디렉터리 항목 오류가 하나라도 있으면 → 그 폴더를 `unknown_dirs`에.
- 맞추기에서 `unknown`에 있거나 `unknown_dirs` 아래에 있는 경로는 **올리기·내려받기·지우기를 모두 하지 않고** `report.later`에 넣는다. base도 바꾸지 않는다.
- **한꺼번에 많이 지우기 막기**: 한 번의 맞추기에서 드라이브 쪽 지우기가 20개 이상이거나 맞추는 파일의 30% 이상이면, 지우기만 멈추고 나머지는 진행한다. 화면에 "이 기기에서 회차 파일 32개가 없어져 드라이브에서도 지우려 합니다. 맞나요?"를 띄우고 작가가 확인하면 지운다. 실수로 폴더를 옮기거나 지웠을 때 다른 기기까지 번지는 것을 막는다. 같은 규칙을 드라이브 → 이 기기 방향 지우기(`remove_here`)에도 둔다.

**시험**: 읽기 실패를 흉내 내는 파일(잠금, 권한)이 있을 때 흉내 드라이브에서 그 파일이 지워지지 않는다. 지우기 30개 시나리오에서 확인 전까지 아무것도 지워지지 않는다.

**구현 (2026-10-03, 완료)**
- `engine.rs`의 `Scan { files, unknown, unknown_dirs }`. metadata 오류가 나면 그 이름을 파일로도 폴더로도 모르는 것으로 둔다. 하위 폴더를 열지 못해도 `unknown_dirs`. 읽는 사이 없어진 파일(NotFound)은 지금처럼 없는 것으로 본다.
- 정한 것: 작품 폴더 자체를 읽지 못하거나(없음 포함) 그 항목을 다 읽지 못하면 맞추기 전체를 오류로 멈춘다. 예전에는 작품 폴더가 없으면 빈 폴더로 보아 모두 지운 것으로 판단할 수 있었다.
- 정한 것: 이 기기 파일을 지우기·덮어쓰기 직전 다시 읽을 때 읽지 못하면(없는 것과 구별) 건드리지 않고 `later`. 지우기가 실패해도 맞추기 전체를 멈추지 않고 `later`.
- 많이 지우기: `too_many_removals(개수, 지난번 맞춘 파일 수)` = 20개 이상, 또는 5개 이상이면서 30% 이상. 정한 것: 30% 규칙에 "5개 이상"을 더했다. 갓 만든 작은 작품(파일 몇 개)에서 회차 두어 개만 지워도 묻게 되는 것을 막는다. 개수는 기록·휴지통·일지 파일까지 모두 센다(묻는 문장은 "회차 파일 30개, 기록 5개"처럼 종류별로 보인다, `lib/syncText.ts`).
- 지우기는 다른 일을 다 한 뒤에 모아서 한다(개수를 먼저 세려고). 휴지통으로 보낸 회차는 새 자리가 먼저 올라가고 옛 자리가 나중에 지워진다.
- 확인 흐름: 명령 `project_sync`에 `choices: { remove, keep }`를 더했다(새 명령 없음). [지우기]는 멈춘 경로를 `remove`로 보내 그 경로만 기준 없이 지운다. 그 사이 새로 생긴 지우기는 다시 기준에 따라 멈춘다. [되살리기]는 `keep`으로 보내 Base에서 그 경로를 잊게 하고, 같은 맞추기에서 남은 쪽 파일로 다시 채운다(드라이브에서 내려받거나 이 기기 것을 다시 올림). [나중에]는 창만 닫고, 조용한 맞추기는 멈춘 목록이 바뀔 때까지 다시 묻지 않는다. 지금 맞추기를 누르면 다시 묻는다.
- 시험: `crates/sync/tests/engine.rs`의 `a_file_that_cannot_be_read_is_not_removed_on_the_drive`, `a_folder_that_cannot_be_read_is_left_alone`(Unix만), `many_removals_wait_for_the_writer`(30개, 두 방향), `held_removals_can_be_taken_back`. 화면은 `lib/syncText.test.ts`, `store/drives.test.ts`.

## S3. 창작 일지: 덜 쓰고, 달마다 나눈다

**지금**: 자동 저장마다 `journal::note(Entry::Save)`가 한 줄(약 280바이트)을 붙이고 fsync한다(`doc.rs` `save_body`, `journal.rs` `append_at`). 하루 2시간 쓰면 약 0.5MB, 한 해면 수십~수백 MB가 될 수 있다. 일지는 기기마다 파일 하나(`.journal/<기기>.jsonl`)라, 드라이브 맞추기가 매분 통째로 다시 올린다.

**설계**

1. **저장 줄 묶기**
   - `journal.rs`에 회차별 대기 묶음을 둔다(작품 경로 + 회차 → `PendingSave { first, last, saves, added, removed, body, chars }`).
   - 저장할 때는 묶음만 갱신한다(파일 쓰기 없음). 마지막 본문 지문과 글자 수는 덮어쓰고, 늘어난·줄어든 글자 수는 더한다.
   - 묶음을 일지에 한 줄로 쓰는 때:
     - 묶음을 시작한 지 10분이 지났을 때
     - 다른 회차를 저장할 때
     - 다른 종류의 줄(기록, 가져오기, 교정, 시각 고정)을 쓰기 직전. 순서와 사슬이 맞게 하려는 것
     - 작품을 닫거나 앱을 끌 때(`journal::flush_all`, 창 닫기 흐름에 넣음)
   - `Save` 줄에 `saves`(묶인 저장 횟수)와 `since`(첫 저장 시각)를 더한다. 옛 줄은 `saves`가 없으면 1로 읽는다.
   - 앱이 갑자기 꺼지면 마지막 묶음(최대 10분)의 일지 줄이 빠질 수 있다. 원고는 이미 저장되어 있다. creation-proof.md 4.4 한계에 이 문장을 더한다.
   - 하루 2시간 쓰기 기준으로 줄 수가 저장 횟수(수천)에서 묶음 수(수십)로 준다.

2. **달마다 파일 나누기**
   - 새 경로: `.journal/<기기>/<YYYY-MM>.jsonl`.
   - 새 달 첫 줄의 `prev`는 지난 파일 마지막 줄의 지문이라, 사슬은 파일을 넘어 이어진다.
   - 옛 `<기기>.jsonl`은 그대로 두고 첫 조각으로 읽는다. 다시 쓰지 않는다.
   - `verify`, `summary`, 증명서, 시각 고정 잎은 조각을 시간순으로 이어 읽는다.
   - 지난 달 파일은 바뀌지 않아, 동기화는 크기·시각만 보고 다시 올리지 않는다. 매분 올리는 것은 이번 달 파일뿐이다.

**시험**: 저장 100번이 묶음 몇 줄이 되고 글자 수 합이 맞는다. 다른 종류 줄 앞에 묶음이 먼저 쓰인다. 달이 바뀌어도 `verify`가 통과하고, 옛 단일 파일과 새 조각이 섞여도 통과한다.

**구현 (2026-10-03)**: `journal.rs`(`note`, `flush`, `flush_all`, `flush_due`, 조각 읽기), 앱 `commands/journal.rs`(`journal_flush`, 1분 점검), `lib.rs`(앱 끝), `store/project.ts`(작품 나가기), `App.tsx`(창 닫기). 정한 것:
- **묶음 열쇠는 작품 하나**: 다른 회차를 저장하면 앞 묶음을 쓰므로, 작품마다 대기 묶음은 하나뿐이다. 작품 폴더 경로로 나눈다.
- **10분**: 첫 저장에서 10분이 된 저장이 묶음에 들어간 뒤 바로 쓴다. 저장이 멈춰도 앱이 1분마다 `flush_due`로 10분 넘은 묶음을 쓴다. 그래서 묶음 하나는 10분 남짓을 넘지 않는다.
- **줄 시각**: `time`은 마지막 저장 시각, `since`는 첫 저장 시각. 묶음은 다음 줄보다 먼저 쓰여 줄 시각 순서가 유지된다.
- **직접 쓰는 줄**: `journal::append`(편집기의 쓰기 묶음·붙여넣기, 시각 고정)는 묶지 않고 바로 쓰되, 같은 작품의 대기 묶음을 먼저 쓴다. 묶는 것은 core의 `note`로 들어오는 저장뿐이다.
- **읽기 전 쓰기**: 증명서(`proof::make`, `preview`), 시각 고정(`anchor::prepare`), 일지 요약·확인 명령은 그 작품의 묶음을 먼저 쓴다. 숫자가 화면과 맞고, 시각 고정 잎이 마지막 저장까지 덮는다.
- **일지 켜고 끄기**: `set_device`가 바꾸기 전에 모든 묶음을 그 묶음의 기기로 쓴다.
- **달 이름**: 줄 시각의 UTC 달. 시계를 되돌려 지난 달 시각이 나와도 가장 새 조각에 써서 지난 조각은 절대 바뀌지 않는다. 앞 조각 끝 줄이 끊겼으면 그 줄을 닫지 않고(지난 조각을 고치지 않으려고) 그 줄에 사슬을 잇는다.
- **확인 결과 모양**: `FileCheck`는 그대로 기기마다 하나, 줄 수와 처음 어긋난 줄 번호는 조각을 이어 센다. 화면은 바꾸지 않았다.
- **중간 지문**: 묶음 안 중간 저장의 본문 지문은 남지 않는다. 증명에 필요한 것(흐름, 날짜별 분량, 시각 고정)은 그대로이고, creation-proof.md 4.4에 적었다.
- **동기화**: 엔진은 바꾸지 않았다. 지난 달 조각은 크기·시각이 같아 base의 지문을 그대로 쓰고 올리지 않는다(시험 `each_device_journal_travels_without_copies`).

## S4. 읽을 수 없는 회차를 알린다

**지금**: `listed_docs`(`overview.rs`)가 `read_doc(&path).ok()?`로, 읽을 수 없는 파일을 조용히 뺀다. 찾기(`search.rs`), 등장 위치(`cards.rs`), 증명서(`proof/facts.rs`), 시각 고정 잎(`anchor/mod.rs`)도 같다. 메모장에서 ANSI로 저장한 파일이 그렇게 사라진다.

**설계**
- 파일이 **없는** 회차(다른 기기에서 오는 중일 수 있음)와 **있지만 읽을 수 없는** 회차를 나눈다.
- `Overview`에 `unreadable: Vec<UnreadableDoc { id, section, title_guess, reason }>`를 더한다. 제목은 project.json에 없으면 파일 이름에서 짐작한다.
- 트리: 원래 자리에 흐리게 "열 수 없음" 표시를 단다. 누르면 "고쳐 열기" 창을 띄운다.
  - 가져오기의 글자 인코딩 판별(`import/text.rs` `decode`: UTF-8, UTF-16, EUC-KR)로 읽어 미리 보여 준다.
  - 작가가 맞다고 하면 원본을 `manuscript/<id>.md.broken-<시각>`으로 남기고 UTF-8로 다시 쓴다.
  - 앞머리(front matter)가 깨졌으면 본문만 살려 새 앞머리를 붙인다.
- 찾기·등장 위치: 결과에 `skipped: usize`를 더해 "열 수 없는 회차 2개는 빼고 찾았습니다"를 보인다.
- 증명서: 범위 안에 읽을 수 없는 회차가 있으면 만들기 전에 알리고, 증명서에도 "빠진 회차"로 적는다. 조용히 빼지 않는다.
- 내보내기: 지금처럼 실패하되, 어느 회차인지 이름을 알려 준다(`export.rs`).

**시험**: EUC-KR 파일, 앞머리가 깨진 파일, 0바이트 파일이 각각 `unreadable`에 나오고, 고쳐 열기 뒤 정상 회차가 된다. 원본이 남는다.

**구현함 (2026-10-03)**: `doc::Damage`·`check_bytes`(엄격해진 `read_doc`), `mend.rs`(읽기·미리 보기·고쳐 쓰기, 시험 포함), `Overview.unreadable`, 찾기·바꾸기·등장 위치 `skipped`, 증명서 `Missing`("빠진 회차"), `export::load_item`. 화면은 `workspace/UnreadableItem.tsx`, `MendDialog.tsx`, `lib/unreadable.ts`. 정한 것: "앞머리가 깨짐"은 `---` 다음 줄이 `키: 값`인데 닫는 `---`가 없을 때만이다(손으로 쓴 글이 `---` 장면 나눔으로 시작해도 읽음). 파일을 다른 프로그램이 잡고 있어 못 읽으면 `unreadable`에 넣되 고쳐 열기는 막는다(`repairable: false`). 트리에서 읽을 수 없는 회차는 번호를 받지 않는다(화면의 번호와 내보내기 제목이 어긋나지 않게).

## S5. project.json 백업과 복구

**지금**: `project::load`(`project/mod.rs`)가 serde 오류를 그대로 내서, 화면에 "파일을 읽을 수 없음 · project.json (expected value…)"처럼 영어가 섞인다. 복구는 파일이 아예 없을 때 동기화 사본을 가져오는 경우뿐이다.

**설계**
- **백업**: `project::save`가 쓰기에 성공하면 같은 내용을 `.backup/project-<YYYY-MM-DD>.json`에도 원자적으로 쓴다(하루 한 파일, 같은 날은 덮어씀). 최근 7일 치만 남긴다. `.backup/`은 동기화 대상이라 다른 기기에서도 쓸 수 있다.
- **깨짐 알아보기**: `load`가 JSON 오류일 때 `Error::ProjectDamaged`를 낸다. 영어 원문은 로그에만 남긴다.
- **복구 화면** (작품 열기에서 `ProjectDamaged`를 받으면): "작품 구조 파일(project.json)이 손상되어 열 수 없습니다. 원고 파일은 그대로 있습니다." 선택지는 셋이다.
  1. **가장 최근 백업으로 되돌리기**: 백업 날짜와 부·회차 수를 보여 준다. 백업 뒤에 생긴 회차 파일은 `repair`가 이미 하는 것처럼 마지막 부 끝에 붙인다.
  2. **원고 파일로 구조 다시 만들기**: 백업도 없을 때. `manuscript/`의 모든 회차를 앞머리의 만든 시각 순으로 부 하나에 넣고, 기획 문서와 설정 카드는 폴더에서 다시 모은다. 작품 설정(서식, 목표)은 기본값으로 둔다.
  3. **닫기**
- 어느 쪽이든 깨진 파일은 `project.json.damaged-<시각>`으로 남기고 지우지 않는다.
- 동기화 사본(`project (1).json` 등)이 있으면 1번 위에 "다른 기기 사본으로 되돌리기"를 먼저 보인다(`copies::merge`가 이미 읽을 수 있음).

**시험**: 반쯤 잘린 project.json, 빈 파일, 다른 JSON에서 각각 복구 화면 → 백업 복구 → 회차 수가 맞는다. 백업이 없을 때 다시 만들기로 모든 회차가 트리에 나온다.

**구현함 (2026-10-03)**: `project/backup.rs`, `project/recover.rs`(`recovery`, `recover`, 시험 포함), `Error::ProjectDamaged`, 동기화 `HIDDEN_KEPT`에 `.backup`. 명령 `project_recovery`·`project_recover`, 화면 `screens/RecoverDialog.tsx`(작품 열기가 실패하면 `project_recovery`로 깨졌는지 묻고 복구 창을 띄움). 백업 날짜는 이 기기 날짜다. 다시 만들 때 제목·종류·id는 깨진 파일 앞부분(`"parts"` 앞)에 남은 값을 살리고, 없으면 폴더 이름·웹소설·새 id. 카드의 분류 중 기본 셋에 없는 것은 "되살린 분류 N"으로 다시 만든다(이름은 잃음).

## S6. 크게 지우기 전 기록

**지금**: 자동 기록은 마지막 기록이 10분보다 오래됐을 때만 남는다(`snapshot::auto_if_due`). 기록 뒤 9분 동안 쓴 2천 자를 전체 선택해 지우면, 자동 저장 뒤에는 편집기 되돌리기(메모리)에만 남는다.

**설계**
- `save_body`에서 저장 전 글자 수(`before`)와 새 글자 수를 비교한다. 줄어든 양이 **500자 이상이거나, 200자 이상이면서 20% 이상**이면 시간과 상관없이 저장 전 본문을 새 종류 `before-shrink` 기록으로 남긴다(같은 내용의 최신 기록이 있으면 건너뜀, `keep_unless_same`).
- 화면 이름은 "크게 지우기 전". 자동 기록처럼 90일 뒤 정리된다(`PRUNED`에 넣음).
- 다른 기기 글로 바뀌는 경우는 이미 `other-device` 기록이 있으므로 해당 없음.

**시험**: 기록 직후 1,000자를 지우고 저장하면 `before-shrink`가 생기고 되돌릴 수 있다. 100자 지우기는 기록하지 않는다.

**구현 (2026-10-03, 완료)**
- `snapshot::shrinks_a_lot(before, after)`와 상수 `SHRINK_CHARS`(500)·`SHRINK_SOME_CHARS`(200)·`SHRINK_PERCENT`(20). 글자 수는 공백 포함. `save_body`는 크게 줄면 자동 기록 대신 `before-shrink`를 남긴다(같은 글이라 둘 다 남길 까닭이 없음).
- 화면 이름 "크게 지우기 전"(`lib/labels.ts`), 브라우저 미리보기 흉내(`api/mock/docs.ts`)도 같은 규칙.
- 시험: `crates/core/tests/project.rs`의 `a_big_deletion_keeps_the_text_before_it`, `a_big_deletion_record_is_cleared_like_automatic_ones`.

## S7. 저장 실패를 놓치지 않는다

**지금**
- `editor/session.ts`: 실패하면 `dirty`만 다시 켜고 다시 시도하는 타이머가 없다. 다음 입력을 기다리는 동안 앱이 죽으면 글을 잃는다.
- 저장 상태(`save`)가 앱 전체에 하나라, 다른 창이나 카드 저장 성공이 실패 표시를 덮는다(`session.ts`, `CardEditor.tsx`).
- 메모 저장 실패는 메모 화면 안에만 표시되고 창 닫기를 막지 않는다(`Notes.tsx`, `store/ui.ts`).
- 제목·시놉시스는 try 전에 `pending.current = {}`로 비워서, 실패하면 고친 값을 버린다(`DocPane.tsx`).

**설계**
- **저장 상태를 대상마다**: `saves: Record<string, SaveState>`(열쇠는 `doc:<id>`, `card:<id>`, `note:<id>`, `meta:<id>`). 상태줄은 그중 가장 나쁜 것을 보인다(오류 > 저장 중 > 저장됨). 성공이 다른 대상의 실패를 덮지 않는다.
- **다시 시도**: 실패하면 2초, 5초, 15초, 30초, 60초 간격으로 다시 시도한다(60초에서 멈추지 않고 계속). 상태줄: "저장하지 못함 · 30초 뒤 다시 시도 [지금 다시]". 원인 문장은 작가 말로 쓴다(디스크 가득 참, 폴더에 쓸 수 없음, 파일이 다른 프로그램에 잡혀 있음).
- **고친 값 버리지 않기**: 제목·시놉시스·카드·메모 모두 보낼 값을 꺼낸 뒤 실패하면 대기 값에 되돌려 합친다(`pending = { ...patch, ...pending }`). 성공한 뒤에만 비운다.
- **창 닫기**: `registerFlusher`에 등록된 모든 대상(본문, 메타, 카드, 메모) 중 하나라도 실패나 대기면 닫기 전에 "저장하지 못한 글이 있습니다" 창을 띄운다. 선택지는 [다시 시도] [비상 저장 위치에 두고 닫기] [닫지 않기].
- **비상 저장**: 같은 대상이 1분 넘게 계속 실패하면(작품 폴더의 드라이브가 끊김, 디스크 가득 참 등) 앱 데이터 폴더 `rescue/<작품 id>/<회차 id>-<시각>.md`에 본문을 써 둔다. 상태줄에 "작품 폴더에 저장하지 못해 비상 위치에 보관했습니다 [위치 열기]"를 보인다. 다음에 작품을 열 때 비상 파일이 지금 회차보다 새로우면 "비상 보관된 글이 있습니다 · 비교하기"를 띄운다(기록 비교 화면을 씀).

**시험**: 저장 명령을 실패시키는 흉내(vitest, 가짜 api)로 다시 시도 간격, 대상별 상태, 메타 값 유지, 닫기 막기를 확인한다. 앱(CDP)에서 작품 폴더를 읽기 전용으로 바꿔 비상 저장을 확인한다.

**구현 (2026-10-03, 완료)**
- 대상별 상태와 다시 시도는 `store/saves.ts`(`registerSave`, `markSaving`·`markSaved`·`markFailed`, `retryNow`, `rescue`, `worstSave`). 본문은 `editor/session.ts`, 제목·시놉시스·카드·메모는 `lib/useDebouncedSave.ts`(`debouncedSaver`, 실패 값 되돌려 합치기 `merge`)가 알린다. 화면이 닫힌 대상도 저장될 때까지 남아 다시 시도한다.
- 상단 표시 `workspace/SaveStatus.tsx`, 원인 문장 `lib/saveReason.ts`. Rust 쪽은 다른 프로그램이 잡은 파일(Windows 공유 위반 32·33, `ResourceBusy`)에 "파일이 다른 프로그램에 잡혀 있음"을 따로 준다(`writer_core::Error::user_message`).
- 비상 보관: core `rescue.rs`, 명령 `commands/rescue.rs`(`rescue_save`, `rescue_list`, `rescue_load`, `rescue_set_aside`, `rescue_folder`). 한 번 이어진 실패는 시각이 같은 한 파일을 덮어 쓰고(최대 30초마다), 처리한 파일은 `old/`로 옮긴다. 작품을 열 때 `store/rescue.ts`가 회차보다 새 파일을 찾아 둘 다 보기(`workspace/Compare.tsx`의 `rescue`)를 권한다. 처음에 쓴 대로 기록 비교 화면이 아니라 둘 다 보기를 썼다(바꾸기·두기 단추가 이미 있음).
- 창 닫기: `confirmClose`(`store/ui.ts`)와 `workspace/CloseAskDialog.tsx`.
- 시험: `store/saves.test.ts`, `lib/saveReason.test.ts`, core `rescue.rs`·`error.rs` 단위 시험. 앱(CDP)에서 읽기 전용 폴더로 비상 저장을 확인하는 일은 남았다.

## K1. 한글 조합 중에는 꾸밈을 다시 계산하지 않는다

**지금**: 조합 중에도 설정집 이름 강조(`editor/cards.ts`), 찾기 강조(`editor/search.ts`), 첫 줄 들여쓰기 꾸밈(`editor/indent.ts`)이 다시 계산된다. 예를 들어 "서하"의 둘째 글자를 조합하는 순간 인라인 꾸밈이 생겨, WebView2에서 조합이 깨질(자모가 쪼개지거나 겹칠) 위험이 있다. 다른 기기 글 불러오기(`editor/shared.ts` `reloadDoc`)도 조합 중인지 보지 않는다.

**설계**
- 공용 플러그인 `composition`을 둔다. `compositionstart`·`compositionend` DOM 이벤트로 조합 상태를 기억하고, 조합이 끝날 때 메타(`composition-end`)를 단 빈 트랜잭션을 하나 보낸다.
- 꾸밈 플러그인 셋은 `apply`에서 조합 중이면 **다시 계산하지 않고 기존 꾸밈을 `tr.mapping`으로 옮기기만** 한다. `composition-end` 메타를 받으면 그때 한 번 다시 계산한다.
- 들여쓰기 꾸밈은 이참에 바뀐 문단과 그 앞뒤 문단만 다시 계산하게 한다. 큰 회차 성능 문제도 함께 줄인다([feature-gap-report.md](feature-gap-report.md) 3.3).
- `reloadDoc`와 교정 반영 뒤 다시 불러오기는 조합 중이면 `compositionend`까지 미룬다.
- 글자 수 세기와 창작 일지의 쓰기 묶음 계산도 조합 중 입력은 조합이 끝난 뒤에 센다.

**한 것 (2026-10-03)** — 사람이 실제 입력기로 확인하는 일만 남음([ime-checklist.md](ime-checklist.md)).
- `editor/composition.ts`: 공용 플러그인. ProseMirror가 조합 중 입력에 붙이는 메타 `composition`으로 조합 중인 트랜잭션을 알아보고, `compositionend` 다음 작업 순서에서 `composition-end` 빈 트랜잭션을 보낸다. 그사이 다음 글자 조합이 시작됐으면 보내지 않는다(그 조합의 끝이 대신함). `afterComposition(view, fn)`은 조합이 끝난 뒤 한 번 실행한다.
- 꾸밈 플러그인 넷(설정집 이름, 찾기, 들여쓰기, 빈칸 부호)이 `decorate.ts` `follow`를 쓴다. 조합 중에는 옮기기만 하고 바뀐 구간을 모아 두었다가, 조합 끝이나 다음 일반 편집 때 그 구간만 다시 계산한다. 소리 내어 읽기 강조는 원래 옮기기만 한다.
- 들여쓰기: 바뀐 문단, 앞 문단 하나, 그리고 바뀐 곳 뒤로 글이 있는 첫 문단까지만 다시 계산한다. 찾기 강조도 바뀐 문단만 다시 찾는다. 145만 자 한 회차(34,326문단)에서 글자 하나 칠 때 들여쓰기 약 5.4초 → 약 5~10ms. 처음 꾸밈 만들기도 들여쓰기 7.4초 → 수십 ms, 설정집 이름 8초 이상 → 0.1~0.2초(ProseMirror `DecorationSet.create`·`map`이 문단 수의 제곱으로 느려져, 회차 최상위 문단별로 직접 짠다). 단, ProseMirror 화면 갱신 자체도 문단 수에 비례하는 일이 남아 있어 큰 회차는 여전히 나누기를 권한다.
- 설정집 이름 낱말 경계: 이름 뒤 글자가 한글 음절이 아니거나 조사·어미 첫 글자(`NAME_ENDINGS`)일 때만 이름으로 본다. "서하늘"의 "서하"는 강조하지 않고 "서하가·서하는·서하의·서하에게"는 강조한다. Rust `cards::find_names`(등장 위치·등장 회차 수)와 화면 `editor/names.ts`가 `crates/core/tests/fixtures/names.json`으로 같은 결과를 확인한다. AI 가리기(`ai/mask.rs`)는 그대로 둔다(덜 가리는 쪽이 위험).
- `reloadDoc`(다른 기기 글, 교정 반영, 사본 바꾸기)는 조합 중이면 조합이 끝날 때까지 미룬다. 기다리는 동안 새로 쓴 글이 있으면 불러오지 않는다: 그 글을 저장할 때 충돌로 잡혀 두 글이 모두 남는다.
- 창작 일지: 조합 중 입력은 조합 하나의 길이 변화로 모아 두었다가 조합이 끝날 때(또는 다음 일반 편집, 묶음 끝) 한 번 센다.
- 남은 것: 상태 막대 글자 수는 `DocPane.tsx`의 150ms 타이머가 센다. 그 파일은 다른 작업에서 고치는 중이라 손대지 않았다. 타이머 안을 `afterComposition(editor.view, …)`로 감싸면 된다.

**확인 목록 (실제 MS 한글 입력기, Windows 11, WebView2)**
1. 설정집 인물 이름("서하") 직전·직후·중간에서 조합하며 입력
2. 찾기 강조가 켜진 상태에서 강조된 낱말 안에 입력
3. 문단 첫머리(들여쓰기 꾸밈)에서 입력, 대화문 따옴표 뒤 입력
4. 조합 중 Enter, Shift+Enter, Backspace, 방향키, 마우스 클릭
5. 조합 중 자동 저장이 도는 순간(입력 멈춤 직전)
6. 조합 중 다른 기기 글 불러오기 알림
7. 2분할에서 한쪽 조합 중 다른 쪽 표시
8. 한자 변환(한자 키), 특수문자(ㅁ+한자)
9. 터치 키보드(태블릿 모드)

이 목록은 CDP 자동 시험으로는 진짜 입력기를 흉내 낼 수 없어, 사람이 한 번 돌리고 결과를 남긴다. 자동 시험에는 `compositionstart/update/end` 이벤트를 흉내 낸 회귀 시험을 둔다.

## K2. 대화상자

**지금** (`components/Modal.tsx`)
- Esc를 `isComposing` 확인 없이 받아서, 한글 조합을 취소하려고 누른 Esc에 창이 닫힌다.
- 바깥을 누르면 바로 닫혀, 가져오기·서식 창에서 고르던 내용을 잃는다.
- Tab 포커스가 대화상자 밖으로 빠지고, 닫은 뒤 원래 자리로 포커스를 돌려주지 않는다.

**설계**
- Esc: `e.isComposing || e.keyCode === 229`이면 무시한다.
- 바깥 누르기: 속성 `dismissOnBackdrop`을 두고 기본값은 끔. 확인·안내만 하는 작은 창만 켠다. 입력이 있는 창(가져오기, 원고 서식, 작품 설정, 증명서, 교정 검토)은 닫기 버튼과 Esc로만 닫는다.
- 고친 내용이 있는 창은 Esc나 닫기에 "고친 내용을 버릴까요?"를 묻는다(`dirty` 속성).
- 포커스: 열 때 이전 포커스를 기억하고 닫을 때 돌려준다. Tab·Shift+Tab을 창 안에서 돈다.

**시험**: vitest(jsdom)로 조합 중 Esc 무시, 바깥 누르기, 포커스 순환과 복귀를 확인한다.

**구현 (2026-10-03, 완료)**: `components/Modal.tsx`. `dismissOnBackdrop`은 확인 창(`ConfirmDialog`)만 켠다. `dirty`는 가져오기(파일을 고른 뒤), 내보내기·편집자에게 보내기, 작품 설정(원고 서식 포함), 증명서 창에 연결했다(`lib/useChanged.ts`: 연 뒤 고른 값이 달라졌는지). "고친 내용을 버릴까요?"는 창 아래에 [계속 고치기] [버리고 닫기]로 뜨고, 한 번 더 누른 Esc는 "계속 고치기"다. 창이 겹치면 맨 위 창만 Esc를 받는다. 교정 검토는 대화상자가 아니라 탭(`ReviewPane.tsx`)이라 해당 없음. 시험은 `components/Modal.test.tsx`(jsdom, devDependency로 추가).

## 진행 순서

1. **S1, S2, S6**: 작고 바로 위험을 줄인다. 함께 한 묶음.
2. **S7, K2**: 화면 쪽 저장·대화상자. 한 묶음.
3. **S3**: 일지 형식이 바뀌므로 증명서·시각 고정 쪽과 같이 본다. creation-proof.md 수정 포함.
4. **S4, S5**: 복구 화면이 둘 다 필요하다. 한 묶음.
5. **K1**: 꾸밈 플러그인 정리 + 실제 입력기 확인 목록 실행(한글 입력기가 있는 Windows에서 사람이 확인).

같은 저장소에서 다른 작업이 진행 중이므로, 각 묶음은 따로 브랜치에서 하고 영향 받는 모듈(`project/`, `sync/engine.rs`, `journal.rs`, `snapshot.rs`, `editor/`, `components/Modal.tsx`)을 미리 알린다.
