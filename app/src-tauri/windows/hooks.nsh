; Installer hooks (tauri.mcp.conf.json): offer to put the install folder on
; the user's PATH so a terminal finds the `writer` command (docs/cli.md), and
; take it off again when WriterProgram is removed.
;
; writer.exe edits PATH itself (crates/mcp/src/path.rs) because NSIS strings
; are cut at 1024 characters and a long PATH would be cut with them.
; Updates (/UPDATE) keep what was chosen at the first install, and installs
; nobody watches (silent, passive) leave PATH alone.

!macro NSIS_HOOK_POSTINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_YESNO|MB_ICONQUESTION "터미널에서 writer 명령으로 작품을 읽고 쓸 수 있게 할까요?$\r$\n$\r$\n설치 폴더를 사용자 PATH에 등록합니다. 나중에 작품 설정 → AI 도우미 연결이나 writer path remove로 뺄 수 있습니다." IDNO writer_path_skip
    nsExec::Exec '"$INSTDIR\writer.exe" path add'
    Pop $0
    writer_path_skip:
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    nsExec::Exec '"$INSTDIR\writer.exe" path remove'
    Pop $0
  ${EndIf}
!macroend
