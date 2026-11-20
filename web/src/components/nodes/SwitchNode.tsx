import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Network } from 'lucide-react';
import { useSimStore, type NetNodeData } from '../../store/useSimStore';

export function SwitchNode({ id, data, selected }: NodeProps) {
  const nodeData = data as NetNodeData;
  const swState = useSimStore((s) => s.switches[id]);
  const macCount = swState ? Object.keys(swState.mac_table).length : 0;

  return (
    <div
      className={`min-w-[145px] rounded-xl border bg-slate-900/95 px-3.5 py-2.5 shadow-lg transition-all ${
        selected
          ? 'border-emerald-400 ring-2 ring-emerald-400/30'
          : 'border-slate-700 hover:border-slate-500'
      }`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-emerald-400"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-emerald-400"
      />
      <div className="flex items-center gap-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-400">
          <Network className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-slate-100">
            {swState?.name ?? nodeData.label}
          </div>
          <div className="font-mono text-[10px] text-slate-400">
            L2 Switch · {swState?.ports.length ?? 0} ports
          </div>
        </div>
      </div>
      <div className="mt-1.5 flex items-center justify-between border-t border-slate-800 pt-1 font-mono text-[9px] text-slate-500">
        <span>MAC Table</span>
        <span className="text-emerald-400">{macCount} learned</span>
      </div>
    </div>
  );
}
