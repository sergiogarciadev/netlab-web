import { useEffect, useState } from 'react';
import { Trash2, Send, Cable, X, Puzzle, Terminal, Sliders, Plus } from 'lucide-react';
import { useSimStore } from '../store/useSimStore';
import type { ConfigFieldSchema, PluginMetadata } from '../types/sim';

type HostTab = 'interfaces' | 'plugins' | 'console';

function SchemaForm({
  fields,
  initialValues,
  submitLabel,
  onSubmit,
}: {
  fields: ConfigFieldSchema[];
  initialValues?: Record<string, unknown>;
  submitLabel: string;
  onSubmit: (values: Record<string, unknown>) => void;
}) {
  const [values, setValues] = useState<Record<string, unknown>>(() => {
    const init: Record<string, unknown> = {};
    for (const f of fields) {
      init[f.key] = initialValues?.[f.key] ?? f.default_value ?? '';
    }
    return init;
  });

  useEffect(() => {
    const next: Record<string, unknown> = {};
    for (const f of fields) {
      next[f.key] = initialValues?.[f.key] ?? f.default_value ?? '';
    }
    setValues(next);
  }, [fields.map((f) => f.key).join(',')]);

  return (
    <div className="space-y-2">
      {fields.map((f) => (
        <div key={f.key}>
          <label className="mb-0.5 block text-[10px] text-slate-400">{f.label}</label>
          {f.field_type === 'textarea' ? (
            <textarea
              rows={3}
              value={String(values[f.key] ?? '')}
              onChange={(e) => setValues((prev) => ({ ...prev, [f.key]: e.target.value }))}
              className="w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-500 focus:outline-none"
            />
          ) : (
            <input
              type={f.field_type === 'number' ? 'number' : 'text'}
              value={String(values[f.key] ?? '')}
              onChange={(e) =>
                setValues((prev) => ({
                  ...prev,
                  [f.key]: f.field_type === 'number' ? Number(e.target.value) : e.target.value,
                }))
              }
              className="w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-500 focus:outline-none"
            />
          )}
        </div>
      ))}
      <button
        onClick={() => onSubmit(values)}
        className="w-full rounded bg-sky-600/90 px-2.5 py-1 text-[11px] font-medium text-white hover:bg-sky-500"
      >
        {submitLabel}
      </button>
    </div>
  );
}

