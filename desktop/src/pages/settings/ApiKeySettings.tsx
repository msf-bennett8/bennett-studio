import { ApiKeyPanel } from '../../components/sharing/ApiKeyPanel';

export function ApiKeySettings() {
  return (
    <div className="card rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
      <ApiKeyPanel />
    </div>
  );
}
