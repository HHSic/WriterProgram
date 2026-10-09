# 배포와 자동 업데이트 (2026-10-09)

Windows 설치 파일을 GitHub Releases로 내보내고, 설치된 앱이 스스로 새 버전을 확인한다. 작가가 업데이트를 선택하면 저장과 서명 검증 후 설치한다. Windows 코드 서명 인증서는 아직 없으며 업데이트 서명은 별도로 사용한다.

## 구성

| 부분 | 위치 | 내용 |
|---|---|---|
| 설치 파일 | `app/src-tauri/tauri.conf.json` `bundle` | NSIS 하나(`.exe`). 관리자 권한 없이 내 계정에만 설치(`currentUser`). WebView2가 없는 PC는 설치 중에 받아 온다. 한국어 설치 화면 |
| 업데이트 확인 | `plugins.updater`, 앱 `commands/update.rs`, 화면 `store/update.ts` | 시작하고 조금 뒤, 하루 한 번 `releases/latest/download/latest.json`을 묻는다. 새 버전이 있으면 알림 하나만 띄우고, 작가가 "바꾸기"를 누르면 원고를 모두 저장한 뒤 받는다. 보기 설정에 버전, "새 버전 확인", 자동 확인 끄기 |
| 업데이트 서명 | 공개 키는 `tauri.conf.json`, 비밀 키는 운영자 PC와 GitHub 비밀값 | 받은 설치 파일의 서명이 공개 키와 맞지 않으면 설치하지 않는다(minisign). 코드 서명과는 다른 것이고 무료 |
| 자동 배포 | `.github/workflows/release.yml` | 정식 버전 태그를 올리면 검사 → 빌드 → 서명 → 초안 릴리스 → 업로드된 파일 서명 검증 → 자동 공개(설치 파일, `.sig`, `latest.json`) |
| 검사 | `.github/workflows/ci.yml` | main에 올릴 때와 PR마다 fmt, clippy, Rust 테스트, 화면 타입 검사·테스트 |

업데이트 확인은 GitHub에 "새 버전이 있나"만 묻는다. 원고나 작품 정보는 보내지 않는다.

## 새 버전 내기

1. 앱 버전을 `app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml`, `app/package.json`, `app/package-lock.json`에 맞춘다. Cargo.lock도 갱신한다. 앱이 보는 버전은 `tauri.conf.json`이다.
2. `docs/releases/v<버전>.md`에 사용자에게 보여 줄 변경 내용을 적는다. 버전·태그·변경 내용 검사를 통과해야 빌드한다.
3. 검증한 변경을 커밋하고 정식 버전 태그를 올린다.
   ```bash
   git tag v0.1.2
   ```
   ```bash
   git push origin v0.1.2
   ```
4. GitHub Actions가 테스트·빌드·서명을 수행하고 먼저 초안을 만든다. 업로드된 설치 파일을 다시 받아 앱 공개 키로 서명을 확인하고 `latest.json`의 버전·다운로드 주소·서명을 검사한 뒤 **자동 공개**한다. 검사에 실패하면 공개하지 않는다.
5. 공개된 릴리스가 최신 버전이 되어 설치된 앱들이 업데이트를 안내한다. 변경 내용 파일의 본문이 릴리스와 업데이트 안내에 함께 들어간다.

수동 실행(`workflow_dispatch`)에서는 이미 존재하는 `v0.1.2` 같은 정식 버전 태그를 입력한다. 브랜치 이름으로 배포하지 않는다. 자동 배포는 현재 정식 버전만 지원한다.

베타 테스터에게 처음 줄 때는 Releases의 `WriterProgram_<버전>_x64-setup.exe`를 받게 한다. 코드 서명 전이라 Windows가 "PC 보호" 창을 띄우면 "추가 정보 → 실행"을 누르도록 안내한다. 그 뒤로는 앱이 알아서 새 버전을 알린다.

## 서명 키

- 비밀 키: 운영자 PC의 `%USERPROFILE%\.tauri\writerprogram.key`(암호는 같은 폴더의 `.password`). 저장소에는 넣지 않는다.
- GitHub 저장소 비밀값(Settings → Secrets and variables → Actions)에 둘 다 넣는다: `TAURI_SIGNING_PRIVATE_KEY`(키 파일 내용), `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **키를 잃으면 이미 설치된 앱에 업데이트를 보낼 수 없다**(새 키로 서명한 파일을 거부함). 키 파일과 암호를 안전한 곳에 따로 백업한다.
- 공개 키를 바꾸면 그 버전부터 새 키만 믿는다. 바꿀 때는 옛 키로 서명한 마지막 버전에 새 공개 키를 넣어 내보내야 이어진다.

## 시험

계정·릴리스 없이 내 PC에서 업데이트를 끝까지 확인할 수 있다(설치는 하지 않음).

- 앱을 `WRITER_UPDATE_ENDPOINT=<latest.json 주소>`로 띄우면 그 주소를 묻는다. `WRITER_UPDATE_DRY=1`이면 받아서 서명까지 확인하고 설치는 건너뛴다.
- 로컬 `http://` 시험 서버를 쓰려면 시험용 빌드에만 `--config '{"plugins":{"updater":{"dangerousInsecureTransportProtocol":true}}}'`를 붙인다. 배포판에는 넣지 않는다.

## 아직 정하지 않은 것

- 코드 서명(Azure Trusted Signing 또는 OV 인증서)과 그 시점.
- macOS(공증), Android, iOS 배포.
- 업데이트 채널(베타·정식 나누기). 지금은 게시된 최신 릴리스 하나만 본다.
