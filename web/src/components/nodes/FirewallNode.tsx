import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Shield } from 'lucide-react';
import { useSimStore, type NetNodeData } from '../../store/useSimStore';

export function FirewallNode({ id, data, selected }: NodeProps) {
  const nodeData = data as NetNodeData;
  const fwState = useSimStore((s) => s.firewalls[id]);

  return (
    <div
      className={`min-w-[165px] rounded-xl border bg-slate-900/95 px-3.5 py-2.5 shadow-lg transition-all ${
        selected
          ? 'border-rose-400 ring-2 ring-rose-400/30'
          : 'border-slate-700 hover:border-slate-500'
      }`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-rose-400"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-rose-400"
      />
      <div className="flex items-center gap-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-rose-500/15 text-rose-400">
          <Shield className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-slate-100">
            {fwState?.name ?? nodeData.label}
          </div>
          <div className="font-mono text-[10px] text-slate-400">
            Firewall · {fwState?.rules.length ?? 0} rules
          </div>
        </div>
      </div>
      {fwState && (
        <div className="mt-1.5 flex items-center justify-between border-t border-slate-800 pt-1 font-mono text-[9px]">
          <span className="text-emerald-400">{fwState.packets_allowed} allow</span>
          <span className="text-rose-400">{fwState.packets_dropped} drop</span>
        </div>
      )}
    </div>
  );
}
