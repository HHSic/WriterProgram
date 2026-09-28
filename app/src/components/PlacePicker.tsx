// 저장 위치 고르기: folders OneDrive, Google Drive, Dropbox or iCloud keep in
// step with other devices, this computer only, or a folder picked by hand.
// Used for new projects and for moving one (crates/core/src/places.rs).

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { Place } from '../api/types';
import { shortPath } from '../lib/format';

/** One line about what a place means for the writer. */
export function placeNote(place: Place | null): string {
  if (!place || place.service === 'local') return '이 PC에만 저장됩니다. 다른 기기에서는 열 수 없습니다.';
  return `다른 기기와 맞춰집니다 (${place.label}). 그 기기에서도 이어 쓸 수 있습니다.`;
}

/** The sync folder `path` is in: a place, null for this computer only, undefined while asking. */
export function usePlaceOf(path: string): Place | null | undefined {
  const [place, setPlace] = useState<Place | null | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    setPlace(undefined);
    api.storageOf(path).then(
      (p) => alive && setPlace(p),
      () => alive && setPlace(null),
    );
    return () => {
      alive = false;
    };
  }, [path]);
  return place;
}

function samePath(a: string, b: string): boolean {
  const norm = (p: string) => p.replace(/[\\/]+$/, '').toLowerCase();
  return norm(a) === norm(b);
}

/**
 * Picks the folder a project goes in. `value` is that folder; places come
 * from `api.storagePlaces()`.
 */
export function PlacePicker({
  places,
  value,
  onChange,
  here,
}: {
  places: Place[] | null;
  value: string;
  onChange: (path: string) => void;
  /** The folder the project is in now (when moving), marked 지금 여기. */
  here?: string;
}) {
  const chosen = places?.find((p) => samePath(p.suggested, value)) ?? null;
  const [custom, setCustom] = useState<Place | null | undefined>(undefined);

  // A folder picked by hand: which sync program keeps it, if any.
  useEffect(() => {
    if (chosen || !value) {
      setCustom(undefined);
      return;
    }
    let alive = true;
    api.storageOf(value).then(
      (place) => alive && setCustom(place),
      () => alive && setCustom(null),
    );
    return () => {
      alive = false;
    };
  }, [chosen, value]);

  const pick = async () => {
    const path = await api.pickFolder('작품을 둘 곳 고르기', value || undefined);
    if (path) onChange(path);
  };

  return (
    <div className="places" role="radiogroup" aria-label="저장 위치">
      {places?.map((place) => {
        const on = samePath(place.suggested, value);
        const isHere = here !== undefined && samePath(place.suggested, here);
        return (
          <label key={place.suggested} className={`place${on ? ' on' : ''}`}>
            <input type="radio" name="place" checked={on} onChange={() => onChange(place.suggested)} />
            <span className={`place-mark service-${place.service}`} aria-hidden="true" />
            <span className="place-text">
              <strong>
                {place.service === 'local' ? '이 PC에만' : place.label}
                {isHere && <span className="place-here">지금 여기</span>}
              </strong>
              <small>{place.service === 'local' ? '다른 기기와 맞추지 않음' : '다른 기기와 맞춤'}</small>
              <small className="place-path" title={place.suggested}>
                {shortPath(place.suggested)}
              </small>
            </span>
          </label>
        );
      })}
      {!chosen && value && (
        <label className="place on">
          <input type="radio" name="place" checked readOnly />
          <span className={`place-mark service-${custom?.service ?? 'local'}`} aria-hidden="true" />
          <span className="place-text">
            <strong>직접 고른 위치</strong>
            <small>{custom === undefined ? '' : custom ? `다른 기기와 맞춤 · ${custom.label}` : '다른 기기와 맞추지 않음'}</small>
            <small className="place-path" title={value}>
              {shortPath(value)}
            </small>
          </span>
        </label>
      )}
      {api.isDesktop && (
        <button type="button" className="btn small place-pick" onClick={() => void pick()}>
          다른 폴더 고르기…
        </button>
      )}
    </div>
  );
}
