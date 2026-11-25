import type { DragEvent } from 'react';
import { Monitor, Network, Router, Shield, Plus } from 'lucide-react';
import { useSimStore } from '../store/useSimStore';
import type { NodeKind } from '../types/sim';

const PALETTE_ITEMS: Array<{
  kind: NodeKind;
  title: string;
  subtitle: string;
  color: string;
  Icon: typeof Monitor;
}> = [
  {
    kind: 'host',
    title: 'Host',
    subtitle: 'smoltcp endpoint',
    color: 'text-sky-400 bg-sky-500/15',
    Icon: Monitor,
  },
  {
    kind: 'switch',
    title: 'L2 Switch',
    subtitle: 'MAC-learning bridge',
    color: 'text-emerald-400 bg-emerald-500/15',
    Icon: Network,
  },
  {
    kind: 'router',
    title: 'L3 Router',
    subtitle: 'Multi-NIC IPv4 router',
    color: 'text-amber-400 bg-amber-500/15',
    Icon: Router,
  },
  {
    kind: 'firewall',
    title: 'Firewall',
    subtitle: 'L3 packet filter & ACLs',
    color: 'text-rose-400 bg-rose-500/15',
    Icon: Shield,
  },
];

export function Sidebar() {
  const addNode = useSimStore((s) => s.addNode);
  const ready = useSimStore((s) => s.ready);

  const onDragStart = (e: DragEvent, kind: NodeKind) => {
    e.dataTransfer.setData('application/netlab-node', kind);
    e.dataTransfer.effectAllowed = 'move';
  };

  return (
    <aside className="flex w-56 flex-col border-r border-slate-800 bg-slate-900/80 p-3">
      <div className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
        Network Devices
      </div>
      <div className="space-y-2">
        {PALETTE_ITEMS.map(({ kind, title, subtitle, color, Icon }) => (
          <button
            key={kind}
            draggable={ready}
            onDragStart={(e) => onDragStart(e, kind)}
            onClick={() => ready && addNode(kind)}
            disabled={!ready}
            className="group flex w-full items-center gap-2.5 rounded-xl border border-slate-800 bg-slate-950/70 p-2.5 text-left transition-all hover:border-slate-600 hover:bg-slate-800/70 disabled:opacity-40"
          >
            <div className={`flex h-8 w-8 items-center justify-center rounded-lg ${color}`}>
              <Icon className="h-4 w-4" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-xs font-semibold text-slate-200">{title}</div>
              <div className="text-[10px] text-slate-500">{subtitle}</div>
            </div>
            <Plus className="h-3.5 w-3.5 text-slate-600 transition-colors group-hover:text-slate-300" />
          </button>
        ))}
      </div>

      <div className="mt-6 rounded-xl border border-slate-800/80 bg-slate-950/60 p-3 text-[11px] leading-relaxed text-slate-400">
        <div className="mb-1 font-semibold text-slate-300">Quick Guide</div>
        <ul className="list-disc space-y-1 pl-3.5">
          <li>Click or drag devices onto the canvas.</li>
          <li>Drag between handles to wire a VirtualCable.</li>
          <li>Click a node to assign IPs or send ICMP pings.</li>
          <li>Press <span className="text-emerald-400">Run</span> to start the virtual clock.</li>
        </ul>
      </div>
    </aside>
  );
}
