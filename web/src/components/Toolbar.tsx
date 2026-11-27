import { useRef } from 'react';
import {
  Pause,
  Play,
  SkipForward,
  Clock,
  Activity,
  FolderOpen,
  Download,
  Upload,
  RotateCcw,
} from 'lucide-react';
import { useSimStore } from '../store/useSimStore';
import { STARTER_LABS, type TopologySnapshot } from '../labs/starterLabs';

const SPEED_OPTIONS = [0.1, 0.25, 0.5, 1, 2, 5, 10];

export function Toolbar() {
  const ready = useSimStore((s) => s.ready);
  const playing = useSimStore((s) => s.playing);
  const speed = useSimStore((s) => s.speed);
  const nowMs = useSimStore((s) => s.nowMs);
  const setPlaying = useSimStore((s) => s.setPlaying);
  const setSpeed = useSimStore((s) => s.setSpeed);
  const stepOnce = useSimStore((s) => s.stepOnce);
  const stepHop = useSimStore((s) => s.stepHop);
  const restartSimulation = useSimStore((s) => s.restartSimulation);
  const loadTopology = useSimStore((s) => s.loadTopology);
  const exportTopology = useSimStore((s) => s.exportTopology);

  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const handleExportJson = () => {
    const snap = exportTopology();
    const blob = new Blob([JSON.stringify(snap, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `netlab-topology-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleImportJson = async (file: File | undefined) => {
    if (!file) return;
    try {
      const text = await file.text();
      const snap = JSON.parse(text) as TopologySnapshot;
      if (snap && Array.isArray(snap.nodes) && Array.isArray(snap.edges)) {
        loadTopology(snap);
      }
    } catch (err) {
      console.error('Failed to import topology JSON:', err);
    }
  };

  return (
    <header className="flex h-12 items-center justify-between border-b border-slate-800 bg-slate-900/95 px-4">
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <Activity className="h-5 w-5 text-sky-400" />
          <span className="text-sm font-bold tracking-tight text-slate-100">NetLab</span>
          <span className="rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[10px] text-sky-400">
            smoltcp + WASM
          </span>
        </div>

        <div className="ml-2 flex items-center gap-1.5 rounded-lg border border-slate-700 bg-slate-950 px-2 py-1">
          <FolderOpen className="h-3.5 w-3.5 text-amber-400" />
          <select
            disabled={!ready}
            defaultValue=""
            onChange={(e) => {
              const idx = Number(e.target.value);
              if (!Number.isNaN(idx) && STARTER_LABS[idx]) {
                loadTopology(STARTER_LABS[idx]);
              }
              e.target.value = '';
            }}
            className="bg-transparent text-xs text-slate-200 focus:outline-none disabled:opacity-40"
          >
            <option value="" disabled className="bg-slate-900">
              Starter Labs...
            </option>
            {STARTER_LABS.map((lab, i) => (
              <option key={lab.name} value={i} className="bg-slate-900">
                {lab.name}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={handleExportJson}
          disabled={!ready}
          title="Export Topology JSON"
          className="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-700 disabled:opacity-40"
        >
          <Download className="h-3 w-3 text-sky-400" />
          Export
        </button>

        <button
          onClick={() => fileInputRef.current?.click()}
          disabled={!ready}
          title="Import Topology JSON"
          className="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-700 disabled:opacity-40"
        >
          <Upload className="h-3 w-3 text-emerald-400" />
          Import
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept="application/json,.json"
          className="hidden"
          onChange={(e) => {
            handleImportJson(e.target.files?.[0]);
            e.target.value = '';
          }}
        />

        {!ready && (
          <span className="animate-pulse font-mono text-xs text-amber-400">
            Initializing WASM Worker...
          </span>
        )}
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={() => setPlaying(!playing)}
          disabled={!ready}
          title="Play / Pause (Space)"
          className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-40 ${
            playing
              ? 'bg-amber-500/20 text-amber-300 hover:bg-amber-500/30'
              : 'bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30'
          }`}
        >
          {playing ? <Pause className="h-3.5 w-3.5" /> : <Play className="h-3.5 w-3.5" />}
          {playing ? 'Pause' : 'Run'}
        </button>

        <button
          onClick={() => {
            if (playing) setPlaying(false);
            stepHop();
          }}
          disabled={!ready}
          title="Advance 1 Hop (H)"
          className="flex items-center gap-1 rounded-lg border border-sky-500/40 bg-sky-500/15 px-2.5 py-1.5 text-xs font-medium text-sky-300 transition-colors hover:bg-sky-500/25 disabled:opacity-40"
        >
          <SkipForward className="h-3.5 w-3.5" />
          Next Hop
        </button>

        <button
          onClick={() => stepOnce(10)}
          disabled={!ready || playing}
          title="Step +10ms (S)"
          className="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-200 transition-colors hover:bg-slate-700 disabled:opacity-40"
        >
          <SkipForward className="h-3.5 w-3.5" />
          +10ms
        </button>

        <button
          onClick={restartSimulation}
          disabled={!ready}
          title="Restart simulation (preserves Play/Pause state)"
          className="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-200 transition-colors hover:bg-slate-700 disabled:opacity-40"
        >
          <RotateCcw className="h-3.5 w-3.5 text-rose-400" />
          Restart
        </button>

        <div className="ml-2 flex items-center gap-1.5 rounded-lg border border-slate-800 bg-slate-950 px-2.5 py-1">
          <span className="text-[11px] text-slate-400">Speed</span>
          <select
            value={speed}
            onChange={(e) => setSpeed(Number(e.target.value))}
            className="bg-transparent font-mono text-xs text-slate-200 focus:outline-none"
          >
            {SPEED_OPTIONS.map((opt) => (
              <option key={opt} value={opt} className="bg-slate-900">
                {opt}x
              </option>
            ))}
          </select>
        </div>

        <div className="ml-2 flex items-center gap-1.5 rounded-lg border border-slate-800 bg-slate-950 px-3 py-1 font-mono text-xs text-slate-300">
          <Clock className="h-3.5 w-3.5 text-slate-500" />
          <span>{(nowMs / 1000).toFixed(3)}s</span>
        </div>
      </div>
    </header>
  );
}
