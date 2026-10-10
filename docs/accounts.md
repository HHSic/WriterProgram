# 계정 로그인과 개인 설정 동기화

Supabase 프로젝트 하나로 Google·Kakao 로그인, 이메일·비밀번호 앱 계정, 등록한 회사·학교 SAML SSO를 제공한다. Google·Kakao·이메일 로그인과 설정 DB는 [Free 플랜](https://supabase.com/pricing)으로 시작할 수 있다. [SAML SSO](https://supabase.com/docs/guides/auth/enterprise-sso/auth-sso-saml)는 Pro 이상이다. 무료 플랜의 사용량 한도·프로젝트 일시 중지·메일 발송 제한은 운영 전에 해당 안내를 확인한다.

앱 시작 화면 또는 **보기 → 보기 설정 → 계정·설정 동기화**에서 로그인한다. 로그인하면 화면 글꼴·크기·자간·줄 간격·색, 쪽 보기·눈금자·문단 부호, 편집 자동 변환, 도구줄 탭·접기, 책 미리보기 글자·배경·용지 크기를 불러온다. 바꾸면 약 1초 뒤 저장하며, 창에 돌아올 때와 1분마다 다른 기기의 변경도 확인한다.

원고 파일·원고의 서식·최근 작품 경로·AI 키·드라이브 로그인은 이 DB에 보내지 않는다. 음성 URI와 책 미리보기의 휴대폰/PC 선택은 기기별로 유지한다. 작품 파일 동기화는 기존 공유 드라이브 기능을 사용한다.

## 운영자가 한 번 설정할 것

1. [Supabase](https://supabase.com/dashboard)에서 프로젝트를 만든다. 프로젝트 URL과 **Publishable key**를 확인한다. 기존 `anon` 공개 키도 지원한다. `service_role`·`sb_secret_…` 키는 설치 앱이나 저장소에 넣지 않는다.
2. SQL Editor에서 [`202610100001_personal_settings.sql`](../supabase/migrations/202610100001_personal_settings.sql) 전체를 실행한다. 앱의 공개 키로 DB를 설치할 수는 없다. 테이블은 로그인한 사용자 자신의 설정만 읽도록 제한되고, 쓰기는 로그인 UID를 서버에서 확인하는 RPC만 허용한다.
3. Authentication → URL Configuration → Redirect URLs에 **`http://127.0.0.1:47821/**`**를 추가한다. 앱은 이 포트에 로그인 콜백을 잠시 받으며, 일회용 state가 붙은 주소로 돌아온다. 포트가 사용 중이면 해당 프로그램을 닫고 다시 로그인한다. Site URL은 실제 운영 사이트나 로그인 안내 페이지로 설정한다.
4. Authentication → Providers에서 Google·Kakao를 켠다. 각 회사의 앱에 Supabase 콘솔에 표시된 콜백 주소(`https://<프로젝트>.supabase.co/auth/v1/callback`)를 등록하고 Client ID·Secret을 **Supabase 콘솔에만** 저장한다. [Google 등록 안내](https://supabase.com/docs/guides/auth/social-login/auth-google), [Kakao 등록 안내](https://supabase.com/docs/guides/auth/social-login/auth-kakao)를 따른다. Kakao 이메일 제공 동의·권한도 확인한다.
5. 이메일 계정의 가입 확인을 설정한다. 사용자에게 가입·확인 메일을 제공하려면 [운영용 SMTP](https://supabase.com/docs/guides/auth/auth-smtp)를 설정한다. 기본 메일 발송 서비스에는 수신자와 발송량 제한이 있으므로 공개 서비스의 메일 서버로 사용하지 않는다. 확인 후 앱의 이메일·비밀번호 로그인으로 들어온다.
6. SSO가 필요하면 Pro 이상에서 SAML 공급자와 회사·학교 도메인을 [등록](https://supabase.com/docs/guides/auth/enterprise-sso/auth-sso-saml)한다. 앱에서는 해당 도메인을 입력한다. 제공하는 SSO 범위는 Supabase에 등록된 SAML 공급자다.

## 설치 앱에 연결 정보 넣기

운영자용 수동 설정은 앱 로그인 창의 **서버 연결 설정**에서 URL과 공개 키를 넣으면 된다. 연결된 계정이 있으면 로그아웃 후 서버를 변경한다. 공개 연결 정보는 앱 데이터 폴더 `account-app.json`에 저장된다.

일반 사용자에게 별도 입력 없이 배포하려면 GitHub 저장소 Settings → Secrets and variables → Actions → **Variables**에 다음 두 값을 등록하고 새 버전을 빌드한다.

- `WRITER_SUPABASE_URL`: 프로젝트 URL
- `WRITER_SUPABASE_PUBLISHABLE_KEY`: Publishable key 또는 anon 공개 키

CI와 release가 이 공개 값을 Rust 빌드에 넣는다. 값이 없으면 설치 앱에 서버 연결 설정 화면이 나온다. 로그인 제공자의 Secret과 관리자 키는 이 과정에 사용하지 않는다.

## 계정과 연결 실패 처리

설정은 **프로젝트 URL + Supabase 사용자 UID**별로 분리한다. 같은 이메일이어도 UID가 다르면 다른 설정이다. Supabase의 계정 연결 정책에 따라 Google·Kakao·이메일 로그인은 같은 UID를 사용할 수 있지만, 특히 SAML SSO 계정이 자동으로 다른 계정과 합쳐진다고 가정하지 않는다.

로그아웃하면 로그인 전의 기기 설정으로 돌아간다. 인터넷이 끊겼을 때는 계정별 캐시를 적용하고 변경을 보관한다. 다시 연결하면 서버의 최신 설정을 읽고 변경한 항목만 합친다. 같은 항목을 동시에 바꾸면 나중에 성공한 저장값이 적용된다. 계정이 바뀐 뒤 이전 계정의 응답이나 대기 중인 저장이 다른 계정으로 적용되지 않도록 화면과 네이티브 명령 모두 확인한다.

로그인 유지용 refresh token은 Windows 자격 증명 관리자에 저장한다. access token은 네이티브 메모리에만 있으며 화면에는 계정 이름·이메일·UID만 전달한다. OAuth·SSO는 시스템 브라우저와 PKCE, 일회용 state를 사용한다.

## 검증 범위

자동 검사는 설정의 계정 분리·오프라인 복구·늦은 응답·동시 변경·리비전 충돌, 실제 PostgreSQL의 RLS·RPC 권한·입력 검증, 네이티브 인증 HTTP 요청·응답 검증을 포함한다. 브라우저 개발 미리보기는 예시 계정이다.

실제 Google·Kakao·이메일·SAML 공급자 로그인은 프로젝트 생성, SQL 설치, 공급자 등록, 연결 정보 입력 후 설치형 앱에서 각각 확인해야 한다. 프로젝트 연결 정보가 없는 빌드가 실제 Supabase 서버에 연결된 상태라는 뜻은 아니다.
