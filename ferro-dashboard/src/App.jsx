import { useState, useEffect, useRef } from 'react';
import { 
  Activity, MessageSquare, Terminal, 
  TrendingUp, Eye, Ear, Send, Shield 
} from 'lucide-react';
import { 
  ResponsiveContainer, LineChart, Line, XAxis, YAxis, Tooltip, CartesianGrid 
} from 'recharts';

const API_BASE = "http://localhost:18080";

export default function App() {
  const [status, setStatus] = useState(() => ({
    fep: 0.0,
    phase: "Wake",
    cpu_temp: 42.0,
    ram_free: 8_000_000_000,
    disk_io: 0.0,
    process_error: 0,
    pain_count: 0,
    cluster_count: 5,
    complexity_level: 0.5,
    timestamp: Date.now(),
    uptime: 0
  }));

  const formatUptime = (seconds) => {
    if (seconds === undefined || seconds === null) return "00:00:00";
    const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
    const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
    const s = Math.floor(seconds % 60).toString().padStart(2, '0');
    return `${h}:${m}:${s}`;
  };

  const [sensory, setSensory] = useState({
    visual: { frame_delta: 0.02, image_embedding: [0.1, 0.2, 0.3, 0.4, 0.5] },
    auditory: { speech_tokens: ["listen"], mfcc: [0.1, 0.12, 0.15, 0.11, 0.09] },
    dev_log: { increment: "INFO: System initialized." }
  });

  const [chatList, setChatList] = useState([]);
  const [inputText, setInputText] = useState("");
  const [logFeed, setLogFeed] = useState(["[Terminal] Estalishing connection to API..."]);
  const [fepHistory, setFepHistory] = useState([]);
  const [isOnline, setIsOnline] = useState(false);

  const canvasRef = useRef(null);
  const chatEndRef = useRef(null);
  const eventSourceRef = useRef(null);
  const nodePhysicsRef = useRef({ nodes: [], links: [] });

  // Define helper functions before useEffect to prevent Temporal Dead Zone issues
  const addLog = (msg) => {
    if (!msg || msg.trim() === "") return;
    setLogFeed((prev) => {
      const next = [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`];
      if (next.length > 50) next.shift();
      return next;
    });
  };

  const fetchFepHistory = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/fep/history`);
      if (res.ok) {
        const data = await res.json();
        const formatted = data.map(r => ({
          time: new Date(r.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
          fep: parseFloat(r.fep.toFixed(4)),
          surprise: 0.0
        }));
        setFepHistory(formatted);
      }
    } catch (e) {
      console.error("Error fetching FEP history", e);
    }
  };

  const fetchChatHistory = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/chat/history`);
      if (res.ok) {
        const data = await res.json();
        setChatList(data);
      }
    } catch (e) {
      console.error("Error fetching chat", e);
    }
  };

  // 1. Establish SSE / Fallback Polling connection
  useEffect(() => {
    // Run initial history fetch in a microtask to avoid calling setState synchronously in the effect setup phase
    Promise.resolve().then(() => {
      fetchFepHistory();
      fetchChatHistory();
    });

    const connectSSE = () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }

      const es = new EventSource(`${API_BASE}/api/stream`);
      eventSourceRef.current = es;

      es.onopen = () => {
        setIsOnline(true);
        addLog("SSE Connection established.");
      };

      es.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          setStatus(data.status);
          setSensory(data.sensory);

          // Append new terminal logs
          if (data.sensory?.dev_log?.increment) {
            addLog(data.sensory.dev_log.increment);
          }

          // Realtime FEP Chart tracking
          setFepHistory((prev) => {
            const next = [...prev, {
              time: new Date(data.status.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
              fep: parseFloat(data.status.fep.toFixed(4)),
              surprise: parseFloat(data.sensory.visual.frame_delta.toFixed(4))
            }];
            if (next.length > 30) next.shift();
            return next;
          });
        } catch (err) {
          console.error("SSE parse error", err);
        }
      };

      es.onerror = () => {
        setIsOnline(false);
        addLog("SSE disconnected. Falling back to HTTP polling...");
        es.close();
        // Fallback polling loop (1.5s interval)
        const interval = setInterval(async () => {
          try {
            const sRes = await fetch(`${API_BASE}/api/status`);
            const sensRes = await fetch(`${API_BASE}/api/sensory`);
            if (sRes.ok && sensRes.ok) {
              const sData = await sRes.json();
              const sensData = await sensRes.json();
              setStatus(sData);
              setSensory(sensData);
              setIsOnline(true);
            }
          } catch {
            setIsOnline(false);
          }
        }, 1500);

        return () => clearInterval(interval);
      };
    };

    connectSSE();

    return () => {
      if (eventSourceRef.current) eventSourceRef.current.close();
    };
  }, []);

  // 2. Chat sending handler
  const handleSendMessage = async (e) => {
    e.preventDefault();
    if (!inputText.trim()) return;

    const textToSend = inputText;
    setInputText("");

    try {
      const res = await fetch(`${API_BASE}/api/chat/talk`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: textToSend })
      });
      if (res.ok) {
        fetchChatHistory();
      }
    } catch {
      addLog("Failed to send message to core.");
    }
  };

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatList]);

  // 3. Dynamic HTML5 Force-Directed Graph simulation for clusters
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    let animationId;

    const resizeCanvas = () => {
      canvas.width = canvas.parentElement.clientWidth;
      canvas.height = canvas.parentElement.clientHeight || 280;
    };
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);

    // Initialize or adjust node list based on status.cluster_count
    const currentCount = status.cluster_count || 5;
    let phys = nodePhysicsRef.current;

    // Adjust nodes length to match cluster count
    if (phys.nodes.length !== currentCount) {
      if (phys.nodes.length < currentCount) {
        // Spawn mitosis children near parents
        while (phys.nodes.length < currentCount) {
          const parentIndex = phys.nodes.length > 0 ? Math.floor(Math.random() * phys.nodes.length) : -1;
          const parent = parentIndex >= 0 ? phys.nodes[parentIndex] : null;
          const px = parent ? parent.x + (Math.random() - 0.5) * 40 : canvas.width / 2;
          const py = parent ? parent.y + (Math.random() - 0.5) * 40 : canvas.height / 2;
          
          const newNode = {
            id: `cortex_cluster_${phys.nodes.length}`,
            x: px,
            y: py,
            vx: 0,
            vy: 0,
            radius: Math.random() * 6 + 8,
            activation: Math.random(),
            pulseRate: 0.05 + Math.random() * 0.05,
            pulseTime: Math.random() * 10
          };
          phys.nodes.push(newNode);

          if (parent) {
            phys.links.push({ source: parent.id, target: newNode.id, weight: 0.95 });
          }
        }
      } else {
        // Shrink (Pruning)
        phys.nodes = phys.nodes.slice(0, currentCount);
        phys.links = phys.links.filter(l => 
          phys.nodes.some(n => n.id === l.source) && phys.nodes.some(n => n.id === l.target)
        );
      }
    }

    // Main animation & physics loop
    const updatePhysics = () => {
      const kRepulsion = 180;
      const kGravity = 0.03;
      const kLink = 0.08;
      const linkLength = 80;

      // Apply forces
      for (let i = 0; i < phys.nodes.length; i++) {
        const n1 = phys.nodes[i];
        
        // Gravity toward center
        n1.vx += (canvas.width / 2 - n1.x) * kGravity;
        n1.vy += (canvas.height / 2 - n1.y) * kGravity;

        // Repulsion between nodes
        for (let j = i + 1; j < phys.nodes.length; j++) {
          const n2 = phys.nodes[j];
          const dx = n2.x - n1.x;
          const dy = n2.y - n1.y;
          const dist = Math.hypot(dx, dy) || 1;
          if (dist < 220) {
            const force = kRepulsion / (dist * dist);
            n1.vx -= (dx / dist) * force;
            n1.vy -= (dy / dist) * force;
            n2.vx += (dx / dist) * force;
            n2.vy += (dy / dist) * force;
          }
        }
      }

      // Link spring forces
      phys.links.forEach(l => {
        const sourceNode = phys.nodes.find(n => n.id === l.source);
        const targetNode = phys.nodes.find(n => n.id === l.target);
        if (sourceNode && targetNode) {
          const dx = targetNode.x - sourceNode.x;
          const dy = targetNode.y - sourceNode.y;
          const dist = Math.hypot(dx, dy) || 1;
          const delta = dist - linkLength;
          const force = delta * kLink * l.weight;
          sourceNode.vx += (dx / dist) * force;
          sourceNode.vy += (dy / dist) * force;
          targetNode.vx -= (dx / dist) * force;
          targetNode.vy -= (dy / dist) * force;
        }
      });

      // Update positions and velocities with friction
      phys.nodes.forEach(n => {
        n.x += n.vx;
        n.y += n.vy;
        n.vx *= 0.85;
        n.vy *= 0.85;

        // Boundaries
        n.x = Math.max(n.radius, Math.min(canvas.width - n.radius, n.x));
        n.y = Math.max(n.radius, Math.min(canvas.height - n.radius, n.y));
        
        // Dynamic pulse
        n.pulseTime += n.pulseRate;
      });
    };

    const draw = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // Draw links
      phys.links.forEach(l => {
        const sourceNode = phys.nodes.find(n => n.id === l.source);
        const targetNode = phys.nodes.find(n => n.id === l.target);
        if (sourceNode && targetNode) {
          ctx.beginPath();
          ctx.moveTo(sourceNode.x, sourceNode.y);
          ctx.lineTo(targetNode.x, targetNode.y);
          ctx.strokeStyle = status.phase === "Sleep" 
            ? `rgba(231, 122, 96, ${0.1 + l.weight * 0.25})` 
            : `rgba(59, 158, 153, ${0.1 + l.weight * 0.25})`;
          ctx.lineWidth = l.weight * 1.5;
          ctx.stroke();
        }
      });

      // Draw nodes
      phys.nodes.forEach(n => {
        const glow = Math.abs(Math.sin(n.pulseTime)) * 6 + 3;
        
        // Node outer glow ring
        ctx.beginPath();
        ctx.arc(n.x, n.y, n.radius + glow / 2, 0, Math.PI * 2);
        ctx.fillStyle = status.phase === "Sleep" 
          ? `rgba(231, 122, 96, ${0.12 + (n.activation * 0.12)})`
          : `rgba(59, 158, 153, ${0.12 + (n.activation * 0.12)})`;
        ctx.fill();

        // Node core
        ctx.beginPath();
        ctx.arc(n.x, n.y, n.radius, 0, Math.PI * 2);
        ctx.fillStyle = status.phase === "Sleep" ? '#E77A60' : '#3B9E99';
        ctx.fill();
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 1.5;
        ctx.stroke();

        // Optional label
        ctx.fillStyle = '#7A756C';
        ctx.font = '8px Fira Code';
        ctx.fillText(n.id.replace('cortex_cluster_', 'C-'), n.x + n.radius + 4, n.y + 3);
      });
    };

    const tickAnimation = () => {
      updatePhysics();
      draw();
      animationId = requestAnimationFrame(tickAnimation);
    };
    tickAnimation();

    return () => {
      window.removeEventListener('resize', resizeCanvas);
      cancelAnimationFrame(animationId);
    };
  }, [status.cluster_count, status.phase]);

  // Calculations for gauges
  const maxRam = 16_000_000_000;
  const ramPercent = Math.min(100, Math.max(0, 100 - (status.ram_free / maxRam) * 100));
  const tempPercent = Math.min(100, Math.max(0, (status.cpu_temp / 100) * 100));

  return (
    <div className="min-h-screen flex flex-col p-6 gap-6">
      
      {/* 1. Header Area styled as an Elegant Style Tile header */}
      <header className="glass-panel p-5 flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
        <div className="flex items-center gap-4">
          <div className="h-10 w-10 rounded-xl bg-gradient-to-tr from-[#3B9E99] to-[#E77A60] flex items-center justify-center shadow-md">
            <Activity className="h-5 w-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold bg-gradient-to-r from-[#3B9E99] to-[#E77A60] bg-clip-text text-transparent">
              FERRO // Active Inference Board
            </h1>
            <p className="text-xs text-secondary font-mono tracking-wider">
              STYLE TILE // COGNITIVE HIERARCHY SIMULATOR
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-4">
          {/* Style Tile Palette swatches */}
          <div className="flex gap-1.5 items-center bg-[#FCFAF7] border border-[#E6E3DB] px-3 py-1.5 rounded-full">
            <span className="text-[9px] font-mono text-[#7A756C] mr-1">SWATCHES:</span>
            <div className="h-3.5 w-3.5 rounded-full bg-[#3B9E99] border border-white/20 shadow-sm" title="Teal (Active)" />
            <div className="h-3.5 w-3.5 rounded-full bg-[#E77A60] border border-white/20 shadow-sm" title="Coral (Surprise)" />
            <div className="h-3.5 w-3.5 rounded-full bg-[#63AB8F] border border-white/20 shadow-sm" title="Green (Online)" />
            <div className="h-3.5 w-3.5 rounded-full bg-[#D9A74A] border border-white/20 shadow-sm" title="Amber (ZPD)" />
            <div className="h-3.5 w-3.5 rounded-full bg-[#D15E4C] border border-white/20 shadow-sm" title="Red (Offline)" />
            <div className="h-3.5 w-3.5 rounded-full bg-[#2D2C2A] border border-white/20 shadow-sm" title="Charcoal (Text)" />
          </div>

          <div className="flex items-center gap-2 font-mono text-xs">
            <span className="text-[#7A756C]">STATE:</span>
            <span className={`neon-badge ${isOnline ? 'neon-green' : 'neon-red'}`}>
              <span className={`h-1.5 w-1.5 rounded-full ${isOnline ? 'bg-[#63AB8F]' : 'bg-[#D15E4C]'}`}></span>
              {isOnline ? 'ONLINE' : 'OFFLINE'}
            </span>
          </div>

          <div className="flex items-center gap-2 font-mono text-xs">
            <span className="text-[#7A756C]">PHASE:</span>
            <span className={`neon-badge ${status.phase === 'Sleep' ? 'neon-purple' : 'neon-cyan'}`}>
              {status.phase}
            </span>
          </div>

          <div className="flex items-center gap-2 font-mono text-xs">
            <span className="text-[#7A756C]">UPTIME:</span>
            <span className="neon-badge neon-amber">
              {formatUptime(status.uptime)}
            </span>
          </div>
        </div>
      </header>

      {/* Main Grid Layout */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 gap-4">
        
        {/* Left column: Metrics, charts, equalizers (7/12) */}
        <div className="lg:col-span-7 flex flex-col gap-4">
          
          {/* Ring Gauges */}
          <div className="grid grid-cols-3 gap-4">
            <div className="glass-panel p-4 flex flex-col items-center justify-center relative min-h-[140px]">
              <svg className="w-20 h-20 transform -rotate-90">
                <circle cx="40" cy="40" r="34" stroke="#E6E3DB" strokeWidth="4" fill="transparent" />
                <circle cx="40" cy="40" r="34" stroke="#E77A60" strokeWidth="4" fill="transparent" 
                        strokeDasharray={213.6} strokeDashoffset={213.6 - (213.6 * tempPercent) / 100} strokeLinecap="round" />
              </svg>
              <div className="absolute flex flex-col items-center justify-center">
                <span className="text-lg font-bold font-mono text-[#2D2C2A]">{status.cpu_temp.toFixed(1)}°C</span>
                <span className="text-[9px] text-[#7A756C] font-mono uppercase tracking-wider mt-1">CPU TEMP</span>
              </div>
            </div>

            <div className="glass-panel p-4 flex flex-col items-center justify-center relative min-h-[140px]">
              <svg className="w-20 h-20 transform -rotate-90">
                <circle cx="40" cy="40" r="34" stroke="#E6E3DB" strokeWidth="4" fill="transparent" />
                <circle cx="40" cy="40" r="34" stroke="#3B9E99" strokeWidth="4" fill="transparent" 
                        strokeDasharray={213.6} strokeDashoffset={213.6 - (213.6 * ramPercent) / 100} strokeLinecap="round" />
              </svg>
              <div className="absolute flex flex-col items-center justify-center">
                <span className="text-lg font-bold font-mono text-[#2D2C2A]">{ramPercent.toFixed(0)}%</span>
                <span className="text-[9px] text-[#7A756C] font-mono uppercase tracking-wider mt-1">RAM USE</span>
              </div>
            </div>

            <div className="glass-panel p-4 flex flex-col items-center justify-center relative min-h-[140px]">
              <svg className="w-20 h-20 transform -rotate-90">
                <circle cx="40" cy="40" r="34" stroke="#E6E3DB" strokeWidth="4" fill="transparent" />
                <circle cx="40" cy="40" r="34" stroke="#D9A74A" strokeWidth="4" fill="transparent" 
                        strokeDasharray={213.6} strokeDashoffset={213.6 - (213.6 * (status.complexity_level * 100)) / 100} strokeLinecap="round" />
              </svg>
              <div className="absolute flex flex-col items-center justify-center">
                <span className="text-lg font-bold font-mono text-[#2D2C2A]">{ (status.complexity_level * 100).toFixed(0) }%</span>
                <span className="text-[9px] text-[#7A756C] font-mono uppercase tracking-wider mt-1">ZPD COMP</span>
              </div>
            </div>
          </div>

          {/* FEP Chart */}
          <div className="glass-panel p-5 flex-1 flex flex-col min-h-[220px]">
            <div className="flex justify-between items-center mb-3">
              <h2 className="text-xs font-semibold text-[#2D2C2A] font-mono flex items-center gap-2">
                <TrendingUp className="h-4 w-4 text-[#3B9E99]" />
                Global Free Energy Trend (FEP)
              </h2>
              <span className="text-xs font-mono text-[#7A756C]">FEP: {status.fep.toFixed(4)}</span>
            </div>
            <div className="w-full flex-1 min-h-[180px]">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={fepHistory} margin={{ top: 10, right: -10, left: -10, bottom: 0 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(230, 227, 219, 0.5)" vertical={false} />
                  <XAxis dataKey="time" stroke="#9C978E" style={{ fontSize: '9px', fontFamily: 'monospace' }} tickLine={false} axisLine={false} />
                  <YAxis yAxisId="left" stroke="#3B9E99" orientation="left" style={{ fontSize: '9px', fontFamily: 'monospace' }} domain={[0, 1.0]} allowDataOverflow={true} tickLine={false} axisLine={false} />
                  <YAxis yAxisId="right" stroke="#E77A60" orientation="right" style={{ fontSize: '9px', fontFamily: 'monospace' }} domain={[0, 1.0]} allowDataOverflow={true} tickLine={false} axisLine={false} />
                  <Tooltip contentStyle={{ background: '#FFFFFF', border: '1px solid #E6E3DB', borderRadius: '8px', fontSize: '11px', fontFamily: 'monospace', color: '#2D2C2A' }} />
                  <Line yAxisId="left" type="monotone" name="Free Energy" dataKey="fep" stroke="#3B9E99" strokeWidth={2.5} dot={false} activeDot={{ r: 4 }} />
                  <Line yAxisId="right" type="monotone" name="Surprise" dataKey="surprise" stroke="#E77A60" strokeWidth={1.5} strokeDasharray="3 3" dot={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Sensory Modality Equalizers */}
          <div className="grid grid-cols-2 gap-4">
            {/* Visual Delta Monitor */}
            <div className="glass-panel p-4 flex flex-col gap-3">
              <div className="flex justify-between items-center">
                <h3 className="text-xs font-semibold text-[#7A756C] font-mono flex items-center gap-2">
                  <Eye className="h-4 w-4 text-[#3B9E99]" /> Visual Ingestion
                </h3>
                <span className="text-xs font-mono text-[#3B9E99]">Δ {sensory.visual.frame_delta.toFixed(4)}</span>
              </div>
              <div className="flex flex-col gap-2 p-2 bg-[#FCFAF7] border border-[#E6E3DB] rounded-lg">
                <div className="relative h-24 w-full rounded bg-white border border-[#E6E3DB] flex flex-col items-center justify-center overflow-hidden">
                  <div className="absolute h-16 w-16 rounded-full border border-dashed border-[#3B9E99]/30 animate-spin" style={{ animationDuration: '8s' }} />
                  <div className="absolute h-10 w-10 rounded-full border border-[#3B9E99]/20 animate-ping" style={{ animationDuration: `${Math.max(0.4, 2.0 - sensory.visual.frame_delta * 10)}s` }} />
                  <div className="absolute h-2.5 w-2.5 rounded-full bg-[#3B9E99] shadow-[0_0_8px_rgba(59,158,153,0.5)]" />
                  <span className="text-[8px] text-[#7A756C] font-mono mt-10 uppercase tracking-widest">SENSORY SIGNAL FEED</span>
                </div>
                
                <div className="flex gap-3 items-center mt-1">
                  <div className="flex-1 flex flex-col justify-center">
                    <span className="text-[9px] text-[#7A756C] font-mono">EMBEDDING VECTOR REPRESENTATION</span>
                    <div className="flex gap-1.5 mt-1.5">
                      {sensory.visual.image_embedding.slice(0, 5).map((v, i) => (
                        <div key={i} className="flex-1 h-2.5 bg-[#E6E3DB] rounded-sm relative overflow-hidden">
                          <div className="absolute top-0 bottom-0 left-0 bg-[#3B9E99]" style={{ width: `${Math.min(100, Math.max(0, (v + 1) * 50))}%` }} />
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Auditory Equalizer */}
            <div className="glass-panel p-4 flex flex-col gap-3">
              <div className="flex justify-between items-center">
                <h3 className="text-xs font-semibold text-[#7A756C] font-mono flex items-center gap-2">
                  <Ear className="h-4 w-4 text-[#E77A60]" /> Auditory Speech
                </h3>
                <span className="text-[10px] font-mono text-[#E77A60] truncate max-w-[80px]">
                  {sensory.auditory.speech_tokens.join(", ") || "none"}
                </span>
              </div>
              <div className="flex items-center gap-3 h-12 px-3 bg-[#FCFAF7] border border-[#E6E3DB] rounded-lg">
                <div className="flex items-end gap-1.5 h-7">
                  {sensory.auditory.mfcc.slice(0, 5).map((v, i) => {
                    const hScale = Math.min(1.0, Math.max(0.1, Math.abs(v)));
                    return (
                      <div 
                        key={i} 
                        className={`w-1.5 bg-[#E77A60] rounded-full eq-bar eq-bar-${i+1}`} 
                        style={{ height: '100%', transform: `scaleY(${hScale})` }} 
                      />
                    );
                  })}
                </div>
                <div className="flex-1 flex flex-col min-w-0">
                  <span className="text-[9px] text-[#7A756C] font-mono uppercase tracking-wider">MFCC ENERGY</span>
                  <span className="text-xs font-mono text-[#2D2C2A] font-semibold truncate">
                    {sensory.auditory.mfcc.map(v => v.toFixed(2)).join(" | ")}
                  </span>
                </div>
              </div>
            </div>
          </div>
          
        </div>

        {/* Right column: Force-directed topology graph, Chat interface (5/12) */}
        <div className="lg:col-span-5 flex flex-col gap-4">
          
          {/* Cortex cluster canvas graph */}
          <div className="glass-panel p-4 flex flex-col h-[280px]">
            <div className="flex justify-between items-center mb-2">
              <h2 className="text-xs font-semibold text-[#2D2C2A] font-mono flex items-center gap-2">
                <Shield className="h-4 w-4 text-[#3B9E99]" />
                Cortex Network Topology (Mitosis)
              </h2>
              <span className="text-[10px] font-mono text-[#7A756C]">CLUSTERS: {status.cluster_count}</span>
            </div>
            <div className="flex-1 relative overflow-hidden bg-[#FCFAF7] border border-[#E6E3DB] rounded-lg">
              <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
            </div>
          </div>

          {/* Interactive Chat Panel */}
          <div className="glass-panel p-4 flex-1 flex flex-col min-h-[300px]">
            <div className="flex justify-between items-center mb-2 border-b border-[#E6E3DB] pb-2">
              <h2 className="text-xs font-semibold text-[#2D2C2A] font-mono flex items-center gap-2">
                <MessageSquare className="h-4 w-4 text-[#3B9E99]" />
                Neural Conversation Feed
              </h2>
              <span className="text-[10px] font-mono text-[#7A756C]">PAIN COUNTER: {status.pain_count}</span>
            </div>

            {/* Chat list viewport */}
            <div className="flex-1 overflow-y-auto flex flex-col gap-3 p-2 my-2 bg-[#FCFAF7] border border-[#E6E3DB] rounded-lg max-h-[320px]">
              {chatList.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-xs text-muted font-mono italic">
                  No signals detected. Start talking to wake up the core...
                </div>
              ) : (
                chatList.map((msg, i) => (
                  <div key={i} className={`flex flex-col max-w-[85%] shrink-0 ${msg.sender === 'user' ? 'self-end items-end' : 'self-start'}`}>
                    <span className="text-[9px] text-[#7A756C] font-semibold font-mono mb-0.5">
                      {msg.sender === 'user' ? 'USER' : `CORE // ${msg.origin.toUpperCase()}`}
                    </span>
                    <div className={`p-2.5 rounded-lg text-sm font-mono leading-relaxed ${
                      msg.sender === 'user' 
                        ? 'bg-[#3B9E99]/6 border border-[#3B9E99]/20 text-[#2D2C2A] rounded-tr-none' 
                        : 'bg-[#E77A60]/6 border border-[#E77A60]/20 text-[#2D2C2A] rounded-tl-none'
                    }`}>
                      {msg.text}
                    </div>
                  </div>
                ))
              )}
              <div ref={chatEndRef} />
            </div>

            {/* Input field */}
            <form onSubmit={handleSendMessage} className="flex gap-2">
              <input 
                type="text" 
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                placeholder="Send speech tokens (e.g. hello, query status)..."
                className="flex-1 px-3 py-2 rounded text-sm focus:outline-none focus:border-[#3B9E99] placeholder-muted"
              />
              <button type="submit" className="p-2 bg-[#3B9E99]/10 hover:bg-[#3B9E99]/20 border border-[#3B9E99]/30 rounded text-[#3B9E99] transition-colors">
                <Send className="h-4 w-4" />
              </button>
            </form>
          </div>

          {/* Subprocess System Logs Feed */}
          <div className="glass-panel p-3 flex flex-col h-[180px]">
            <h2 className="text-xs font-semibold text-[#2D2C2A] font-mono flex items-center gap-1.5 mb-1.5">
              <Terminal className="h-3.5 w-3.5 text-[#7A756C]" />
              System Log Feed
            </h2>
            <div className="flex-1 overflow-y-auto bg-[#1E1D1B] border border-[#2D2C2A] p-2 rounded font-mono text-xs text-[#E6E3DB] leading-normal flex flex-col gap-1">
              {logFeed.map((log, i) => (
                <div key={i} className="truncate select-all hover:text-white shrink-0">{log}</div>
              ))}
            </div>
          </div>
          
        </div>

      </div>
    </div>
  );
}
