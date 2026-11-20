import { useMemo, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Download,
  Filter,
  Trash2,
  TerminalSquare,
} from 'lucide-react';
import { useSimStore } from '../store/useSimStore';
import type { CapturedPacket } from '../types/sim';

const PROTO_BADGE: Record<string, string> = {
  ICMP: 'bg-sky-500/20 text-sky-300 border-sky-500/30',
  ARP: 'bg-amber-500/20 text-amber-300 border-amber-500/30',
  TCP: 'bg-emerald-500/20 text-emerald-300 border-emerald-500/30',
  HTTP: 'bg-teal-500/20 text-teal-300 border-teal-500/30',
  UDP: 'bg-violet-500/20 text-violet-300 border-violet-500/30',
  DNS: 'bg-indigo-500/20 text-indigo-300 border-indigo-500/30',
  DHCP: 'bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/30',
};

function matchesBpfFilter(pkt: CapturedPacket, rawFilter: string): boolean {
  const f = rawFilter.trim().toLowerCase();
  if (!f) return true;

  const s = pkt.summary;
  const proto = s.protocol.toLowerCase();

  // Support `&&` or space-separated clauses
  const clauses = f.split(/\s+&&\s+/).map((c) => c.trim());
  for (const clause of clauses) {
    if (clause.includes('==')) {
      const [lhs, rhs] = clause.split('==').map((x) => x.trim());
      if (lhs === 'ip') {
        if (s.src_ip !== rhs && s.dst_ip !== rhs) return false;
      } else if (lhs === 'ip.src') {
        if (s.src_ip !== rhs) return false;
      } else if (lhs === 'ip.dst') {
        if (s.dst_ip !== rhs) return false;
      } else if (lhs === 'tcp.port') {
        const port = Number(rhs);
        if (proto !== 'tcp' && proto !== 'http') return false;
        if (s.src_port !== port && s.dst_port !== port) return false;
      } else if (lhs === 'udp.port') {
        const port = Number(rhs);
        if (proto !== 'udp' && proto !== 'dns' && proto !== 'dhcp') return false;
        if (s.src_port !== port && s.dst_port !== port) return false;
      } else if (lhs === 'port') {
        const port = Number(rhs);
        if (s.src_port !== port && s.dst_port !== port) return false;
      } else if (lhs === 'link') {
        if (pkt.cable_id.toLowerCase() !== rhs) return false;
      }
    } else {
      const hay = `${proto} ${s.src_ip ?? ''} ${s.dst_ip ?? ''} ${s.src_mac} ${s.dst_mac} ${s.info} ${pkt.cable_id} ${pkt.from_node} ${pkt.to_node}`.toLowerCase();
      if (!hay.includes(clause)) return false;
    }
  }
  return true;
}

function formatHexAscii(bytes: number[]) {
  const lines: Array<{ offset: string; hex: string; ascii: string }> = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hexParts = chunk.map((b) => b.toString(16).padStart(2, '0'));
    while (hexParts.length < 16) hexParts.push('  ');
    const hex = `${hexParts.slice(0, 8).join(' ')}  ${hexParts.slice(8).join(' ')}`;
    const ascii = chunk
      .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.'))
      .join('');
    lines.push({
      offset: i.toString(16).padStart(4, '0'),
      hex,
      ascii,
    });
  }
  return lines;
}

