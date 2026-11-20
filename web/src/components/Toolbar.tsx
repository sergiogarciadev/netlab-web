import { Pause, Play, SkipForward, Clock, Activity } from 'lucide-react';
import { useSimStore } from '../store/useSimStore';

const SPEED_OPTIONS = [0.1, 0.25, 0.5, 1, 2, 5, 10];

export function Toolbar() {
  const ready = useSimStore((s) => s.ready);
  const playing = useSimStore((s) => s.playing);
  const speed = useSimStore((s) => s.speed);
  const nowMs = useSimStore((s) => s.nowMs);
  const setPlaying = useSimStore((s) => s.setPlaying);
  const setSpeed = useSimStore((s) => s.setSpeed);
  const stepOnce = useSimStore((s) => s.stepOnce);

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
          onClick={() => stepOnce(10)}
          disabled={!ready || playing}
          title="Step +10ms"
          className="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-200 transition-colors hover:bg-slate-700 disabled:opacity-40"
        >
          <SkipForward className="h-3.5 w-3.5" />
          Step +10ms
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