export function InspectorPanel() {
  const selectedNodeId = useSimStore((s) => s.selectedNodeId);
  const selectedEdgeId = useSimStore((s) => s.selectedEdgeId);
  const nodes = useSimStore((s) => s.nodes);
  const edges = useSimStore((s) => s.edges);
  const hosts = useSimStore((s) => s.hosts);
  const switches = useSimStore((s) => s.switches);
  const routers = useSimStore((s) => s.routers);
  const cables = useSimStore((s) => s.cables);
  const pluginCatalog = useSimStore((s) => s.pluginCatalog);
  const consoleHistory = useSimStore((s) => s.consoleHistory);

  const configureInterface = useSimStore((s) => s.configureInterface);
  const setHostDns = useSimStore((s) => s.setHostDns);
  const updateLinkParams = useSimStore((s) => s.updateLinkParams);
  const sendPing = useSimStore((s) => s.sendPing);
  const attachPlugin = useSimStore((s) => s.attachPlugin);
  const detachPlugin = useSimStore((s) => s.detachPlugin);
  const runPluginCommand = useSimStore((s) => s.runPluginCommand);
  const execConsole = useSimStore((s) => s.execConsole);
  const addRouterRoute = useSimStore((s) => s.addRouterRoute);
  const removeRouterRoute = useSimStore((s) => s.removeRouterRoute);
  const removeNode = useSimStore((s) => s.removeNode);
  const removeEdge = useSimStore((s) => s.removeEdge);
  const selectNode = useSimStore((s) => s.selectNode);
  const selectEdge = useSimStore((s) => s.selectEdge);

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);
  const selectedEdge = edges.find((e) => e.id === selectedEdgeId);

  const host = selectedNodeId ? hosts[selectedNodeId] : undefined;
  const sw = selectedNodeId ? switches[selectedNodeId] : undefined;
  const router = selectedNodeId ? routers[selectedNodeId] : undefined;

  const [hostTab, setHostTab] = useState<HostTab>('interfaces');
  const [ipInput, setIpInput] = useState('');
  const [prefixInput, setPrefixInput] = useState('24');
  const [gwInput, setGwInput] = useState('');
  const [dnsInput, setDnsInput] = useState('');
  const [pingTarget, setPingTarget] = useState('');
  const [selectedPluginToAdd, setSelectedPluginToAdd] = useState('');
  const [consoleInput, setConsoleInput] = useState('');

  const [routeNet, setRouteNet] = useState('');
  const [routePrefix, setRoutePrefix] = useState('24');
  const [routeNextHop, setRouteNextHop] = useState('');
  const [routeOutIface, setRouteOutIface] = useState('eth0');

  const [routerIfaces, setRouterIfaces] = useState<Record<string, { ip: string; prefix: string }>>({});

  useEffect(() => {
    if (host) {
      setIpInput(host.ip ?? '');
      setPrefixInput(String(host.prefix_len ?? 24));
      setGwInput(host.gateway ?? '');
      setDnsInput(host.dns_servers?.[0] ?? '');
    }
  }, [host?.id, host?.ip, host?.prefix_len, host?.gateway, host?.dns_servers?.join(',')]);

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

  useEffect(() => {
    if (pluginCatalog.length > 0 && !selectedPluginToAdd) {
      setSelectedPluginToAdd(pluginCatalog[0].id);
    }
  }, [pluginCatalog, selectedPluginToAdd]);

  if (!selectedNode && !selectedEdge) {
    return null;
  }

  const metaById: Record<string, PluginMetadata> = {};
  for (const m of pluginCatalog) {
    metaById[m.id] = m;
  }

  return (
    <aside className="flex w-96 flex-col border-l border-slate-800 bg-slate-900/95 p-4 overflow-y-auto">
      {selectedNode && (
        <div className="flex flex-col space-y-4">
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
              {/* Host Inspector Tabs */}
              <div className="grid grid-cols-3 gap-1 rounded-lg border border-slate-800 bg-slate-950 p-1">
                <button
                  onClick={() => setHostTab('interfaces')}
                  className={`flex items-center justify-center gap-1 rounded-md py-1 text-[11px] font-medium transition-colors ${
                    hostTab === 'interfaces'
                      ? 'bg-slate-800 text-sky-300'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  <Sliders className="h-3 w-3" />
                  Interfaces
                </button>
                <button
                  onClick={() => setHostTab('plugins')}
                  className={`flex items-center justify-center gap-1 rounded-md py-1 text-[11px] font-medium transition-colors ${
                    hostTab === 'plugins'
                      ? 'bg-slate-800 text-sky-300'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  <Puzzle className="h-3 w-3" />
                  Plugins ({host.plugins.length})
                </button>
                <button
                  onClick={() => setHostTab('console')}
                  className={`flex items-center justify-center gap-1 rounded-md py-1 text-[11px] font-medium transition-colors ${
                    hostTab === 'console'
                      ? 'bg-slate-800 text-sky-300'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  <Terminal className="h-3 w-3" />
                  Console
                </button>
              </div>

              {hostTab === 'interfaces' && (
                <div className="space-y-4">
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
                    <div>
                      <label className="mb-1 block text-[11px] text-slate-400">DNS Server</label>
                      <input
                        value={dnsInput}
                        onChange={(e) => setDnsInput(e.target.value)}
                        placeholder="10.0.2.10 (optional)"
                        className="w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                      />
                    </div>
                    <button
                      onClick={() => {
                        configureInterface(
                          host.id,
                          'eth0',
                          ipInput.trim() || null,
                          Number(prefixInput) || 24,
                          gwInput.trim() || null
                        );
                        setHostDns(host.id, dnsInput.trim() || null);
                      }}
                      className="w-full rounded-lg bg-sky-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-sky-500"
                    >
                      Apply Interface Config
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
                </div>
              )}

              {hostTab === 'plugins' && (
                <div className="space-y-3">
                  <div className="flex gap-1.5">
                    <select
                      value={selectedPluginToAdd}
                      onChange={(e) => setSelectedPluginToAdd(e.target.value)}
                      className="flex-1 rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-200 focus:outline-none"
                    >
                      {pluginCatalog.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                    <button
                      onClick={() =>
                        selectedPluginToAdd && attachPlugin(host.id, selectedPluginToAdd)
                      }
                      className="flex items-center gap-1 rounded-lg bg-emerald-600 px-2.5 py-1.5 text-xs font-medium text-white hover:bg-emerald-500"
                    >
                      <Plus className="h-3.5 w-3.5" />
                      Add
                    </button>
                  </div>

                  {host.plugins.length === 0 ? (
                    <div className="rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-500">
                      No plugins installed. Add DHCP, DNS, HTTP, File, or Mail plugins above.
                    </div>
                  ) : (
                    <div className="space-y-3">
                      {host.plugins.map((inst) => {
                        const meta = metaById[inst.id];
                        return (
                          <div
                            key={inst.id}
                            className="space-y-2.5 rounded-xl border border-slate-800 bg-slate-950 p-3"
                          >
                            <div className="flex items-center justify-between">
                              <div>
                                <div className="text-xs font-semibold text-sky-300">
                                  {inst.name}
                                </div>
                                <div className="text-[10px] text-slate-500">
                                  {meta?.description}
                                </div>
                              </div>
                              <button
                                onClick={() => detachPlugin(host.id, inst.id)}
                                title="Remove plugin"
                                className="rounded p-1 text-slate-500 hover:bg-red-500/15 hover:text-red-400"
                              >
                                <Trash2 className="h-3.5 w-3.5" />
                              </button>
                            </div>

                            {meta && meta.config_schema.length > 0 && (
                              <div className="border-t border-slate-800/80 pt-2">
                                <SchemaForm
                                  fields={meta.config_schema}
                                  initialValues={inst.state}
                                  submitLabel="Save Config"
                                  onSubmit={(vals) =>
                                    runPluginCommand(host.id, inst.id, {
                                      command: 'configure',
                                      config: vals,
                                    })
                                  }
                                />
                              </div>
                            )}

                            {meta &&
                              meta.actions.map((act) => (
                                <div
                                  key={act.command}
                                  className="border-t border-slate-800/80 pt-2"
                                >
                                  <SchemaForm
                                    fields={act.fields}
                                    submitLabel={act.label}
                                    onSubmit={(vals) =>
                                      runPluginCommand(host.id, inst.id, {
                                        command: act.command,
                                        ...vals,
                                      })
                                    }
                                  />
                                </div>
                              ))}

                            <div className="border-t border-slate-800/80 pt-2">
                              <div className="mb-1 text-[10px] font-semibold uppercase text-slate-500">
                                Live State
                              </div>
                              <pre className="max-h-28 overflow-auto rounded bg-slate-900 p-1.5 font-mono text-[10px] text-slate-300">
                                {JSON.stringify(inst.state, null, 2)}
                              </pre>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}

              {hostTab === 'console' && (
                <div className="flex flex-col space-y-2">
                  <div className="h-64 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2.5 font-mono text-[11px] leading-relaxed">
                    <div className="mb-1 text-slate-500">
                      Type &apos;help&apos; for commands (ip addr, ping, dig, curl, ls, cat).
                    </div>
                    {(consoleHistory[host.id] ?? []).map((line, i) => (
                      <div key={`cmd-${i}`} className="whitespace-pre-wrap text-sky-300">
                        {line}
                      </div>
                    ))}
                    {host.console_logs.map((log, i) => (
                      <div key={`log-${i}`} className="text-emerald-400/90">
                        {log}
                      </div>
                    ))}
                  </div>
                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      if (!consoleInput.trim()) return;
                      execConsole(host.id, consoleInput.trim());
                      setConsoleInput('');
                    }}
                    className="flex gap-1.5"
                  >
                    <input
                      value={consoleInput}
                      onChange={(e) => setConsoleInput(e.target.value)}
                      placeholder="e.g. dig web.local"
                      className="flex-1 rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 focus:border-sky-500 focus:outline-none"
                    />
                    <button
                      type="submit"
                      className="rounded-lg bg-sky-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-sky-500"
                    >
                      Run
                    </button>
                  </form>
                </div>
              )}
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

              <div className="space-y-2 border-t border-slate-800 pt-3">
                <div className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Routing Rules (Static Routes)
                </div>
                <div className="text-[10px] text-slate-500">
                  Connected subnets are routed automatically. Add static routes for remote networks.
                </div>

                {router.routes.length > 0 && (
                  <div className="space-y-1 rounded-lg border border-slate-800 bg-slate-950 p-2 font-mono text-[11px]">
                    {router.routes.map((r, idx) => (
                      <div key={idx} className="flex items-center justify-between text-slate-300">
                        <span>
                          {r.network}/{r.prefix_len} → {r.next_hop ?? 'direct'} ({r.out_iface})
                        </span>
                        <button
                          onClick={() => removeRouterRoute(router.id, idx)}
                          className="rounded p-0.5 text-slate-500 hover:text-red-400"
                          title="Delete route"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}

                <div className="space-y-1.5 rounded-lg border border-slate-800 bg-slate-950 p-2.5">
                  <div className="grid grid-cols-3 gap-1.5">
                    <input
                      value={routeNet}
                      onChange={(e) => setRouteNet(e.target.value)}
                      placeholder="10.0.3.0"
                      className="col-span-2 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                    />
                    <input
                      value={routePrefix}
                      onChange={(e) => setRoutePrefix(e.target.value)}
                      placeholder="24"
                      className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-1.5">
                    <input
                      value={routeNextHop}
                      onChange={(e) => setRouteNextHop(e.target.value)}
                      placeholder="Next hop IP (opt)"
                      className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                    />
                    <select
                      value={routeOutIface}
                      onChange={(e) => setRouteOutIface(e.target.value)}
                      className="rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-xs text-slate-100"
                    >
                      {router.interfaces.map((i) => (
                        <option key={i.id} value={i.id}>
                          {i.id}
                        </option>
                      ))}
                      {router.interfaces.length === 0 && <option value="eth0">eth0</option>}
                    </select>
                  </div>
                  <button
                    onClick={() => {
                      if (!routeNet.trim()) return;
                      addRouterRoute(
                        router.id,
                        routeNet.trim(),
                        Number(routePrefix) || 24,
                        routeNextHop.trim() || null,
                        routeOutIface
                      );
                      setRouteNet('');
                      setRouteNextHop('');
                    }}
                    className="w-full rounded bg-amber-600 px-2.5 py-1 text-xs font-medium text-white hover:bg-amber-500"
                  >
                    Add Route Rule
                  </button>
                </div>
              </div>
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
