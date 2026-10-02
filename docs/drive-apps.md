# 드라이브 앱 등록 (2026-09-28)

앱이 작가의 Google Drive·OneDrive·Dropbox에 직접 연결하려면(휴대폰에서 쓸 때 필요), 각 회사에 WriterProgram을 한 번 등록해서 "앱 아이디"를 받아야 한다. 등록은 운영자 계정으로 약관에 동의하는 일이라 사람이 직접 한다. 받은 값을 `drive-apps.json`에 넣으면 앱의 "기기 간 맞추기"에서 해당 드라이브의 "연결" 버튼이 켜진다. 코드는 이미 들어 있다 ([architecture.md](architecture.md) "드라이브 직접 연결").

## 넣는 곳

`drive-apps.json`을 앱 설정 폴더에 둔다.

- Windows: `%APPDATA%\com.writerprogram.desktop\drive-apps.json`
- macOS: `~/Library/Application Support/com.writerprogram.desktop/drive-apps.json`

```json
{
  "google": { "clientId": "1234567890-abc.apps.googleusercontent.com", "clientSecret": "GOCSPX-..." },
  "onedrive": { "clientId": "00000000-0000-0000-0000-000000000000" },
  "dropbox": { "clientId": "앱 키" }
}
```

배포판에는 빌드할 때 환경 변수로 넣을 수도 있다: `WRITER_GOOGLE_CLIENT_ID`, `WRITER_GOOGLE_CLIENT_SECRET`, `WRITER_ONEDRIVE_CLIENT_ID`, `WRITER_DROPBOX_APP_KEY`. 파일이 있으면 파일이 먼저다.

설치형 앱의 "client secret"은 비밀이 아니다(앱 안에 들어 있어 누구나 꺼낼 수 있음). 구글도 데스크톱 앱에서는 그렇게 본다. 보안은 PKCE와 사용자의 로그인·허용으로 지킨다.

## Google Drive

