// 기기 간 맞추기 with the writer's own drive (crates/sync): connecting Google
// Drive, OneDrive or Dropbox, keeping a project in step with one, and
// bringing a project from a drive to this device.

import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import type { DriveInfo, DriveProject, DriveProvider, DriveStatus } from '../api/types';
import { Modal } from '../components/Modal';
import { placeNote, usePlaceOf } from '../components/PlacePicker';
import { errorText, timeLabel } from '../lib/format';
import { closeDialog, enterProject, linkProject, openDialog, syncNow, toastError, unlinkProject, useApp } from '../store';

/** The drives and their connections, read once and on demand. */
function useDrives(): [DriveStatus | null, () => void] {
  const [status, setStatus] = useState<DriveStatus | null>(null);
  const load = useCallback(() => {
    api.driveStatus().then(setStatus, (e) => toastError('드라이브 연결 상태를 읽지 못함', e));
  }, []);
  useEffect(load, [load]);
  return [status, load];
}

export function DrivesDialog() {
  const [status, reload] = useDrives();
  const [connecting, setConnecting] = useState<DriveProvider | null>(null);
  const [error, setError] = useState<{ provider: DriveProvider; text: string } | null>(null);

  const connect = async (d: DriveInfo) => {
    setError(null);
    setConnecting(d.provider);
    try {
      await api.driveConnect(d.provider);
    } catch (e) {
      setError({ provider: d.provider, text: errorText(e) });
    } finally {
      setConnecting(null);
      reload();
    }
  };

  const disconnect = (d: DriveInfo) =>
    openDialog({
      kind: 'confirm',
      title: `${d.label} 연결 끊기`,
      message: `이 PC에서 ${d.label} 연결을 지웁니다. 이 드라이브와 맞추던 작품은 더 맞추지 않습니다. 드라이브와 이 PC의 파일은 그대로 남습니다.`,
      confirm: '연결 끊기',
      danger: true,
      onConfirm: async () => {
        try {
          await api.driveDisconnect(d.provider);
          if (useApp.getState().link?.provider === d.provider) useApp.setState({ link: null });
        } catch (e) {
          toastError('연결을 끊지 못함', e);
        }
        openDialog({ kind: 'drives' });
      },
    });

  const anyRegistered = status?.drives.some((d) => d.registered);

  return (
    <Modal title="기기 간 맞추기" onClose={closeDialog} width={580}>
      <div className="form">
        <p className="dialog-text">
          휴대폰이나 다른 컴퓨터에서 이어 쓰려면 드라이브를 연결하세요. 드라이브에 이 앱 전용 폴더를 만들어 작품을 맞춥니다.
          컴퓨터끼리는 작품을 OneDrive 같은 폴더에 두기만 해도 됩니다.
        </p>
        <ul className="drive-list">
          {status?.drives.map((d) => (
            <li key={d.provider} className="drive-item">
              <span className={`place-mark service-${d.provider === 'google' ? 'googledrive' : d.provider}`} aria-hidden="true" />
              <div className="grow">
                <strong>{d.label}</strong>
                <span className="meta">
                  {d.account
                    ? `${d.account.name || d.account.email} · ${d.account.email}`
                    : connecting === d.provider
                      ? '브라우저에서 로그인하고 허용을 누르세요'
                      : d.registered
                        ? '연결 안 됨'
                        : '준비 중 (앱 등록 전)'}
                </span>
                {error?.provider === d.provider && <span className="warn-text">{error.text}</span>}
              </div>
              {d.account ? (
                <button type="button" className="btn small" onClick={() => disconnect(d)}>
                  연결 끊기
                </button>
              ) : connecting === d.provider ? (
                <button type="button" className="btn small" onClick={() => void api.driveCancel()}>
                  그만두기
                </button>
              ) : (
                <button
                  type="button"
                  className="btn small primary"
                  disabled={!d.registered || connecting !== null}
                  onClick={() => void connect(d)}
                >
                  연결
                </button>
              )}
            </li>
          ))}
        </ul>
        <ul className="drive-notes">
          <li>비밀번호는 이 앱이 모릅니다. 로그인 창은 각 드라이브의 것입니다.</li>
          <li>연결 정보는 이 PC의 자격 증명 관리자에만 보관합니다.</li>
          <li>드라이브의 다른 파일은 보지 않습니다. 이 앱이 만든 폴더만 씁니다.</li>
        </ul>
        {status && !anyRegistered && (
          <small className="hint">
            드라이브마다 앱 등록을 마치면 연결할 수 있습니다. 등록 정보는 {status.appsFile}에 넣습니다.
          </small>
        )}
      </div>
    </Modal>
  );
}

