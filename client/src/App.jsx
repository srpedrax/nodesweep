import { useMemo, useState } from 'react';

const STORAGE_KEY = 'nodesweep:lastRootPath';

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toLocaleString('pt-BR', { maximumFractionDigits: index > 2 ? 2 : 1 })} ${units[index]}`;
}

function projectAge(dateValue) {
  const days = Math.max(0, Math.floor((Date.now() - new Date(dateValue).getTime()) / 86400000));
  if (days === 0) return { text: 'Modificado hoje', badge: 'Ativo', tone: 'active' };
  if (days === 1) return { text: 'Modificado ontem', badge: 'Ativo', tone: 'active' };
  if (days < 30) return { text: `Modificado há ${days} dias`, badge: 'Recente', tone: 'recent' };
  const months = Math.floor(days / 30);
  if (days < 365) return { text: `Modificado há ${months} ${months === 1 ? 'mês' : 'meses'}`, badge: 'Antigo', tone: 'old' };
  const years = Math.floor(days / 365);
  return { text: `Modificado há ${years} ${years === 1 ? 'ano' : 'anos'}`, badge: 'Antigo', tone: 'old' };
}

async function apiRequest(url, options) {
  const response = await fetch(url, options);
  let body;
  try { body = await response.json(); } catch { body = {}; }
  if (!response.ok) throw new Error(body.error || 'Não foi possível concluir a operação.');
  return body;
}

function App() {
  const [rootPath, setRootPath] = useState(() => localStorage.getItem(STORAGE_KEY) || '');
  const [projects, setProjects] = useState([]);
  const [selected, setSelected] = useState(new Set());
  const [status, setStatus] = useState('idle');
  const [error, setError] = useState('');
  const [hasScanned, setHasScanned] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [freedBytes, setFreedBytes] = useState(null);

  const totalBytes = useMemo(() => projects.reduce((sum, item) => sum + item.sizeBytes, 0), [projects]);
  const selectedProjects = useMemo(() => projects.filter((item) => selected.has(item.nodeModulesPath)), [projects, selected]);
  const selectedBytes = selectedProjects.reduce((sum, item) => sum + item.sizeBytes, 0);
  const allSelected = projects.length > 0 && selected.size === projects.length;

  async function scan(event) {
    event.preventDefault();
    const value = rootPath.trim();
    if (!value) return setError('Informe a pasta onde ficam seus projetos.');
    setStatus('scanning'); setError(''); setFreedBytes(null); setHasScanned(true);
    try {
      const data = await apiRequest('/api/scan', {
        method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ rootPath: value })
      });
      localStorage.setItem(STORAGE_KEY, value);
      setProjects(data.projects); setSelected(new Set());
    } catch (requestError) {
      setProjects([]); setSelected(new Set()); setError(requestError.message);
    } finally { setStatus('idle'); }
  }

  function toggle(path) {
    setSelected((current) => {
      const next = new Set(current);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }

  function toggleAll() {
    setSelected(allSelected ? new Set() : new Set(projects.map((item) => item.nodeModulesPath)));
  }

  async function cleanup() {
    setStatus('cleaning'); setError('');
    try {
      const data = await apiRequest('/api/cleanup', {
        method: 'DELETE', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ paths: selectedProjects.map((item) => item.nodeModulesPath), confirmed: true })
      });
      setProjects((current) => current.filter((item) => !selected.has(item.nodeModulesPath)));
      setSelected(new Set()); setFreedBytes(data.totalFreedBytes); setConfirmOpen(false);
    } catch (requestError) {
      setError(requestError.message); setConfirmOpen(false);
    } finally { setStatus('idle'); }
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <a className="brand" href="#top" aria-label="NodeSweep início"><span className="brand-mark">N</span>Node<span>Sweep</span></a>
        <span className="version">v0.1</span>
      </header>

      <main id="top">
        <section className="hero">
          <p className="eyebrow"><span /> Espaço limpo. Projetos intactos.</p>
          <h1>Recupere o espaço que suas<br /><em>dependências esqueceram.</em></h1>
          <p className="hero-copy">Encontre e remova pastas <code>node_modules</code> antigas com clareza, controle e segurança.</p>

          <form className="scan-panel" onSubmit={scan}>
            <label htmlFor="rootPath">Pasta para escanear</label>
            <div className="input-row">
              <div className="path-input"><span aria-hidden="true">⌘</span><input id="rootPath" value={rootPath} onChange={(e) => setRootPath(e.target.value)} placeholder="C:\\Users\\...\\Projetos" disabled={status !== 'idle'} /></div>
              <button className="primary" disabled={status !== 'idle'}>{status === 'scanning' ? <><i className="spinner" /> Escaneando</> : <>Escanear <span>→</span></>}</button>
            </div>
            <p className="hint">Cole o caminho da pasta onde ficam seus projetos Node.js.</p>
          </form>
        </section>

        {error && <div className="notice error" role="alert"><strong>Não foi possível concluir</strong><span>{error}</span><button onClick={() => setError('')} aria-label="Fechar">×</button></div>}
        {freedBytes !== null && <div className="notice success" role="status"><strong>Limpeza concluída</strong><span>{formatBytes(freedBytes)} recuperados com segurança.</span><button onClick={() => setFreedBytes(null)} aria-label="Fechar">×</button></div>}

        {hasScanned && status !== 'scanning' && (
          <section className="results">
            <div className="results-head">
              <div><p className="section-label">Resultado da varredura</p><h2>{projects.length} {projects.length === 1 ? 'projeto encontrado' : 'projetos encontrados'}</h2></div>
              <div className="recoverable"><strong>{formatBytes(totalBytes)}</strong><span>recuperáveis</span></div>
            </div>

            {projects.length > 0 ? <>
              <button className="select-all" onClick={toggleAll}><span className={`checkbox ${allSelected ? 'checked' : ''}`}>{allSelected ? '✓' : ''}</span> Selecionar todos</button>
              <div className="project-list">
                {projects.map((project) => {
                  const age = projectAge(project.lastModified);
                  const checked = selected.has(project.nodeModulesPath);
                  return <button key={project.nodeModulesPath} className={`project-card ${checked ? 'selected' : ''}`} onClick={() => toggle(project.nodeModulesPath)}>
                    <span className={`checkbox ${checked ? 'checked' : ''}`}>{checked ? '✓' : ''}</span>
                    <span className="project-main"><strong>{project.name}</strong><small title={project.path}>{project.path}</small><span className="meta">{age.text} <i className={`age ${age.tone}`}>{age.badge}</i></span></span>
                    <strong className="size">{formatBytes(project.sizeBytes)}</strong>
                  </button>;
                })}
              </div>
              <div className="action-bar"><p><span>Selecionados</span><strong>{selected.size} {selected.size === 1 ? 'projeto' : 'projetos'} · {formatBytes(selectedBytes)}</strong></p><button className="danger" disabled={!selected.size} onClick={() => setConfirmOpen(true)}>Limpar selecionados <span>⌫</span></button></div>
            </> : <div className="empty"><span>✓</span><h3>Nada para limpar por aqui</h3><p>Nenhum projeto com <code>node_modules</code> foi encontrado nessa pasta.</p></div>}
          </section>
        )}
      </main>

      <footer><span>NodeSweep</span><p>Somente <code>node_modules</code>. Seus projetos permanecem intactos.</p></footer>

      {confirmOpen && <div className="modal-backdrop" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && setConfirmOpen(false)}>
        <div className="modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
          <div className="modal-icon">!</div><p className="section-label">Confirmação necessária</p><h2 id="confirm-title">Remover dependências selecionadas?</h2>
          <p>Você removerá <strong>{selected.size} {selected.size === 1 ? 'pasta' : 'pastas'} node_modules</strong> e recuperará aproximadamente <strong>{formatBytes(selectedBytes)}</strong>.</p>
          <div className="modal-note">O código-fonte e os arquivos do projeto não serão alterados.</div>
          <div className="modal-actions"><button className="secondary" onClick={() => setConfirmOpen(false)} disabled={status === 'cleaning'}>Cancelar</button><button className="danger" onClick={cleanup} disabled={status === 'cleaning'}>{status === 'cleaning' ? <><i className="spinner" /> Limpando</> : 'Sim, limpar agora'}</button></div>
        </div>
      </div>}
    </div>
  );
}

export default App;
