// AI 연결 (내 API 키): this device's AI settings. Off by default; turning it
// on asks the writer to agree first. The key goes to the system's
// credential store and never comes back to the screen.

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { AiCompany, AiModels, AiProvider, AiSettings } from '../api/types';
import { Modal } from '../components/Modal';
import { errorText } from '../lib/format';
import { closeDialog, removeAiKey, setAiKey, toastError, updateAi, useApp } from '../store';

/** Where each company hands out keys, in words (no links to click). */
const KEY_PLACE: Record<AiProvider, string> = {
  anthropic: 'Anthropic Console(console.anthropic.com)의 API Keys',
  openai: 'OpenAI Platform(platform.openai.com)의 API keys',
  gemini: 'Google AI Studio(aistudio.google.com)의 API 키 (결제를 연결한 유료 등급)',
};

export function AiDialog() {
  const ai = useApp((s) => s.ai);
  const [asking, setAsking] = useState(false);

  useEffect(() => {
    api.aiSettings().then(
      (settings) => useApp.setState({ ai: settings }),
      (e) => toastError('AI 설정을 읽지 못함', e),
    );
  }, []);

  return (
    <Modal
      title="AI 연결 (내 API 키)"
      onClose={closeDialog}
      width={600}
      footer={
        <button type="button" className="btn primary" onClick={closeDialog}>
          닫기
        </button>
      }
    >
      {!ai ? (
        <p className="hint">불러오는 중…</p>
      ) : (
        <div className="form ai-settings">
          <p className="hint">
            내가 가입한 AI 회사의 API 키로 회차 요약과 설정 모순 점검을 씁니다. 요금은 AI 회사에 직접 내고, 이 앱은 중간에서 아무것도
            받거나 남기지 않습니다. 이 기기에만 적용됩니다.
          </p>
          <div className="field">
            <span className="field-label">AI 점검</span>
            {ai.enabled ? (
              <label className="check">
                <input type="checkbox" checked onChange={() => void updateAi({ enabled: false })} />
                켜짐 · 점검할 때마다 보낼 내용을 먼저 보여 드립니다
              </label>
            ) : asking ? (
              <Consent
                onAgree={async () => {
                  if (await updateAi({ enabled: true })) setAsking(false);
                }}
                onCancel={() => setAsking(false)}
              />
            ) : (
              <label className="check">
                <input type="checkbox" checked={false} onChange={() => setAsking(true)} />
                꺼짐 · 켜면 원고 일부를 내가 고른 AI 회사로 보낼 수 있습니다
              </label>
            )}
          </div>
          <CompanyField ai={ai} />
        </div>
      )}
    </Modal>
  );
}

function Consent({ onAgree, onCancel }: { onAgree: () => Promise<void>; onCancel: () => void }) {
  return (
    <div className="ai-consent" role="group" aria-label="AI 점검을 켜기 전에">
      <strong>켜기 전에 읽어 주세요</strong>
      <ul>
        <li>점검할 때 원고 일부(고른 회차와, 설정 모순 점검이면 그 회차에 나오는 설정 카드)가 내가 고른 AI 회사로 갑니다. 보내기 전에 보낼 내용을 그대로 보여 드립니다.</li>
        <li>인물·장소·용어 이름은 ‘인물A’처럼 가려서 보내고, 답을 받은 뒤 되돌립니다.</li>
        <li>학습에 쓰지 않는다는 약관이 있는 유료 API 키를 쓰세요. 무료 등급 키는 보낸 글이 AI 학습에 쓰일 수 있습니다.</li>
        <li>본문을 써 주거나 이어 쓰는 기능은 없습니다. AI는 읽고, 요약하고, 점검만 합니다. 결과가 원고를 저절로 바꾸지 않습니다.</li>
        <li>보낸 글과 받은 답은 이 기기에 저장하지 않습니다. 화면을 닫으면 사라집니다.</li>
      </ul>
      <div className="row">
        <button type="button" className="btn primary small" onClick={() => void onAgree()}>
          동의하고 켜기
        </button>
        <button type="button" className="btn small" onClick={onCancel}>
          취소
        </button>
      </div>
    </div>
  );
}

