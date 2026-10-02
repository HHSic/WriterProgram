// 이 작품 크기 in 작품 설정: how much room the writing, records and trash
// take, a suggestion to tidy old automatic records when they have grown far
// past the writing, and a gentle warning when this computer's disk is
// nearly full (crates/core/src/project/size.rs).

import { useEffect, useState } from 'react';
import { api } from '../../api';
import type { ProjectSizes } from '../../api/types';
import { sizeText } from '../../lib/format';
import { openDialog, showToast, toastError, useApp } from '../../store';

/** Under this much free room on the disk, say so. */
const LOW_DISK = 200 * 1024 * 1024;

export function ProjectSizeField() {
  const root = useApp((s) => s.overview!.root);
  const [sizes, setSizes] = useState<ProjectSizes | null>(null);

  useEffect(() => {
    let live = true;
    api.projectSize(root).then(
      (s) => live && setSizes(s),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [root]);

  if (!sizes) return null;

  const parts = [`원고 ${sizeText(sizes.writing)}`];
  if (sizes.records) parts.push(`기록 ${sizeText(sizes.records)}`);
  if (sizes.trash) parts.push(`휴지통 ${sizeText(sizes.trash)}`);
  if (sizes.journal) parts.push(`작업 일지 ${sizeText(sizes.journal)}`);

  const tidy = () =>
    openDialog({
      kind: 'confirm',
      title: '오래된 자동 기록 정리',
      message: `2주보다 오래된 자동 기록을 지워 약 ${sizeText(sizes.tidyFrees)}를 비웁니다. 회차마다 가장 최근 자동 기록 하나와, 지금 원고 보관으로 직접 남긴 기록은 그대로 둡니다.`,
      confirm: '정리하기',
      onConfirm: async () => {
        try {
          const freed = await api.recordsTidy(root);
          showToast({ text: `오래된 자동 기록을 정리해 ${sizeText(freed)}를 비웠습니다.` });
        } catch (e) {
          toastError('기록을 정리하지 못함', e);
        }
        openDialog({ kind: 'project', tab: 'basic' });
      },
    });

  return (
    <div className="field">
      <span>이 작품 크기: {parts.join(' · ')}</span>
      {sizes.suggestTidy && (
        <div className="row wrap">
          <small className="hint grow">
            기록이 원고보다 훨씬 큽니다. 오래된 자동 기록을 정리하면 약 {sizeText(sizes.tidyFrees)}가 줄어듭니다.
          </small>
          <button type="button" className="btn small" onClick={tidy}>
            오래된 자동 기록 정리
          </button>
        </div>
      )}
      {sizes.diskFree !== null && sizes.diskFree < LOW_DISK && (
        <small className="warn-text">
          이 컴퓨터에 남은 공간이 {sizeText(sizes.diskFree)}뿐입니다. 공간이 모자라면 원고를 저장하지 못할 수 있으니, 필요 없는 파일을 지워 주세요.
        </small>
      )}
    </div>
  );
}
