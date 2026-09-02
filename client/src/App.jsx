import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

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

function App() {
  const [rootPath, setRootPath] = useState(() => localStorage.getItem(STORAGE_KEY) || '');
  const [projects, setProjects] = useState([]);
  const [gradleScan, setGradleScan] = useState({ items: [], projects: [], totalSizeBytes: 0, recoverableSizeBytes: 0, gradleHome: null });
  const [ecosystem, setEcosystem] = useState('node');
  const [selected, setSelected] = useState(new Set());
  const [status, setStatus] = useState('idle');
  const [error, setError] = useState('');
  const [hasScanned, setHasScanned] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [freedBytes, setFreedBytes] = useState(null);

  const visibleItems = ecosystem === 'node' ? projects : gradleScan.items;
  const totalBytes = ecosystem === 'node' ? projects.reduce((sum, item) => sum + item.sizeBytes, 0) : gradleScan.recoverableSizeBytes;
  const selectedProjects = useMemo(() => projects.filter((item) => selected.has(item.id)), [projects, selected]);
  const selectedGradle = gradleScan.items.filter((item) => selected.has(item.id));
  const selectedBytes = (ecosystem === 'node' ? selectedProjects : selectedGradle).reduce((sum, item) => sum + item.sizeBytes, 0);
  const selectableItems = ecosystem === 'node' ? projects : gradleScan.items.filter((item) => item.deletable);
  const allSelected = selectableItems.length > 0 && selectableItems.every((item) => selected.has(item.id));

  async function chooseFolder() {
    setError('');
    try {
      const selectedPath = await open({ directory: true, multiple: false, title: 'Escolha a pasta dos seus projetos' });
      if (selectedPath) setRootPath(selectedPath);
    } catch (dialogError) { setError(String(dialogError)); }
  }

  async function scan(event) {
    event.preventDefault();
    const value = rootPath.trim();
    if (!value) return setError('Informe a pasta onde ficam seus projetos.');
    setStatus('scanning'); setError(''); setFreedBytes(null); setHasScanned(true);
    try {
      const [data, gradleData] = await Promise.all([invoke('scan_projects', { rootPath: value }), invoke('scan_gradle', { rootPath: value })]);
      localStorage.setItem(STORAGE_KEY, value);
      setProjects(data.projects); setGradleScan(gradleData); setSelected(new Set());
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
    setSelected(allSelected ? new Set() : new Set(selectableItems.map((item) => item.id)));
  }

  function applyPreset(preset) {
    const categories = preset === 'conservative' ? ['BuildOutput', 'ProjectCache', 'Daemon'] : preset === 'balanced' ? ['BuildOutput', 'ProjectCache', 'Daemon', 'WrapperDistribution'] : ['BuildOutput', 'ProjectCache', 'Daemon', 'WrapperDistribution', 'GlobalCache'];
    setSelected(new Set(gradleScan.items.filter((item) => item.deletable && categories.includes(item.category)).map((item) => item.id)));
  }

  async function cleanup() {
    setStatus('cleaning'); setError('');
    try {
      const data = ecosystem === 'node'
        ? await invoke('cleanup_projects', { ids: selectedProjects.map((item) => item.id), confirmed: true })
        : await invoke('cleanup_gradle', { ids: selectedGradle.map((item) => item.id), confirmed: true });
      if (ecosystem === 'node') setProjects((current) => current.filter((item) => !selected.has(item.id)));
      else setGradleScan((current) => ({ ...current, items: current.items.filter((item) => !selected.has(item.id)), recoverableSizeBytes: Math.max(0, current.recoverableSizeBytes - data.totalFreedBytes) }));
      setSelected(new Set()); setFreedBytes(data.totalFreedBytes); setConfirmOpen(false);
    } catch (requestError) {
      setError(requestError.message); setConfirmOpen(false);
    } finally { setStatus('idle'); }
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <a className="brand" href="#top" aria-label="NodeSweep início"><img className="app-logo" src="/nodesweep-icon.png" alt="" />Node<span>Sweep</span></a>
        <span className="version">v2.1 alpha</span>
      </header>

      <main id="top">
        <section className="hero">
          <p className="eyebrow"><span /> Espaço limpo. Projetos intactos.</p>
          <h1>Recupere o espaço que suas<br /><em>dependências esqueceram.</em></h1>
          <p className="hero-copy">Encontre e remova pastas <code>node_modules</code> antigas com clareza, controle e segurança.</p>

          <form className="scan-panel" onSubmit={scan}>
            <label htmlFor="rootPath">Pasta para escanear</label>
            <div className="input-row">
              <div className="path-input"><span aria-hidden="true">⌘</span><input id="rootPath" value={rootPath} onChange={(e) => setRootPath(e.target.value)} placeholder="C:\\Users\\...\\Projetos" disabled={status !== 'idle'} /><button type="button" className="browse" onClick={chooseFolder} disabled={status !== 'idle'}>Escolher pasta</button></div>
              <button className="primary" disabled={status !== 'idle'}>{status === 'scanning' ? <><i className="spinner" /> Escaneando</> : <>Escanear <span>→</span></>}</button>
            </div>
            <p className="hint">Cole o caminho da pasta onde ficam seus projetos Node.js.</p>
          </form>
        </section>

        {error && <div className="notice error" role="alert"><strong>Não foi possível concluir</strong><span>{error}</span><button onClick={() => setError('')} aria-label="Fechar">×</button></div>}
        {freedBytes !== null && <div className="notice success" role="status"><strong>Limpeza concluída</strong><span>{formatBytes(freedBytes)} recuperados com segurança.</span><button onClick={() => setFreedBytes(null)} aria-label="Fechar">×</button></div>}

        {hasScanned && status !== 'scanning' && (
          <section className="results">
            <div className="ecosystem-tabs"><button className={ecosystem === 'node' ? 'active' : ''} onClick={() => { setEcosystem('node'); setSelected(new Set()); }}>Node.js <span>{formatBytes(projects.reduce((s,p)=>s+p.sizeBytes,0))}</span></button><button className={ecosystem === 'gradle' ? 'active' : ''} onClick={() => { setEcosystem('gradle'); setSelected(new Set()); }}>Gradle <span>{formatBytes(gradleScan.recoverableSizeBytes)}</span></button></div>
            <div className="results-head">
              <div><p className="section-label">{ecosystem === 'node' ? 'Projetos Node.js' : 'Gradle storage management'}</p><h2>{visibleItems.length} {visibleItems.length === 1 ? 'item encontrado' : 'itens encontrados'}</h2></div>
              <div className="recoverable"><strong>{formatBytes(totalBytes)}</strong><span>recuperáveis</span></div>
            </div>

            {visibleItems.length > 0 ? <>
              {ecosystem === 'gradle' && <div className="presets"><span>Estratégia</span><button onClick={()=>applyPreset('conservative')}>Conservative</button><button onClick={()=>applyPreset('balanced')}>Balanced</button><button onClick={()=>applyPreset('deep')}>Deep clean</button></div>}
              <button className="select-all" onClick={toggleAll}><span className={`checkbox ${allSelected ? 'checked' : ''}`}>{allSelected ? '✓' : ''}</span> Selecionar todos</button>
              <div className="project-list">
                {ecosystem === 'node' ? projects.map((project) => {
                  const age = projectAge(project.lastModified);
                  const checked = selected.has(project.id);
                  return <button key={project.id} className={`project-card ${checked ? 'selected' : ''}`} onClick={() => toggle(project.id)}>
                    <span className={`checkbox ${checked ? 'checked' : ''}`}>{checked ? '✓' : ''}</span>
                    <span className="project-main"><strong>{project.name}</strong><small title={project.path}>{project.path}</small><span className="meta">{age.text} <i className={`age ${age.tone}`}>{age.badge}</i></span></span>
                    <strong className="size">{formatBytes(project.sizeBytes)}</strong>
                  </button>;
                }) : gradleScan.items.map((item) => {
                  const checked = selected.has(item.id); const age = projectAge(item.lastModified);
                  return <button key={item.id} disabled={!item.deletable} className={`project-card ${checked ? 'selected' : ''} ${!item.deletable ? 'protected' : ''}`} onClick={() => toggle(item.id)}>
                    <span className={`checkbox ${checked ? 'checked' : ''}`}>{checked ? '✓' : item.deletable ? '' : '•'}</span>
                    <span className="project-main"><strong>{item.name}</strong><small title={item.path}>{item.path}</small><span className="meta">{item.description} {item.usedBy?.length > 0 && ` Used by: ${item.usedBy.join(', ')}`}</span></span>
                    <span className="gradle-side"><strong className="size">{formatBytes(item.sizeBytes)}</strong><i className={`risk ${item.riskLevel.toLowerCase()}`}>{item.riskLevel}</i><small>{age.text}</small></span>
                  </button>;
                })}
              </div>
              <div className="action-bar"><p><span>Selecionados</span><strong>{selected.size} {selected.size === 1 ? 'item' : 'itens'} · {formatBytes(selectedBytes)}</strong></p><button className="danger" disabled={!selected.size} onClick={() => setConfirmOpen(true)}>Revisar limpeza <span>⌫</span></button></div>
            </> : <div className="empty"><span>✓</span><h3>Nada para limpar por aqui</h3><p>Nenhum projeto com <code>node_modules</code> foi encontrado nessa pasta.</p></div>}
          </section>
        )}
      </main>

      <footer><span>NodeSweep</span><p>Somente <code>node_modules</code>. Seus projetos permanecem intactos.</p></footer>

      {confirmOpen && <div className="modal-backdrop" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && setConfirmOpen(false)}>
        <div className="modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
          <div className="modal-icon">!</div><p className="section-label">Confirmação necessária</p><h2 id="confirm-title">Remover dependências selecionadas?</h2>
          <p>Você removerá <strong>{selected.size} {selected.size === 1 ? 'item reconstruível' : 'itens reconstruíveis'}</strong> e recuperará aproximadamente <strong>{formatBytes(selectedBytes)}</strong>.</p>
          <div className="modal-note">O código-fonte e as configurações protegidas não serão alterados. Builds podem ser recompilados e dependências baixadas novamente.</div>
          <div className="modal-actions"><button className="secondary" onClick={() => setConfirmOpen(false)} disabled={status === 'cleaning'}>Cancelar</button><button className="danger" onClick={cleanup} disabled={status === 'cleaning'}>{status === 'cleaning' ? <><i className="spinner" /> Limpando</> : 'Sim, limpar agora'}</button></div>
        </div>
      </div>}
    </div>
  );
}

export default App;
