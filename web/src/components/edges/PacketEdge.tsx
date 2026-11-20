import { useEffect, useRef, useState } from 'react';
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from '@xyflow/react';
import { useSimStore, type NetEdgeData } from '../../store/useSimStore';

const PROTOCOL_COLORS: Record<string, string> = {
  ICMP: '#38bdf8',
  ARP: '#fbbf24',
  TCP: '#34d399',
  UDP: '#a78bfa',
  IPv4: '#94a3b8',
};

export function PacketEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  selected,
  data,
}: EdgeProps) {
  const edgeData = data as NetEdgeData | undefined;
  const isUp = edgeData?.params?.up ?? true;
  const loss = edgeData?.params?.loss_percent ?? 0;

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const pathRef = useRef<SVGPathElement | null>(null);
  const allPackets = useSimStore((s) => s.activePackets);
  const edgePackets = allPackets.filter((p) => p.cable_id === id);

  const [dotPositions, setDotPositions] = useState<
    Array<{ id: string; x: number; y: number; color: string }>
  >([]);

  useEffect(() => {
    if (edgePackets.length === 0) {
      setDotPositions([]);
      return;
    }

    let rafId = 0;
    const update = () => {
      const pathEl = pathRef.current;
      if (!pathEl) return;
      const totalLen = pathEl.getTotalLength();
      const now = performance.now();

      const next: Array<{ id: string; x: number; y: number; color: string }> = [];
      for (const pkt of edgePackets) {
        const elapsed = now - pkt.createdAtPerfMs;
        const t = Math.min(1, Math.max(0, elapsed / pkt.durationMs));
        if (t >= 1) continue;
        const dist = pkt.direction === 'AToB' ? t * totalLen : (1 - t) * totalLen;
        const pt = pathEl.getPointAtLength(dist);
        next.push({
          id: pkt.id,
          x: pt.x,
          y: pt.y,
          color: PROTOCOL_COLORS[pkt.protocol] ?? '#f8fafc',
        });
      }
      setDotPositions(next);
      if (next.length > 0) {
        rafId = requestAnimationFrame(update);
      }
    };

    rafId = requestAnimationFrame(update);
    return () => cancelAnimationFrame(rafId);
  }, [edgePackets.length, edgePath]);

  const strokeColor = !isUp
    ? '#ef4444'
    : selected
      ? '#38bdf8'
      : loss > 0
        ? '#f59e0b'
        : '#475569';

  return (
    <>
      <path ref={pathRef} d={edgePath} fill="none" stroke="none" />
      <BaseEdge
        id={id}
        path={edgePath}
        style={{
          stroke: strokeColor,
          strokeWidth: selected ? 2.5 : 2,
          strokeDasharray: !isUp ? '6 4' : undefined,
        }}
      />
      {dotPositions.map((dot) => (
        <circle
          key={dot.id}
          cx={dot.x}
          cy={dot.y}
          r={4.5}
          fill={dot.color}
          stroke="#020617"
          strokeWidth={1.5}
          className="pointer-events-none"
        />
      ))}
      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            pointerEvents: 'all',
          }}
          className={`rounded border px-1.5 py-0.5 font-mono text-[9px] transition-colors ${
            selected
              ? 'border-sky-400 bg-slate-900 text-sky-300'
              : 'border-slate-800 bg-slate-950/85 text-slate-400'
          }`}
        >
          {edgeData ? `${edgeData.portA} ↔ ${edgeData.portB}` : id}
          {!isUp && <span className="ml-1 text-red-400">DOWN</span>}
        </div>
      </EdgeLabelRenderer>
    </>
  );
}
