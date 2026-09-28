import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import type { RecentItem } from '../api/types';
import { Icon } from '../components/Icon';
import { openMenu } from '../components/Menu';
import { num, shortPath, timeLabel } from '../lib/format';
import { KIND_LABEL } from '../lib/labels';
import { openDialog, openProject, toastError } from '../store';

export function StartScreen() {
  const [items, setItems] = useState<RecentItem[] | null>(null);

  const load = useCallback(() => {
    api.recentList().then(setItems, (e) => {
      setItems([]);
      toastError('최근 작품 목록을 읽지 못함', e);
    });
  }, []);

  useEffect(load, [load]);

  const openFolder = async () => {
    const path = await api.pickFolder('작품 폴더 열기');
    if (path) await openProject(path);
  };

  const relocate = async (item: RecentItem) => {
    const path = await api.pickFolder(`'${item.title}' 폴더 찾기`);
    if (!path) return;
    if (await openProject(path)) await api.recentRemove(item.path).catch(() => {});
  };

  const forget = async (item: RecentItem) => {
    await api.recentRemove(item.path).catch((e) => toastError('목록에서 빼지 못함', e));
    load();
  };

  const newProject = () => openDialog({ kind: 'newProject' });

  return (
    <div className="start">
      <header className="start-head">
        <div className="logo" aria-hidden="true">
          글
        </div>
        <div className="grow">
          <h1>WriterProgram</h1>
          <p>한 편을 끝까지 쓰는 곳</p>
        </div>
        <div className="row">
          <button type="button" className="btn ghost" onClick={() => openDialog({ kind: 'driveImport' })}>
            드라이브에서 가져오기
          </button>
          <button type="button" className="btn ghost" onClick={() => openDialog({ kind: 'drives' })}>
            기기 간 맞추기
          </button>
        </div>
      </header>

      {items !== null && items.length === 0 && (
        <section className="start-empty">
          <button type="button" className="start-big primary" onClick={newProject}>
            <Icon name="plus" size={22} />
            <span>
              <strong>새 작품</strong>
              <small>웹소설 연재나 출판 장편을 새로 시작합니다</small>
            </span>
          </button>
          <button type="button" className="start-big" onClick={openFolder}>
            <Icon name="folder" size={22} />
            <span>
              <strong>폴더에서 열기</strong>
              <small>이 앱으로 만든 작품 폴더를 엽니다</small>
            </span>
          </button>
        </section>
      )}

      {items !== null && items.length > 0 && (
        <section className="start-list">
          <div className="start-list-head">
            <h2>최근 작품</h2>
            <div className="row">
              <button type="button" className="btn" onClick={openFolder}>
                <Icon name="folder" />
                폴더에서 열기
              </button>
              <button type="button" className="btn primary" onClick={newProject}>
                <Icon name="plus" />새 작품
              </button>
            </div>
          </div>
          <div className="project-grid">
            {items.map((item) => (
              <article key={item.path} className={`project-card${item.exists ? '' : ' missing'}`}>
                <button
                  type="button"
                  className="project-open"
                  onClick={() => (item.exists ? void openProject(item.path) : void relocate(item))}
                >
                  <span className="kind-chip">{KIND_LABEL[item.kind]}</span>
                  <strong className="project-title">{item.title}</strong>
                  <span className="meta">
                    {item.exists ? `공백 포함 ${num(item.chars)}자 · ${timeLabel(item.openedAt)}` : '폴더를 찾을 수 없음'}
                  </span>
                  <span className="path" title={item.path}>
                    {shortPath(item.path)}
                  </span>
                </button>
                <button
                  type="button"
                  className="icon-btn card-menu"
                  aria-label={`${item.title} 메뉴`}
                  onClick={(e) =>
                    openMenu(e, [
                      item.exists
                        ? { label: '열기', onSelect: () => void openProject(item.path) }
                        : { label: '위치 다시 지정', onSelect: () => void relocate(item) },
                      {
                        label: '폴더 위치 열기',
                        disabled: !item.exists,
                        onSelect: () => void api.reveal(item.path).catch((err) => toastError('폴더를 열지 못함', err)),
                      },
                      { separator: true },
                      { label: '목록에서 빼기 (파일은 그대로)', onSelect: () => void forget(item) },
                    ])
                  }
                >
                  <Icon name="more" />
                </button>
              </article>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