export function PacketInspector() {
  const packets = useSimStore((s) => s.packets);
  const selectedNodeId = useSimStore((s) => s.selectedNodeId);
  const selectedEdgeId = useSimStore((s) => s.selectedEdgeId);
  const exportPcap = useSimStore((s) => s.exportPcap);
  const clearPackets = useSimStore((s) => s.clearPackets);

  const [open, setOpen] = useState(true);
  const [filterText, setFilterText] = useState('');
  const [scopeSelected, setScopeSelected] = useState(true);
  const [selectedSeq, setSelectedSeq] = useState<number | null>(null);
  const [expandedLayers, setExpandedLayers] = useState<Record<number, boolean>>({ 0: true, 1: true, 2: true });

  const filtered = useMemo(() => {
    return packets.filter((pkt) => {
      if (scopeSelected) {
        if (selectedEdgeId && pkt.cable_id !== selectedEdgeId) return false;
        if (
          selectedNodeId &&
          pkt.from_node !== selectedNodeId &&
          pkt.to_node !== selectedNodeId
        ) {
          return false;
        }
      }
      return matchesBpfFilter(pkt, filterText);
    });
  }, [packets, scopeSelected, selectedEdgeId, selectedNodeId, filterText]);

  const activePacket = useMemo(
    () => filtered.find((p) => p.seq === selectedSeq) ?? filtered[filtered.length - 1] ?? null,
    [filtered, selectedSeq]
  );

  const hexLines = useMemo(
    () => (activePacket ? formatHexAscii(activePacket.raw) : []),
    [activePacket]
  );

  return (
    <section className="flex flex-col border-t border-slate-800 bg-slate-900/95">
      {/* Drawer Header */}
      <div className="flex h-9 items-center justify-between px-3">
        <div className="flex items-center gap-3">
          <button
            onClick={() => setOpen(!open)}
            className="flex items-center gap-1.5 text-xs font-semibold text-slate-200 hover:text-sky-400"
          >
            {open ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
            <TerminalSquare className="h-3.5 w-3.5 text-sky-400" />
            <span>Packet Inspector</span>
            <span className="rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[10px] text-slate-300">
              {filtered.length} / {packets.length}
            </span>
          </button>

          {(selectedNodeId || selectedEdgeId) && (
            <label className="flex cursor-pointer items-center gap-1.5 rounded border border-slate-700 bg-slate-950 px-2 py-0.5 text-[11px] text-slate-300">
              <input
                type="checkbox"
                checked={scopeSelected}
                onChange={(e) => setScopeSelected(e.target.checked)}
                className="accent-sky-400"
              />
              Scope to {selectedEdgeId ? `link ${selectedEdgeId}` : `node ${selectedNodeId}`}
            </label>
          )}
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 rounded-md border border-slate-700 bg-slate-950 px-2 py-0.5">
            <Filter className="h-3 w-3 text-slate-500" />
            <input
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              placeholder="Filter: icmp, tcp.port == 80, ip == 10.0.0.2"
              className="w-64 bg-transparent font-mono text-[11px] text-slate-200 placeholder:text-slate-600 focus:outline-none"
            />
          </div>

          <button
            onClick={exportPcap}
            disabled={packets.length === 0}
            className="flex items-center gap-1 rounded-md border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] font-medium text-slate-200 hover:bg-slate-700 disabled:opacity-40"
          >
            <Download className="h-3 w-3 text-sky-400" />
            Export .pcap
          </button>

          <button
            onClick={clearPackets}
            disabled={packets.length === 0}
            title="Clear capture"
            className="flex items-center gap-1 rounded-md border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] text-slate-300 hover:bg-red-500/20 hover:text-red-300 disabled:opacity-40"
          >
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* Drawer Body */}
      {open && (
        <div className="grid h-64 grid-cols-12 border-t border-slate-800 bg-slate-950">
          {/* Packet Table */}
          <div className="col-span-7 overflow-y-auto border-r border-slate-800 font-mono text-[11px]">
            <table className="w-full border-collapse text-left">
              <thead className="sticky top-0 bg-slate-900 text-[10px] uppercase text-slate-400">
                <tr>
                  <th className="px-2 py-1">No.</th>
                  <th className="px-2 py-1">Time</th>
                  <th className="px-2 py-1">Path</th>
                  <th className="px-2 py-1">Source</th>
                  <th className="px-2 py-1">Destination</th>
                  <th className="px-2 py-1">Proto</th>
                  <th className="px-2 py-1">Len</th>
                  <th className="px-2 py-1">Info</th>
                </tr>
              </thead>
              <tbody>
                {filtered.length === 0 ? (
                  <tr>
                    <td colSpan={8} className="py-8 text-center text-slate-600">
                      No packets captured yet.
                    </td>
                  </tr>
                ) : (
                  filtered
                    .slice()
                    .reverse()
                    .slice(0, 300)
                    .map((pkt) => {
                      const isSel = activePacket?.seq === pkt.seq;
                      const badge =
                        PROTO_BADGE[pkt.summary.protocol] ??
                        'bg-slate-800 text-slate-300 border-slate-700';
                      return (
                        <tr
                          key={pkt.seq}
                          onClick={() => setSelectedSeq(pkt.seq)}
                          className={`cursor-pointer border-b border-slate-900/80 transition-colors ${
                            isSel ? 'bg-sky-500/15' : 'hover:bg-slate-900/70'
                          }`}
                        >
                          <td className="px-2 py-1 text-slate-500">{pkt.seq}</td>
                          <td className="px-2 py-1 text-slate-400">
                            {(pkt.timestamp_ms / 1000).toFixed(3)}
                          </td>
                          <td className="px-2 py-1 text-slate-400">
                            {pkt.from_node}→{pkt.to_node}
                          </td>
                          <td className="px-2 py-1 text-slate-200">
                            {pkt.summary.src_ip ?? pkt.summary.src_mac}
                          </td>
                          <td className="px-2 py-1 text-slate-200">
                            {pkt.summary.dst_ip ?? pkt.summary.dst_mac}
                          </td>
                          <td className="px-2 py-1">
                            <span className={`rounded border px-1 py-0.2 text-[10px] ${badge}`}>
                              {pkt.summary.protocol}
                            </span>
                          </td>
                          <td className="px-2 py-1 text-slate-400">{pkt.length}</td>
                          <td className="max-w-[260px] truncate px-2 py-1 text-slate-300">
                            {pkt.summary.info}
                          </td>
                        </tr>
                      );
                    })
                )}
              </tbody>
            </table>
          </div>

          {/* Protocol Tree + Hex Dump */}
          <div className="col-span-5 grid grid-rows-2 overflow-hidden">
            {/* Protocol Tree */}
            <div className="overflow-y-auto border-b border-slate-800 p-2 font-mono text-[11px]">
              {!activePacket ? (
                <div className="text-slate-600">Select a packet to inspect protocol layers.</div>
              ) : (
                <div className="space-y-1">
                  {activePacket.summary.layers.map((layer, idx) => {
                    const isExp = expandedLayers[idx] ?? true;
                    return (
                      <div key={idx}>
                        <button
                          onClick={() =>
                            setExpandedLayers((prev) => ({ ...prev, [idx]: !isExp }))
                          }
                          className="flex w-full items-center gap-1 text-left font-semibold text-sky-300 hover:text-sky-200"
                        >
                          {isExp ? (
                            <ChevronDown className="h-3 w-3 shrink-0" />
                          ) : (
                            <ChevronRight className="h-3 w-3 shrink-0" />
                          )}
                          <span className="truncate">{layer.name}</span>
                        </button>
                        {isExp && (
                          <div className="ml-4 mt-0.5 space-y-0.5 border-l border-slate-800 pl-2">
                            {layer.fields.map(([k, v], fi) => (
                              <div key={fi} className="flex gap-2 text-[10px]">
                                <span className="text-slate-500">{k}:</span>
                                <span className="break-all text-slate-200">{v}</span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Hex + ASCII Dump */}
            <div className="overflow-y-auto bg-slate-950 p-2 font-mono text-[10px] leading-relaxed">
              {!activePacket ? (
                <div className="text-slate-600">No frame selected.</div>
              ) : (
                hexLines.map((line) => (
                  <div key={line.offset} className="flex gap-3 whitespace-pre">
                    <span className="text-slate-600">{line.offset}</span>
                    <span className="text-sky-300/90">{line.hex}</span>
                    <span className="text-emerald-300/80">{line.ascii}</span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
