import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Router } from 'lucide-react';
import { useSimStore, type NetNodeData } from '../../store/useSimStore';
import { NodeCallout } from './NodeCallout';

export function RouterNode({ id, data, selected }: NodeProps) {
  const nodeData = data as NetNodeData;
  const routerState = useSimStore((s) => s.routers[id]);

  return (
    <div
      className={`relative min-w-[160px] rounded-xl border bg-slate-900/95 px-3.5 py-2.5 shadow-lg transition-all ${
        selected
          ? 'border-amber-400 ring-2 ring-amber-400/30'
          : 'border-slate-700 hover:border-slate-500'
      }`}
    >
      <NodeCallout nodeId={id} />
      <Handle
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-amber-400"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-amber-400"
      />
      <div className="flex items-center gap-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-500/15 text-amber-400">
          <Router className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-slate-100">
            {routerState?.name ?? nodeData.label}
          </div>
          <div className="font-mono text-[10px] text-slate-400">
            L3 Router · {routerState?.interfaces.length ?? 0} NICs
          </div>
        </div>
      </div>
      {routerState && routerState.interfaces.length > 0 && (
        <div className="mt-1.5 space-y-0.5 border-t border-slate-800 pt-1 font-mono text-[9px] text-slate-400">
          {routerState.interfaces.slice(0, 3).map((iface) => (
            <div key={iface.id} className="flex justify-between">
              <span className="text-slate-500">{iface.id}</span>
              <span>{iface.ip ? `${iface.ip}/${iface.prefix_len}` : 'unassigned'}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
