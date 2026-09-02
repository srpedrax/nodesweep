import { useEffect, useMemo, useState } from 'react';
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
  const [drives, setDrives] = useState([]);
  const [systemScan, setSystemScan] = useState({ items: [], totalSizeBytes: 0, recommendedSizeBytes: 0 });
  const [startupStatus, setStartupStatus] = useState('loading');

  useEffect(() => {
    let active = true;
    Promise.all([invoke('discover_storage'), invoke('scan_system')])
      .then(([storage, system]) => {
        if (!active) return;
        setDrives(storage); setSystemScan(system); setEcosystem('system'); setHasScanned(true);
      })
      .catch((startupError) => active && setError(startupError?.message || String(startupError)))
      .finally(() => active && setStartupStatus('idle'));
    return () => { active = false; };
  }, []);

  const visibleItems = ecosystem === 'node' ? projects : ecosystem === 'gradle' ? gradleScan.items : systemScan.items;
  const totalBytes = ecosystem === 'node' ? projects.reduce((sum, item) => sum + item.sizeBytes, 0) : ecosystem === 'gradle' ? gradleScan.recoverableSizeBytes : systemScan.totalSizeBytes;
  const selectedProjects = useMemo(() => projects.filter((item) => selected.has(item.id)), [projects, selected]);
  const selectedGradle = gradleScan.items.filter((item) => selected.has(item.id));
  const selectedSystem = systemScan.items.filter((item) => selected.has(item.id));
  const selectedBytes = (ecosystem === 'node' ? selectedProjects : ecosystem === 'gradle' ? selectedGradle : selectedSystem).reduce((sum, item) => sum + item.sizeBytes, 0);
  const selectableItems = ecosystem === 'node' ? projects : ecosystem === 'gradle' ? gradleScan.items.filter((item) => item.deletable) : systemScan.items.filter((item) => item.deletable);
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
    if (ecosystem === 'system') {
      setSelected(new Set(systemScan.items.filter((item) => item.deletable && (preset !== 'recommended' || item.recommendation === 'Recommended')).map((item) => item.id)));
      return;
    }
    const categories = preset === 'conservative' ? ['BuildOutput', 'ProjectCache', 'Daemon'] : preset === 'balanced' ? ['BuildOutput', 'ProjectCache', 'Daemon', 'WrapperDistribution'] : ['BuildOutput', 'ProjectCache', 'Daemon', 'WrapperDistribution', 'GlobalCache'];
    setSelected(new Set(gradleScan.items.filter((item) => item.deletable && categories.includes(item.category)).map((item) => item.id)));
  }

  async function cleanup() {
    setStatus('cleaning'); setError('');
    try {
      const data = ecosystem === 'node'
        ? await invoke('cleanup_projects', { ids: selectedProjects.map((item) => item.id), confirmed: true })
        : ecosystem === 'gradle'
          ? await invoke('cleanup_gradle', { ids: selectedGradle.map((item) => item.id), confirmed: true })
          : await invoke('cleanup_system', { ids: selectedSystem.map((item) => item.id), confirmed: true });
      if (ecosystem === 'node') setProjects((current) => current.filter((item) => !selected.has(item.id)));
      else if (ecosystem === 'gradle') setGradleScan((current) => ({ ...current, items: current.items.filter((item) => !selected.has(item.id)), recoverableSizeBytes: Math.max(0, current.recoverableSizeBytes - data.totalFreedBytes) }));
      else setSystemScan((current) => ({ ...current, items: current.items.filter((item) => !selected.has(item.id)), totalSizeBytes: Math.max(0, current.totalSizeBytes - data.requestedBytes), recommendedSizeBytes: Math.max(0, current.recommendedSizeBytes - data.requestedBytes) }));
      setSelected(new Set()); setFreedBytes(data.totalFreedBytes ?? data.freedBytes); setConfirmOpen(false);
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
        <section className="storage-overview" aria-busy={startupStatus === 'loading'}>
          <div className="results-head"><div><p className="section-label">Armazenamento detectado</p><h2>{startupStatus === 'loading' ? 'Analisando este computador…' : `${drives.length} ${drives.length === 1 ? 'unidade encontrada' : 'unidades encontradas'}`}</h2></div><div className="recoverable"><strong>{formatBytes(systemScan.recommendedSizeBytes)}</strong><span>limpeza recomendada</span></div></div>
          <div className="drive-grid">{drives.map((drive) => { const used = Math.max(0, drive.totalBytes - drive.freeBytes); const percentage = drive.totalBytes ? Math.round(used / drive.totalBytes * 100) : 0; return <article className={`drive-card ${drive.autoSelected ? 'selected' : ''}`} key={drive.id}><div><strong>{drive.label || `Disco local (${drive.mountPoint.slice(0, 2)})`}</strong><span>{drive.isSystem ? 'Sistema' : drive.driveType}{drive.isRemovable ? ' · removível' : ''}</span></div><p>{formatBytes(drive.freeBytes)} livres de {formatBytes(drive.totalBytes)}</p><div className="capacity"><i style={{ width: `${percentage}%` }} /></div></article>; })}</div>
        </section>
        <section className="hero">
          <p className="eyebrow"><span /> Espaço limpo. Projetos intactos.</p>
          <h1>Veja o que ocupa espaço.<br /><em>Recupere com segurança.</em></h1>
          <p className="hero-copy">O NodeSweep encontra resíduos do sistema e artefatos reconstruíveis de desenvolvimento.</p>

          <form className="scan-panel" onSubmit={scan}>
            <label htmlFor="rootPath">Adicionar local de desenvolvimento</label>
            <div className="input-row">
              <div className="path-input"><span aria-hidden="true">⌘</span><input id="rootPath" value={rootPath} onChange={(e) => setRootPath(e.target.value)} placeholder="C:\\Users\\...\\Projetos" disabled={status !== 'idle'} /><button type="button" className="browse" onClick={chooseFolder} disabled={status !== 'idle'}>Escolher pasta</button></div>
              <button className="primary" disabled={status !== 'idle'}>{status === 'scanning' ? <><i className="spinner" /> Escaneando</> : <>Escanear <span>→</span></>}</button>
            </div>
            <p className="hint">Opcional: adicione uma pasta específica para procurar projetos Node.js e Gradle.</p>
          </form>
        </section>

        {error && <div className="notice error" role="alert"><strong>Não foi possível concluir</strong><span>{error}</span><button onClick={() => setError('')} aria-label="Fechar">×</button></div>}
        {freedBytes !== null && <div className="notice success" role="status"><strong>Limpeza concluída</strong><span>{formatBytes(freedBytes)} recuperados com segurança.</span><button onClick={() => setFreedBytes(null)} aria-label="Fechar">×</button></div>}

        {hasScanned && status !== 'scanning' && (
          <section className="results">
            <div className="ecosystem-tabs"><button className={ecosystem === 'system' ? 'active' : ''} onClick={() => { setEcosystem('system'); setSelected(new Set()); }}>Sistema <span>{formatBytes(systemScan.totalSizeBytes)}</span></button><button className={ecosystem === 'node' ? 'active' : ''} onClick={() => { setEcosystem('node'); setSelected(new Set()); }}>Node.js <span>{formatBytes(projects.reduce((s,p)=>s+p.sizeBytes,0))}</span></button><button className={ecosystem === 'gradle' ? 'active' : ''} onClick={() => { setEcosystem('gradle'); setSelected(new Set()); }}>Gradle <span>{formatBytes(gradleScan.recoverableSizeBytes)}</span></button></div>
            <div className="results-head">
              <div><p className="section-label">{ecosystem === 'node' ? 'Projetos Node.js' : ecosystem === 'gradle' ? 'Gradle storage management' : 'Limpeza do sistema'}</p><h2>{visibleItems.length} {visibleItems.length === 1 ? 'item encontrado' : 'itens encontrados'}</h2></div>
              <div className="recoverable"><strong>{formatBytes(totalBytes)}</strong><span>recuperáveis</span></div>
            </div>

            {visibleItems.length > 0 ? <>
              {ecosystem === 'gradle' && <div className="presets"><span>Estratégia</span><button onClick={()=>applyPreset('conservative')}>Conservative</button><button onClick={()=>applyPreset('balanced')}>Balanced</button><button onClick={()=>applyPreset('deep')}>Deep clean</button></div>}
              {ecosystem === 'system' && <div className="presets"><span>Seleção segura</span><button onClick={()=>applyPreset('recommended')}>Recomendado</button><button onClick={()=>applyPreset('all')}>Todos (revisar)</button></div>}
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
                }) : ecosystem === 'gradle' ? gradleScan.items.map((item) => {
                  const checked = selected.has(item.id); const age = projectAge(item.lastModified);
                  return <button key={item.id} disabled={!item.deletable} className={`project-card ${checked ? 'selected' : ''} ${!item.deletable ? 'protected' : ''}`} onClick={() => toggle(item.id)}>
                    <span className={`checkbox ${checked ? 'checked' : ''}`}>{checked ? '✓' : item.deletable ? '' : '•'}</span>
                    <span className="project-main"><strong>{item.name}</strong><small title={item.path}>{item.path}</small><span className="meta">{item.description} {item.usedBy?.length > 0 && ` Used by: ${item.usedBy.join(', ')}`}</span></span>
                    <span className="gradle-side"><strong className="size">{formatBytes(item.sizeBytes)}</strong><i className={`risk ${item.riskLevel.toLowerCase()}`}>{item.riskLevel}</i><small>{age.text}</small></span>
                  </button>;
                }) : systemScan.items.map((item) => { const checked = selected.has(item.id); return <button key={item.id} className={`project-card ${checked ? 'selected' : ''}`} onClick={() => toggle(item.id)}><span className={`checkbox ${checked ? 'checked' : ''}`}>{checked ? '✓' : ''}</span><span className="project-main"><strong>{item.displayName}</strong><small title={item.path}>{item.path}</small><span className="meta">{item.consequence}</span></span><span className="gradle-side"><strong className="size">{formatBytes(item.sizeBytes)}</strong><i className={`risk ${item.riskLevel.toLowerCase()}`}>{item.riskLevel}</i><small>{item.recommendation === 'Recommended' ? 'Recomendado' : 'Opcional'}</small></span></button>; })}
              </div>
              <div className="action-bar"><p><span>Selecionados</span><strong>{selected.size} {selected.size === 1 ? 'item' : 'itens'} · {formatBytes(selectedBytes)}</strong></p><button className="danger" disabled={!selected.size} onClick={() => setConfirmOpen(true)}>Revisar limpeza <span>⌫</span></button></div>
            </> : <div className="empty"><span>✓</span><h3>Nada para limpar por aqui</h3><p>{ecosystem === 'system' ? 'Nenhum resíduo elegível foi encontrado neste computador.' : <>Nenhum artefato reconstruível foi encontrado nessa pasta.</>}</p></div>}
          </section>
        )}
      </main>

      <footer><span>NodeSweep</span><p>Limpeza explícita, validada e limitada a alvos reconstruíveis.</p></footer>

      {confirmOpen && <div className="modal-backdrop" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && setConfirmOpen(false)}>
        <div className="modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
          <div className="modal-icon">!</div><p className="section-label">Confirmação necessária</p><h2 id="confirm-title">Limpar os itens selecionados?</h2>
          <p>Você removerá <strong>{selected.size} {selected.size === 1 ? 'item reconstruível' : 'itens reconstruíveis'}</strong> e recuperará aproximadamente <strong>{formatBytes(selectedBytes)}</strong>.</p>
          <div className="modal-note">Os alvos serão validados novamente antes da primeira exclusão. Itens bloqueados pelo Windows serão ignorados e informados no resultado.</div>
          <div className="modal-actions"><button className="secondary" onClick={() => setConfirmOpen(false)} disabled={status === 'cleaning'}>Cancelar</button><button className="danger" onClick={cleanup} disabled={status === 'cleaning'}>{status === 'cleaning' ? <><i className="spinner" /> Limpando</> : 'Sim, limpar agora'}</button></div>
        </div>
      </div>}
    </div>
  );
}

export default App;
