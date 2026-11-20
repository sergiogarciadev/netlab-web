import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Monitor } from 'lucide-react';
import { useSimStore, type NetNodeData } from '../../store/useSimStore';

export function HostNode({ id, data, selected }: NodeProps) {
  const nodeData = data as NetNodeData;
  const hostState = useSimStore((s) => s.hosts[id]);

  return (
    <div
      className={`min-w-[150px] rounded-xl border bg-slate-900/95 px-3.5 py-2.5 shadow-lg transition-all ${
        selected
          ? 'border-sky-400 ring-2 ring-sky-400/30'
          : 'border-slate-700 hover:border-slate-500'
      }`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-sky-400"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!h-3 !w-3 !border-2 !border-slate-900 !bg-sky-400"
      />
      <div className="flex items-center gap-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-sky-500/15 text-sky-400">
          <Monitor className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-slate-100">
            {hostState?.name ?? nodeData.label}
          </div>
          <div className="truncate font-mono text-[10px] text-slate-400">
            {hostState?.ip ? `${hostState.ip}/${hostState.prefix_len}` : 'No IP'}
          </div>
        </div>
      </div>
      {hostState && (
        <div className="mt-1.5 flex items-center justify-between border-t border-slate-800 pt-1 font-mono text-[9px] text-slate-500">
          <span>{hostState.mac}</span>
          {hostState.ping_replies.length > 0 && (
            <span className="rounded bg-emerald-500/15 px-1 text-emerald-400">
              {hostState.ping_replies.length} pong
            </span>
          )}
        </div>
      )}
    </div>
  );
}