/** In 작품 설정: whether this project follows a drive, and the controls for it. */
export function ProjectDriveField() {
  const ov = useApp((s) => s.overview)!;
  const link = useApp((s) => s.link);
  const syncing = useApp((s) => s.syncing);
  const where = usePlaceOf(ov.root);
  const [status] = useDrives();
  const connected = status?.drives.filter((d) => d.account) ?? [];
  const [pick, setPick] = useState<DriveProvider | ''>('');
  const label = (p: DriveProvider) => status?.drives.find((d) => d.provider === p)?.label ?? p;

  useEffect(() => {
    if (!pick && connected[0]) setPick(connected[0].provider);
  }, [connected, pick]);

  return (
    <div className="field">
      <span className="field-label">드라이브와 맞추기</span>
      {link ? (
        <>
          <div className="row wrap">
            <span className="grow">
              {label(link.provider)}의 '{link.folder}' 폴더와 맞춤
              {link.syncedAt && <span className="meta"> · 마지막 {timeLabel(link.syncedAt)}</span>}
            </span>
            <button type="button" className="btn small" disabled={syncing} onClick={() => void syncNow()}>
              {syncing ? '맞추는 중…' : '지금 맞추기'}
            </button>
            <button type="button" className="btn small" onClick={() => void unlinkProject()}>
              그만 맞추기
            </button>
          </div>
          {link.error && <small className="warn-text">마지막으로 맞추지 못함 · {link.error}</small>}
        </>
      ) : connected.length ? (
        <div className="row">
          <select value={pick} onChange={(e) => setPick(e.target.value as DriveProvider)} aria-label="맞출 드라이브">
            {connected.map((d) => (
              <option key={d.provider} value={d.provider}>
                {d.label} · {d.account?.email}
              </option>
            ))}
          </select>
          <button type="button" className="btn" disabled={!pick} onClick={() => pick && void linkProject(pick)}>
            맞추기 시작
          </button>
        </div>
      ) : (
        <div className="row">
          <span className="grow meta">연결된 드라이브가 없습니다.</span>
          <button type="button" className="btn" onClick={() => openDialog({ kind: 'drives' })}>
            기기 간 맞추기…
          </button>
        </div>
      )}
      {where && !link && <small className="hint">{placeNote(where)} 휴대폰에서도 쓰려면 드라이브와 맞추세요.</small>}
    </div>
  );
}

/** Bringing a project from a connected drive to this device. */
export function DriveImportDialog() {
  const [status] = useDrives();
  const [found, setFound] = useState<{ provider: DriveProvider; label: string; projects: DriveProject[] | null; error?: string }[]>([]);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    if (!status) return;
    const connected = status.drives.filter((d) => d.account);
    setFound(connected.map((d) => ({ provider: d.provider, label: d.label, projects: null })));
    for (const d of connected) {
      api.driveProjects(d.provider).then(
        (projects) => setFound((prev) => prev.map((f) => (f.provider === d.provider ? { ...f, projects } : f))),
        (e) => setFound((prev) => prev.map((f) => (f.provider === d.provider ? { ...f, projects: [], error: errorText(e) } : f))),
      );
    }
  }, [status]);

  const bring = async (provider: DriveProvider, p: DriveProject) => {
    setBusy(p.folder);
    try {
      const dest = await api.defaultLocation();
      const ov = await api.driveFetch(provider, p.folder, dest);
      closeDialog();
      enterProject(ov);
    } catch (e) {
      toastError('작품을 가져오지 못함', e);
      setBusy(null);
    }
  };

  const none = status && !status.drives.some((d) => d.account);

  return (
    <Modal title="드라이브에서 가져오기" onClose={closeDialog} width={560}>
      {none && (
        <div className="form">
          <p className="dialog-text">연결된 드라이브가 없습니다. 먼저 드라이브를 연결하세요.</p>
          <button type="button" className="btn primary" onClick={() => openDialog({ kind: 'drives' })}>
            기기 간 맞추기…
          </button>
        </div>
      )}
      {found.map((f) => (
        <section key={f.provider} className="drive-import">
          <h3>{f.label}</h3>
          {f.projects === null && <p className="meta">불러오는 중</p>}
          {f.error && <p className="warn-text">{f.error}</p>}
          {f.projects?.length === 0 && !f.error && <p className="empty-note">이 드라이브에 맞춘 작품이 없습니다.</p>}
          <ul className="copy-list">
            {f.projects?.map((p) => (
              <li key={p.folder} className="copy-item">
                <div className="grow">
                  <strong>{p.title}</strong>
                  <span className="meta">{p.folder}</span>
                </div>
                <button type="button" className="btn small primary" disabled={busy !== null} onClick={() => void bring(f.provider, p)}>
                  {busy === p.folder ? '가져오는 중…' : '이 기기로 가져오기'}
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </Modal>
  );
}
