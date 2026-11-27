import { useSimStore } from '../../store/useSimStore';

const TONE_STYLES: Record<string, string> = {
  info: 'border-sky-400/70 bg-slate-900/95 text-sky-100',
  success: 'border-emerald-400/70 bg-slate-900/95 text-emerald-100',
  warn: 'border-amber-400/70 bg-slate-900/95 text-amber-100',
  danger: 'border-rose-400/70 bg-slate-900/95 text-rose-100',
};

const BADGE_STYLES: Record<string, string> = {
  info: 'bg-sky-500/20 text-sky-300',
  success: 'bg-emerald-500/20 text-emerald-300',
  warn: 'bg-amber-500/20 text-amber-300',
  danger: 'bg-rose-500/20 text-rose-300',
};

export function NodeCallout({ nodeId }: { nodeId: string }) {
  const callout = useSimStore((s) => s.nodeCallouts[nodeId]);
  if (!callout) return null;

  const toneClass = TONE_STYLES[callout.tone] ?? TONE_STYLES.info;
  const badgeClass = BADGE_STYLES[callout.tone] ?? BADGE_STYLES.info;

  return (
    <div className="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 w-56 -translate-x-1/2">
      <div className={`rounded-xl border px-3 py-2 shadow-2xl backdrop-blur-sm ${toneClass}`}>
        <div className="mb-0.5 flex items-center justify-between gap-1.5">
          <span className={`rounded px-1.5 py-0.5 font-mono text-[9px] font-semibold ${badgeClass}`}>
            {callout.title}
          </span>
          <span className="font-mono text-[9px] opacity-60">{callout.protocol}</span>
        </div>
        <div className="text-[11px] leading-snug">{callout.message}</div>
      </div>
      <div className="mx-auto h-2 w-2 -translate-y-1 rotate-45 border-b border-r border-slate-600 bg-slate-900" />
    </div>
  );
}