1. [Google Cloud Console](https://console.cloud.google.com/)에서 새 프로젝트 만들기 (이름 예: WriterProgram).
2. API 및 서비스 → 라이브러리 → **Google Drive API** 사용 설정.
3. OAuth 동의 화면(Google 인증 플랫폼):
   - 사용자 유형: 외부. 앱 이름 WriterProgram, 지원 이메일, 개발자 연락처.
   - 데이터 액세스(범위): `.../auth/drive.file`, `openid`, `email`, `profile`. `drive.file`은 앱이 만든 파일만 보는 권한이라 민감 범위가 아니다.
   - 테스트 사용자: 시험할 구글 계정을 넣는다 (테스트 단계에서는 여기 넣은 계정만 로그인됨, 최대 100명).
4. 사용자 인증 정보 → 사용자 인증 정보 만들기 → **OAuth 클라이언트 ID** → 애플리케이션 유형 **데스크톱 앱**. 만든 뒤 나오는 클라이언트 ID와 클라이언트 보안 비밀을 `google.clientId`, `google.clientSecret`에 넣는다.
5. 주의: 게시 상태가 "테스트"인 동안에는 로그인이 7일마다 풀린다(구글 정책). 실제로 쓰기 시작할 때 "앱 게시"로 바꾼다. `drive.file`만 쓰면 구글의 보안 심사는 필요 없고, 앱 이름·로고를 보이려면 브랜딩 확인을 받는다.

돌아오는 주소는 `http://127.0.0.1:<빈 포트>/`이다. 데스크톱 앱 유형은 따로 등록하지 않아도 된다.

## OneDrive (Microsoft)

1. [Microsoft Entra 관리 센터](https://entra.microsoft.com/) → 앱 등록 → 새 등록.
   - 이름 WriterProgram.
   - 지원되는 계정 유형: **모든 조직 디렉터리의 계정 및 개인 Microsoft 계정**.
   - 리디렉션 URI: 플랫폼 **퍼블릭 클라이언트/네이티브(모바일 및 데스크톱)**, 값 `http://localhost`.
2. 개요의 **애플리케이션(클라이언트) ID**를 `onedrive.clientId`에 넣는다. 클라이언트 비밀은 만들지 않는다.
3. API 권한 → Microsoft Graph → 위임된 권한: `Files.ReadWrite.AppFolder`, `User.Read`, `offline_access`.
4. 인증 → "퍼블릭 클라이언트 흐름 허용"은 꺼 둬도 된다(앱은 PKCE를 쓰는 권한 부여 코드 흐름).

`http://localhost`로 등록하면 포트는 아무거나 받아 준다. 파일은 작가 OneDrive의 `앱/WriterProgram` 폴더에 들어간다.

## Dropbox

1. [Dropbox App Console](https://www.dropbox.com/developers/apps) → Create app.
   - Scoped access, 접근 범위 **App folder**, 이름 WriterProgram (작가 Dropbox의 `앱/WriterProgram` 폴더 이름이 됨).
2. Permissions 탭: `account_info.read`, `files.metadata.read`, `files.content.read`, `files.content.write` → Submit.
3. Settings 탭:
   - OAuth 2 Redirect URIs에 `http://localhost:53682/` 추가 (Dropbox는 포트까지 똑같아야 해서 이 번호를 쓴다).
   - Allow public clients (Implicit Grant & PKCE): Allow.
4. App key를 `dropbox.clientId`에 넣는다. App secret은 쓰지 않는다.
5. 개발 단계(Development)에서는 연결할 수 있는 사용자 수에 한도가 있다. 실제로 쓰기 시작할 때 Production 신청을 한다.

## 계정 없이 시험하기

`crates/sync`의 흉내 드라이브 서버로 앱의 연결·맞추기를 끝까지 돌려 볼 수 있다.

```bash
cargo run -p writer-sync --example stand_in -- dropbox 8765
```

앱을 `WRITER_DROPBOX_ENDPOINT=http://127.0.0.1:8765`, `WRITER_SIGN_IN=fetch` 환경 변수로 띄우고, `drive-apps.json`에 `{"dropbox": {"clientId": "stand-in"}}`를 넣으면 "연결"이 바로 통과한다. 로그인 창 대신 앱이 흉내 서버의 로그인 주소를 직접 방문한다. 흉내 서버가 꺼지면 올린 파일도 사라진다.

## 드라이브 공간 (2026-10-02 결정, 2026-10-03 구현)

원고는 텍스트라 작지만, 작가의 드라이브가 다른 파일로 가득 차는 일은 흔하다(Google 무료 15GB는 Gmail·사진과 같이 쓴다). 차면 동기화가 조용히 멈추고 작가는 백업되는 줄 안다. 이것을 막는다.

| 방식 | 할 일 |
|---|---|
| 폴더 동기화 (PC의 드라이브 폴더) | 클라우드 공간은 드라이브 앱이 알린다. 우리는 PC 디스크 여유만 확인한다 |
| 직접 연결 (모바일, 앱 안 연결) | ① 공간 부족 오류를 정확한 말로 알린다 ② 여유가 적으면 미리 알린다 ③ 실패해도 원고는 기기에 남기고 나중에 다시 올린다 |

- **오류 말** (`writer-sync`의 `providers/session.rs` `drive_error`, `space_full`): 세 드라이브 모두 "드라이브 공간이 가득 참. 원고는 이 기기에 그대로 있고, 공간이 생기면 다시 올립니다."로 알린다. OneDrive는 507, Google은 403에 `error.errors[].reason`(또는 새 형식의 `error.details[].reason`)이 `storageQuotaExceeded`·`quotaExceeded`, Dropbox는 409에 `error_summary`가 `insufficient_space`를 담은 것(올리기는 `path/insufficient_space/..`). Dropbox 올리기의 409를 전에는 모두 "그 사이 바뀜"으로 읽었는데, 이제 공간 부족은 오류로 멈춘다. 덤으로 Google의 403 `rateLimitExceeded`·`userRateLimitExceeded`는 "드라이브가 잠시 바쁨"으로 읽는다.
- **미리 알리기** (`providers::space`, `accounts::sync_project`): 맞추기 전에 남은 공간을 가볍게 읽는다. 남은 공간이 50MB와 작품 크기(원고·기록·휴지통)의 5배 중 큰 쪽보다 적으면 맞추기 결과(`Report.space_left`)에 남은 바이트를 담고, 화면은 하루 한 번 "드라이브 남은 공간이 ○○MB뿐입니다. 공간이 차면 맞추기가 멈춥니다."를 띄운다. 읽기에 실패하거나 거절되면 알림 없이 그대로 맞춘다.

  | 드라이브 | 읽는 곳 | 우리 권한으로 |
  |---|---|---|
  | Google | `about.get?fields=storageQuota(limit,usage)` | 읽힌다. 문서상 `drive.file`로 `about.get`을 부를 수 있다(계정 이름도 같은 곳에서 읽음). `limit`이 없으면(무제한) 알리지 않음 |
  | Dropbox | `users/get_space_usage` | 읽힌다. 앱이 요청하는 `account_info.read`로 된다. 팀 공간은 사람마다 한도가 있으면 그것, 없으면 팀 전체로 계산 |
  | OneDrive | `GET /me/drive?$select=quota` | **확실하지 않음.** Microsoft 문서는 최소 권한을 `Files.Read`로 적는다. 앱 폴더 권한(`Files.ReadWrite.AppFolder`)만으로 거절되면 "모름"으로 두고 507 오류 처리만 남는다. 실제 계정으로 확인 필요 |
- **실패해도 원고는 그대로**: 올리기가 공간 부족으로 실패하면 그 맞추기는 멈추고(링크에 이유가 남아 상태줄·작품 설정에 보임), 이 기기 파일은 건드리지 않는다. 이미 올라간 파일만 Base에 남으므로 못 올린 파일은 다음 맞추기에서 다시 올린다(`tests/providers.rs`의 `*_filling_up`이 세 드라이브에서 확인).
- **우리 쪽 크기 관리** (`writer_core::project::sizes`, `snapshot::tidy`): 작품 설정 "기본 정보"에 "이 작품 크기: 원고 3.1MB · 기록 41MB · 휴지통 0.2MB"(교정 주고받기 `.exchanges/`, 창작 일지 `.journal/`이 있으면 그것도)를 보인다. 원고 = `project.json`·회차·기획·설정집·메모. 기록이 20MB를 넘고 원고의 10배를 넘으면 "오래된 자동 기록 정리" 버튼을 보이고, 확인 창에서 비울 크기를 알린다. 정리는 2주보다 오래된 자동 기록(`auto`)을 지우되 문서마다 가장 최근 자동 기록 하나는 남기고, 원래 규칙(`auto`·`before-reload` 90일)에 걸린 것도 함께 지운다. 직접 보관한 기록과 바꾸기·되돌리기·퇴고·다른 기기 관련 안전 기록은 건드리지 않는다.
- **PC 디스크**: 같은 자리에서 작품 폴더가 있는 디스크의 남은 공간이 200MB보다 적으면 "이 컴퓨터에 남은 공간이 ○○MB뿐입니다…"를 보인다. Windows는 이미 쓰던 `windows-sys`의 `GetDiskFreeSpaceExW`, macOS·모바일은 `libc::statvfs`(Tauri가 이미 쓰는 crate라 새로 받는 것은 없음).
- **흉내 서버**: `POST /stand-in/room`에 `{"total": 바이트 또는 null, "elsewhere": 다른 파일이 차지한 바이트, "readable": false면 남은 공간을 알려 주지 않음}`을 보내면 세 흉내 드라이브가 그만큼 찬 것처럼 답하고, 넘치면 각 드라이브의 공간 부족 오류를 돌려준다.
