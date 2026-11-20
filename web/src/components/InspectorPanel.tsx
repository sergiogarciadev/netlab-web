import { useEffect, useState } from 'react';
import { Trash2, Send, Cable, X } from 'lucide-react';
import { useSimStore } from '../store/useSimStore';

export function InspectorPanel() {
  const selectedNodeId = useSimStore((s) => s.selectedNodeId);
  const selectedEdgeId = useSimStore((s) => s.selectedEdgeId);
  const nodes = useSimStore((s) => s.nodes);
  const edges = useSimStore((s) => s.edges);
  const hosts = useSimStore((s) => s.hosts);
  const switches = useSimStore((s) => s.switches);
  const routers = useSimStore((s) => s.routers);
  const cables = useSimStore((s) => s.cables);

  const configureInterface = useSimStore((s) => s.configureInterface);
  const updateLinkParams = useSimStore((s) => s.updateLinkParams);
  const sendPing = useSimStore((s) => s.sendPing);
  const removeNode = useSimStore((s) => s.removeNode);
  const removeEdge = useSimStore((s) => s.removeEdge);
  const selectNode = useSimStore((s) => s.selectNode);
  const selectEdge = useSimStore((s) => s.selectEdge);

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const selectedEdge = edges.find((e) => e.id === selectedEdgeId);

  const host = selectedNodeId ? hosts[selectedNodeId] : undefined;
  const sw = selectedNodeId ? switches[selectedNodeId] : undefined;
  const router = selectedNodeId ? routers[selectedNodeId] : undefined;

  const [ipInput, setIpInput] = useState('');
  const [prefixInput, setPrefixInput] = useState('24');
  const [gwInput, setGwInput] = useState('');
  const [pingTarget, setPingTarget] = useState('');

  const [routerIfaces, setRouterIfaces] = useState<Record<string, { ip: string; prefix: string }>>({});

  useEffect(() => {
    if (host) {
      setIpInput(host.ip ?? '');
      setPrefixInput(String(host.prefix_len ?? 24));
      setGwInput(host.gateway ?? '');
    }
  }, [host?.id, host?.ip, host?.prefix_len, host?.gateway]);

  useEffect(() => {
    if (router) {
      const next: Record<string, { ip: string; prefix: string }> = {};
      for (const iface of router.interfaces) {
        next[iface.id] = {
          ip: iface.ip ?? '',
          prefix: String(iface.prefix_len ?? 24),
        };
      }
      setRouterIfaces(next);
    }
  }, [router?.id, router?.interfaces.length]);

  if (!selectedNode && !selectedEdge) {
    return null;
  }

  return (
    <aside className="flex w-80 flex-col border-l border-slate-800 bg-slate-900/95 p-4 overflow-y-auto">
      {selectedNode && (
        <div className="space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <div>
              <div className="text-sm font-semibold text-slate-100">{selectedNode.data.label}</div>
              <div className="font-mono text-[11px] text-slate-400">
                {selectedNode.id} · {selectedNode.data.kind.toUpperCase()}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={() => removeNode(selectedNode.id)}
                title="Delete node"
                className="rounded p-1.5 text-slate-400 hover:bg-red-500/15 hover:text-red-400"
              >
                <Trash2 className="h-4 w-4" />
              </button>
              <button
                onClick={() => selectNode(null)}
                className="rounded p-1.5 text-slate-400 hover:bg-slate-800 hover:text-slate-200"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
          </div>

          {host && (
            <>
              <div className="space-y-2.5">
                <div className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Interface eth0 ({host.mac})
                </div>
                <div className="grid grid-cols-3 gap-2">
                  <div className="col-span-2">
                    <label className="mb-1 block text-[11px] text-slate-400">IPv4 Address</label>
                    <input
                      value={ipInput}
                      onChange={(e) => setIpInput(e.target.value)}
                      placeholder="10.0.0.1"
                      className="w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-[11px] text-slate-400">Prefix</label>
                    <input
                      value={prefixInput}
                      onChange={(e) => setPrefixInput(e.target.value)}
                      placeholder="24"
                      className="w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                    />
                  </div>
                </div>
                <div>
                  <label className="mb-1 block text-[11px] text-slate-400">Default Gateway</label>
                  <input
                    value={gwInput}
                    onChange={(e) => setGwInput(e.target.value)}
                    placeholder="10.0.0.254 (optional)"
                    className="w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                  />
                </div>
                <button
                  onClick={() =>
                    configureInterface(
                      host.id,
                      'eth0',
                      ipInput.trim() || null,
                      Number(prefixInput) || 24,
                      gwInput.trim() || null
                    )
                  }
                  className="w-full rounded-lg bg-sky-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-sky-500"
                >
                  Apply IP Configuration
                </button>
              </div>

              <div className="space-y-2.5 border-t border-slate-800 pt-3">
                <div className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  ICMP Ping
                </div>
                <div className="flex gap-2">
                  <input
                    value={pingTarget}
                    onChange={(e) => setPingTarget(e.target.value)}
                    placeholder="Target IP (e.g. 10.0.0.2)"
                    className="flex-1 rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                  />
                  <button
                    onClick={() => pingTarget.trim() && sendPing(host.id, pingTarget.trim())}
                    className="flex items-center gap-1 rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500"
                  >
                    <Send className="h-3.5 w-3.5" />
                    Ping
                  </button>
                </div>
                {host.ping_replies.length > 0 && (
                  <div className="max-h-36 space-y-1 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2 font-mono text-[11px]">
                    {host.ping_replies
                      .slice()
                      .reverse()
                      .slice(0, 10)
                      .map((r, i) => (
                        <div key={`${r.seq}-${i}`} className="text-emerald-400">
                          Reply from {r.from}: seq={r.seq} t={r.received_at_ms}ms
                        </div>
                      ))}
                  </div>
                )}
              </div>
            </>
          )}

          {sw && (
            <div className="space-y-2.5">
              <div className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                Learned MAC Table
              </div>
              {Object.keys(sw.mac_table).length === 0 ? (
                <div className="rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-500">
                  No MAC addresses learned yet.
                </div>
              ) : (
                <div className="space-y-1 rounded-lg border border-slate-800 bg-slate-950 p-2 font-mono text-[11px]">
                  {Object.entries(sw.mac_table).map(([mac, port]) => (
                    <div key={mac} className="flex justify-between text-slate-300">
                      <span>{mac}</span>
                      <span className="text-emerald-400">{port}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {router && (
            <div className="space-y-3">
              <div className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                Router Interfaces
              </div>
              {router.interfaces.length === 0 ? (
                <div className="rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-500">
                  Connect cables to create router interfaces.
                </div>
              ) : (
                router.interfaces.map((iface) => {
                  const draft = routerIfaces[iface.id] ?? {
                    ip: iface.ip ?? '',
                    prefix: String(iface.prefix_len ?? 24),
                  };
                  return (
                    <div
                      key={iface.id}
                      className="space-y-2 rounded-lg border border-slate-800 bg-slate-950 p-2.5"
                    >
                      <div className="flex justify-between font-mono text-[11px]">
                        <span className="font-semibold text-amber-400">{iface.id}</span>
                        <span className="text-slate-500">{iface.mac}</span>
                      </div>
                      <div className="flex gap-1.5">
                        <input
                          value={draft.ip}
                          onChange={(e) =>
                            setRouterIfaces((prev) => ({
                              ...prev,
                              [iface.id]: { ...draft, ip: e.target.value },
                            }))
                          }
                          placeholder="10.0.1.1"
                          className="flex-1 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                        />
                        <input
                          value={draft.prefix}
                          onChange={(e) =>
                            setRouterIfaces((prev) => ({
                              ...prev,
                              [iface.id]: { ...draft, prefix: e.target.value },
                            }))
                          }
                          className="w-12 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                        />
                        <button
                          onClick={() =>
                            configureInterface(
                              router.id,
                              iface.id,
                              draft.ip.trim() || null,
                              Number(draft.prefix) || 24,
                              null
                            )
                          }
                          className="rounded bg-amber-600 px-2 py-1 text-xs font-medium text-white hover:bg-amber-500"
                        >
                          Set
                        </button>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          )}
        </div>
      )}

      {selectedEdge && selectedEdge.data && (
        <div className="space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <div className="flex items-center gap-2">
              <Cable className="h-4 w-4 text-sky-400" />
              <div>
                <div className="text-sm font-semibold text-slate-100">Link {selectedEdge.id}</div>
                <div className="font-mono text-[11px] text-slate-400">
                  {selectedEdge.source}:{selectedEdge.data.portA} ↔ {selectedEdge.target}:
                  {selectedEdge.data.portB}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={() => removeEdge(selectedEdge.id)}
                title="Disconnect cable"
                className="rounded p-1.5 text-slate-400 hover:bg-red-500/15 hover:text-red-400"
              >
                <Trash2 className="h-4 w-4" />
              </button>
              <button
                onClick={() => selectEdge(null)}
                className="rounded p-1.5 text-slate-400 hover:bg-slate-800 hover:text-slate-200"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-slate-300">Link State</span>
              <button
                onClick={() =>
                  updateLinkParams(selectedEdge.id, {
                    ...selectedEdge.data!.params,
                    up: !selectedEdge.data!.params.up,
                  })
                }
                className={`rounded-full px-3 py-1 text-xs font-medium ${
                  selectedEdge.data.params.up
                    ? 'bg-emerald-500/20 text-emerald-300'
                    : 'bg-red-500/20 text-red-300'
                }`}
              >
                {selectedEdge.data.params.up ? 'UP' : 'DOWN'}
              </button>
            </div>

            <div>
              <div className="mb-1 flex justify-between text-xs">
                <span className="text-slate-400">Latency</span>
                <span className="font-mono text-slate-200">
                  {selectedEdge.data.params.latency_ms} ms
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={500}
                value={selectedEdge.data.params.latency_ms}
                onChange={(e) =>
                  updateLinkParams(selectedEdge.id, {
                    ...selectedEdge.data!.params,
                    latency_ms: Number(e.target.value),
                  })
                }
                className="w-full accent-sky-400"
              />
            </div>

            <div>
              <div className="mb-1 flex justify-between text-xs">
                <span className="text-slate-400">Jitter</span>
                <span className="font-mono text-slate-200">
                  {selectedEdge.data.params.jitter_ms} ms
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={200}
                value={selectedEdge.data.params.jitter_ms}
                onChange={(e) =>
                  updateLinkParams(selectedEdge.id, {
                    ...selectedEdge.data!.params,
                    jitter_ms: Number(e.target.value),
                  })
                }
                className="w-full accent-sky-400"
              />
            </div>

            <div>
              <div className="mb-1 flex justify-between text-xs">
                <span className="text-slate-400">Packet Loss</span>
                <span className="font-mono text-slate-200">
                  {selectedEdge.data.params.loss_percent}%
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={100}
                step={1}
                value={selectedEdge.data.params.loss_percent}
                onChange={(e) =>
                  updateLinkParams(selectedEdge.id, {
                    ...selectedEdge.data!.params,
                    loss_percent: Number(e.target.value),
                  })
                }
                className="w-full accent-amber-400"
              />
            </div>

            {cables[selectedEdge.id] && (
              <div className="rounded-lg border border-slate-800 bg-slate-950 p-2.5 font-mono text-[11px] text-slate-400">
                <div className="flex justify-between">
                  <span>Frames Sent</span>
                  <span className="text-slate-200">{cables[selectedEdge.id].packets_sent}</span>
                </div>
                <div className="flex justify-between">
                  <span>Frames Dropped</span>
                  <span className="text-red-400">{cables[selectedEdge.id].packets_dropped}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </aside>
  );
}
