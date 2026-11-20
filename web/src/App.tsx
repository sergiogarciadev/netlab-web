import { useCallback, useEffect, useRef, type DragEvent } from 'react';
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type EdgeTypes,
  type NodeTypes,
} from '@xyflow/react';
import { HostNode } from './components/nodes/HostNode';
import { SwitchNode } from './components/nodes/SwitchNode';
import { RouterNode } from './components/nodes/RouterNode';
import { PacketEdge } from './components/edges/PacketEdge';
import { Toolbar } from './components/Toolbar';
import { Sidebar } from './components/Sidebar';
import { InspectorPanel } from './components/InspectorPanel';
import { PacketInspector } from './components/PacketInspector';
import { useSimStore } from './store/useSimStore';
import type { NodeKind } from './types/sim';

const nodeTypes: NodeTypes = {
  host: HostNode,
  switch: SwitchNode,
  router: RouterNode,
};

const edgeTypes: EdgeTypes = {
  packetEdge: PacketEdge,
};

function NetworkCanvas() {
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const { screenToFlowPosition } = useReactFlow();

  const nodes = useSimStore((s) => s.nodes);
  const edges = useSimStore((s) => s.edges);
  const onNodesChange = useSimStore((s) => s.onNodesChange);
  const onEdgesChange = useSimStore((s) => s.onEdgesChange);
  const onConnect = useSimStore((s) => s.onConnect);
  const addNode = useSimStore((s) => s.addNode);
  const selectNode = useSimStore((s) => s.selectNode);
  const selectEdge = useSimStore((s) => s.selectEdge);

  const onDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault();
      const kind = event.dataTransfer.getData('application/netlab-node') as NodeKind;
      if (!kind) return;
      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      addNode(kind, position);
    },
    [screenToFlowPosition, addNode]
  );

  return (
    <div ref={wrapperRef} className="relative flex-1 bg-slate-950">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onNodeClick={(_, node) => selectNode(node.id)}
        onEdgeClick={(_, edge) => selectEdge(edge.id)}
        onPaneClick={() => {
          selectNode(null);
          selectEdge(null);
        }}
        fitView
        colorMode="dark"
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#1e293b" gap={20} size={1} />
        <Controls />
        <MiniMap
          nodeColor={(n) =>
            n.type === 'host' ? '#38bdf8' : n.type === 'switch' ? '#34d399' : '#fbbf24'
          }
          maskColor="rgba(2, 6, 23, 0.75)"
        />
      </ReactFlow>
    </div>
  );
}

export default function App() {
  const initWorker = useSimStore((s) => s.initWorker);
  const lastError = useSimStore((s) => s.lastError);
  const clearError = useSimStore((s) => s.clearError);

  useEffect(() => {
    initWorker();
  }, [initWorker]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName?.toLowerCase();
      if (tag === 'input' || tag === 'textarea' || tag === 'select') return;

      const state = useSimStore.getState();
      if (!state.ready) return;

      if (e.code === 'Space') {
        e.preventDefault();
        state.setPlaying(!state.playing);
      } else if (e.key === 's' || e.key === 'S') {
        if (!state.playing) {
          e.preventDefault();
          state.stepOnce(10);
        }
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        if (state.selectedNodeId) {
          e.preventDefault();
          state.removeNode(state.selectedNodeId);
        } else if (state.selectedEdgeId) {
          e.preventDefault();
          state.removeEdge(state.selectedEdgeId);
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
      <Toolbar />
      {lastError && (
        <div className="flex items-center justify-between bg-red-500/20 px-4 py-1.5 text-xs text-red-300 border-b border-red-500/30">
          <span>{lastError}</span>
          <button onClick={clearError} className="underline hover:text-red-100">
            Dismiss
          </button>
        </div>
      )}
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <ReactFlowProvider>
            <NetworkCanvas />
          </ReactFlowProvider>
          <PacketInspector />
        </div>
        <InspectorPanel />
      </div>
    </div>
  );
}