function CompanyField({ ai }: { ai: AiSettings }) {
  const company = ai.companies.find((c) => c.provider === ai.provider) ?? ai.companies[0];
  return (
    <>
      <fieldset className="field">
        <legend className="field-label">AI 회사</legend>
        <div className="segmented">
          {ai.companies.map((c) => (
            <label key={c.provider} className={c.provider === ai.provider ? 'on' : ''}>
              <input type="radio" checked={c.provider === ai.provider} onChange={() => void updateAi({ provider: c.provider })} />
              {c.label}
            </label>
          ))}
        </div>
      </fieldset>
      <KeyField key={`key-${company.provider}`} company={company} />
      <ModelFields key={`models-${company.provider}-${company.models.summary}-${company.models.check}`} company={company} />
      <CheckConnection key={`check-${company.provider}`} company={company} />
    </>
  );
}

function KeyField({ company }: { company: AiCompany }) {
  const [editing, setEditing] = useState(!company.hasKey);
  const [key, setKey] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const save = async () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await setAiKey(company.provider, key);
      setKey('');
      setEditing(false);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="field">
      <span className="field-label">{company.label} API 키</span>
      {company.hasKey && !editing ? (
        <div className="row wrap">
          <span className="grow">키 저장됨 · 이 기기의 자격 증명 저장소에 있습니다</span>
          <button type="button" className="btn small" onClick={() => setEditing(true)}>
            키 바꾸기
          </button>
          <button type="button" className="btn small" onClick={() => void removeAiKey(company.provider)}>
            키 지우기
          </button>
        </div>
      ) : (
        <form
          className="row"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <input
            className="grow"
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="API 키를 붙여 넣으세요"
            aria-label={`${company.label} API 키`}
          />
          <button type="submit" className="btn small primary" disabled={busy || !key.trim()}>
            저장
          </button>
          {company.hasKey && (
            <button type="button" className="btn small" onClick={() => setEditing(false)}>
              취소
            </button>
          )}
        </form>
      )}
      {error && <small className="warn-text">{error}</small>}
      <small className="hint">키는 {KEY_PLACE[company.provider]}에서 만듭니다. 화면에 다시 보이지 않고, 파일에도 남지 않습니다.</small>
    </div>
  );
}

function ModelFields({ company }: { company: AiCompany }) {
  const [models, setModels] = useState<AiModels>(company.models);
  const changed = models.summary !== company.models.summary || models.check !== company.models.check;
  const isDefault = company.models.summary === company.defaults.summary && company.models.check === company.defaults.check;

  const save = (next: AiModels) => void updateAi({ models: [company.provider, next] });

  return (
    <div className="field">
      <span className="field-label">모델</span>
      <div className="ai-models">
        <label>
          <span>회차 요약</span>
          <input
            value={models.summary}
            spellCheck={false}
            placeholder={company.defaults.summary}
            onChange={(e) => setModels({ ...models, summary: e.target.value })}
          />
        </label>
        <label>
          <span>설정 모순 점검</span>
          <input
            value={models.check}
            spellCheck={false}
            placeholder={company.defaults.check}
            onChange={(e) => setModels({ ...models, check: e.target.value })}
          />
        </label>
      </div>
      <div className="row wrap">
        <small className="hint grow">
          요약은 값싼 모델, 점검은 중간 모델이 기본입니다. 모델 이름은 자주 바뀌니 AI 회사 안내에 맞게 고쳐 써도 됩니다.
        </small>
        {changed && (
          <button type="button" className="btn small primary" onClick={() => save(models)}>
            모델 저장
          </button>
        )}
        {!changed && !isDefault && (
          <button type="button" className="btn small" onClick={() => save(company.defaults)}>
            기본값으로
          </button>
        )}
      </div>
    </div>
  );
}

function CheckConnection({ company }: { company: AiCompany }) {
  const [result, setResult] = useState<{ text: string; ok: boolean } | null>(null);
  const [busy, setBusy] = useState(false);

  const check = async () => {
    setBusy(true);
    setResult(null);
    try {
      setResult({ text: await api.aiConnectionCheck(), ok: true });
    } catch (e) {
      setResult({ text: errorText(e), ok: false });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="field">
      <div className="row wrap">
        <button type="button" className="btn small" disabled={busy || !company.hasKey} onClick={() => void check()}>
          {busy ? '확인하는 중…' : '연결 확인'}
        </button>
        <small className="hint grow">키와 모델 이름만 확인합니다. 원고는 보내지 않고 요금도 들지 않습니다.</small>
      </div>
      {result && <small className={result.ok ? 'hint ai-ok' : 'warn-text'}>{result.text}</small>}
    </div>
  );
}
