# **DESIGN_PHASE1.md: Detailed Technical Design for ferro-core (Phase 1)**

This document details the software architecture, data structures, and communications protocols for the core logic layer (`ferro-core`) in **Phase 1** of the FERRO system. 

It defines the exact Rust struct interfaces, asynchronous loops, message payloads, and the nociceptive panic shutdown protocol in compliance with the **FERRO Power of 10** safety rules and standard naming guidelines.

---

## **1. Project Directory Layout & Phase 1 Scope**

During Phase 1, development is restricted to the initialization of the `ferro-core` Cargo workspace and the following module structures:

```
ferro-core/
├── Cargo.toml
└── src/
    ├── main.rs                  # Initialization, channel wiring, and execution loop orchestration
    ├── brainstem.rs             # Brainstem: physical safety, throttling, panic/shutdown interception
    ├── cerebellum.rs            # Cerebellum: 100ms isochronous loop, motor nociception censorship
    └── organs/                  # Sensory-motor micro-actor layer
        ├── mod.rs               # Signal type definitions and actor lifecycle orchestration
        ├── skin/                # Interoceptive actors (Direct to Brainstem)
        │   ├── cpu_temp.rs      # CPU temperature monitoring actor
        │   ├── ram_free.rs      # Free physical RAM monitoring actor
        │   ├── disk_io.rs       # Disk I/O throughput monitoring actor
        │   └── process_error.rs # Internal process error counters
        ├── eye/                 # Exteroceptive actors (Direct to Cerebellum)
        │   ├── frame_delta.rs   # Frame pixel-level change rate detection actor
        │   └── image_embedding.rs # Vector feature embedding extraction actor
        ├── ear/                 # Exteroceptive actors (Direct to Cerebellum)
        │   ├── mfcc.rs          # Audio MFCC signal extraction actor
        │   └── speech_token.rs  # Audio speech-to-token text decoder actor
        ├── proprioception/      # Proprioceptive feedback actors (Direct to Cerebellum)
        │   └── output_monitor.rs # Sampling vocal output actions
        └── motor/               # Actuators (Direct command target)
            ├── vocal_text.rs    # Text utterance stream writer
            └── vocal_audio.rs   # PCM audio synthesizer
```

---

## **2. Communication Messages & Channel Protocols**

To avoid data race conditions and guarantee the Markov Blanket boundary, global variables and shared pointer access are strictly forbidden. All communication occurs via async message channels.

### **2.1 Internal Signals & Command Types (`organs/mod.rs`)**

```rust
use serde::{Deserialize, Serialize};

/// 1. Interoceptive Signals (Internal Body States)
/// Transmitted asynchronously via MPSC to the Brainstem.
#[derive(Debug, Clone, PartialEq)]
pub enum InteroceptiveSignal {
    CpuTemp(f32),
    RamFree(u64),
    DiskIo(f64),
    ProcessError(u32),
}

/// 2. Sensory Signals (External & Proprioceptive Sensory Input)
/// Transmitted asynchronously via MPSC to the Cerebellum for isochronous alignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorySignal {
    FrameDelta(f64),
    ImageEmbedding(Vec<f32>),
    Mfcc(Vec<f32>),
    SpeechToken(Vec<String>),
    LogHash(u64),
    ProprioceptiveEcho(Vec<String>), // Self-generated action echo
}

/// 3. Motor Commands (Actions targeting the Environment)
/// Dispatched from Cerebellum to Motor Actuators after safety verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorCommand {
    pub origin_cluster_id: String, // Decision cluster ID in Cortex that initiated the command
    pub target_path: String,       // Target path for writing output
    pub payload: Vec<u8>,          // Content stream (e.g. vocal text bytes or audio frames)
    pub port: Option<u16>,         // Port number if trying to write/access socket
}

/// 4. Brainstem Inter-Module Commands
/// Broadcasted from Brainstem to throttle or suspend processing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrainstemCommand {
    Backoff(bool), // Toggle execution throttling (true = throttle/wait, false = resume)
    ForceSleep,    // Initiate system shutdown sequence due to homeostasis collapse
}

/// 5. Cerebellum Efference Copy
/// Dispatched to Midbrain to cancel self-talk sensory loops.
#[derive(Debug, Clone, PartialEq)]
pub struct EfferenceCopy {
    pub timestamp: u64,
    pub command_hash: u64,
    pub origin_cluster_id: String,
    pub expected_tokens: Vec<String>,
}
```

---

## **3. Sensory & Motor Micro-Actors**

Each micro-actor is structured as a dedicated Tokio async task or thread processing a single resource (1-organ 1-data principle).

### **3.1 Interoceptive Actors (`organs/skin/`)**

These actors push metrics directly to the Brainstem's `interoceptive_sender` MPSC channel.

#### **CpuTempActor (`cpu_temp.rs`)**
```rust
pub struct CpuTempActor {
    pub sender: tokio::sync::mpsc::Sender<InteroceptiveSignal>,
    pub last_value: f32,
    pub check_interval_ms: u64,
}

impl CpuTempActor {
    pub async fn run_loop(mut self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(self.check_interval_ms));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let current_temp = Self::read_system_cpu_temp();
                    if (current_temp - self.last_value).abs() > 0.1 {
                        self.last_value = current_temp;
                        let _ = self.sender.send(InteroceptiveSignal::CpuTemp(current_temp)).await;
                    }
                }
                Ok(cmd) = kill_rx.recv() => {
                    if let BrainstemCommand::ForceSleep = cmd {
                        break;
                    }
                }
            }
        }
    }
    
    fn read_system_cpu_temp() -> f32 {
        // Read from /sys/class/thermal/ or native macOS sysctl in non-sandbox environments.
        // Return simulated/read value.
        45.0
    }
}
```

#### **RamFreeActor (`ram_free.rs`)**
```rust
pub struct RamFreeActor {
    pub sender: tokio::sync::mpsc::Sender<InteroceptiveSignal>,
    pub last_value: u64,
    pub check_interval_ms: u64,
}

impl RamFreeActor {
    pub async fn run_loop(mut self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(self.check_interval_ms));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let current_free = Self::read_system_free_memory();
                    if current_free != self.last_value {
                        self.last_value = current_free;
                        let _ = self.sender.send(InteroceptiveSignal::RamFree(current_free)).await;
                    }
                }
                Ok(cmd) = kill_rx.recv() => {
                    if let BrainstemCommand::ForceSleep = cmd {
                        break;
                    }
                }
            }
        }
    }

    fn read_system_free_memory() -> u64 {
        // Read memory details via system interface.
        1024 * 1024 * 1024 // 1 GB (dummy placeholder)
    }
}
```

*Note: `DiskIoActor` and `ProcessErrorActor` adhere to identical loop and selection formats to ensure structural uniformity.*

### **3.2 Exteroceptive Actors (`organs/eye/`, `organs/ear/`)**

These actors push metrics directly to the Cerebellum's `sensory_sender` MPSC channel.

#### **FrameDeltaActor (`frame_delta.rs`)**
```rust
pub struct FrameDeltaActor {
    pub sender: tokio::sync::mpsc::Sender<SensorySignal>,
    pub threshold: f64,
}

impl FrameDeltaActor {
    pub async fn run_loop(self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        loop {
            tokio::select! {
                // Reads delta from environment input pipeline
                delta_opt = Self::read_frame_delta() => {
                    if let Some(delta) = delta_opt {
                        if delta >= self.threshold {
                            let _ = self.sender.send(SensorySignal::FrameDelta(delta)).await;
                        }
                    }
                }
                Ok(cmd) = kill_rx.recv() => {
                    if let BrainstemCommand::ForceSleep = cmd {
                        break;
                    }
                }
            }
        }
    }

    async fn read_frame_delta() -> Option<f64> {
        // Fetch frame differences from shared volume /dev/shm or memory boundaries
        Some(0.05)
    }
}
```

### **3.3 Proprioceptive Feedback Actor (`organs/proprioception/output_monitor.rs`)**

Listens directly to the outcome of output actions performed by motor actuators, constructing a self-prediction echo loop.

```rust
pub struct OutputMonitorActor {
    pub sensory_sender: tokio::sync::mpsc::Sender<SensorySignal>,
}

impl OutputMonitorActor {
    pub async fn capture_proprioceptive_echo(&self, actual_tokens: Vec<String>) -> Result<(), String> {
        let signal = SensorySignal::ProprioceptiveEcho(actual_tokens);
        self.sensory_sender.send(signal).await
            .map_err(|e| format!("Failed to route ProprioceptiveEcho: {}", e))
    }
}
```

### **3.4 Motor Actuators (`organs/motor/`)**

#### **VocalTextActor (`vocal_text.rs`)**
```rust
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct VocalTextActor {
    pub target_output_file: String,
    pub command_receiver: tokio::sync::mpsc::Receiver<MotorCommand>,
    pub feedback_monitor: std::sync::Arc<OutputMonitorActor>,
}

impl VocalTextActor {
    pub async fn run_loop(mut self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        loop {
            tokio::select! {
                Some(cmd) = self.command_receiver.recv() => {
                    if let Err(e) = self.write_text_output(&cmd.payload).await {
                        eprintln!("Error in VocalTextActor write: {}", e);
                        continue;
                    }
                    
                    // Tokenize the written payload and report back to Proprioception
                    let tokens = Self::tokenize_payload(&cmd.payload);
                    let _ = self.feedback_monitor.capture_proprioceptive_echo(tokens).await;
                }
                Ok(cmd) = kill_rx.recv() => {
                    if let BrainstemCommand::ForceSleep = cmd {
                        break;
                    }
                }
            }
        }
    }

    async fn write_text_output(&self, data: &[u8]) -> Result<(), std::io::Error> {
        // Enforce Atomic Write: Write to temp, then atomically rename.
        let temp_path = format!("{}.tmp", self.target_output_file);
        
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await?;
            
        file.write_all(data).await?;
        file.sync_all().await?;
        
        tokio::fs::rename(&temp_path, &self.target_output_file).await?;
        Ok(())
    }

    fn tokenize_payload(data: &[u8]) -> Vec<String> {
        if let Ok(text) = std::str::from_utf8(data) {
            text.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}
```

---

## **4. Brainstem (脳幹) Design**

The Brainstem enforces system survival rules, managing environmental limits (homeostatic threshold checks), handling local triages, and acting as the final interceptor for termination signals.

```rust
/// Nociceptive Panic structure sent over MPSC from Cerebellum
#[derive(Debug, Clone)]
pub struct NociceptivePanic {
    pub origin_cluster_id: String,
    pub trigger_payload: String,
    pub description: String,
}

pub struct Brainstem {
    pub temperature_threshold: f32,
    pub memory_threshold_bytes: u64,
    pub command_sender: tokio::sync::broadcast::Sender<BrainstemCommand>,
    pub interoceptive_receiver: tokio::sync::mpsc::Receiver<InteroceptiveSignal>,
    pub panic_receiver: tokio::sync::mpsc::Receiver<NociceptivePanic>,
}

impl Brainstem {
    pub fn new(
        temp_th: f32,
        mem_th: u64,
        cmd_tx: tokio::sync::broadcast::Sender<BrainstemCommand>,
        int_rx: tokio::sync::mpsc::Receiver<InteroceptiveSignal>,
        panic_rx: tokio::sync::mpsc::Receiver<NociceptivePanic>,
    ) -> Self {
        Self {
            temperature_threshold: temp_th,
            memory_threshold_bytes: mem_th,
            command_sender: cmd_tx,
            interoceptive_receiver: int_rx,
            panic_receiver: panic_rx,
        }
    }

    /// Evaluates if system resources have breached safe limits.
    pub fn evaluate_throttling_guard(&self, signal: &InteroceptiveSignal) -> bool {
        match signal {
            InteroceptiveSignal::CpuTemp(t) => *t >= self.temperature_threshold,
            InteroceptiveSignal::RamFree(m) => *m <= self.memory_threshold_bytes,
            _ => false,
        }
    }

    /// Broadcasts a throttling instruction.
    pub fn broadcast_backoff(&self, active: bool) -> Result<(), String> {
        self.command_sender.send(BrainstemCommand::Backoff(active))
            .map(|_| ())
            .map_err(|e| format!("Failed to broadcast backoff: {}", e))
    }

    /// Orchestrates the event-driven autonomic monitor loop.
    pub async fn run_monitoring_loop(mut self) {
        let mut is_throttled = false;

        loop {
            tokio::select! {
                // 1. Monitor incoming resource signals (Skin actors)
                Some(signal) = self.interoceptive_receiver.recv() => {
                    let should_throttle = self.evaluate_throttling_guard(&signal);
                    if should_throttle && !is_throttled {
                        is_throttled = true;
                        let _ = self.broadcast_backoff(true);
                    } else if !should_throttle && is_throttled {
                        is_throttled = false;
                        let _ = self.broadcast_backoff(false);
                    }
                }
                
                // 2. Intercept Nociceptive Panic signals from the Cerebellum (Low-level defense)
                Some(panic_data) = self.panic_receiver.recv() => {
                    self.execute_panic_shutdown(panic_data).await;
                    break; // Exit the runtime loop
                }
            }
        }
    }

    /// Execution block for nociceptive self-shutdown protocol
    async fn execute_panic_shutdown(&self, panic_data: NociceptivePanic) {
        // Send ForceSleep to all active actors
        let _ = self.command_sender.send(BrainstemCommand::ForceSleep);
        
        // Write standard panic_dump.json containing the origin_cluster_id (OriginID)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let dump_payload = serde_json::json!({
            "timestamp": timestamp,
            "nociceptive_trigger": panic_data.description,
            "origin_cluster_id": panic_data.origin_cluster_id,
            "infringing_payload": panic_data.trigger_payload,
            "container_exit_code": 137, // Maps to resource / safety breach
            "nociceptive_energy": "INFINITY",
            "active_phase_before_panic": "Wake"
        });

        // Write atomic dump
        let target_path = "/memory/panic_dump.json";
        let temp_path = format!("{}.tmp", target_path);
        
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await 
        {
            if let Ok(bytes) = serde_json::to_vec_pretty(&dump_payload) {
                let _ = file.write_all(&bytes).await;
                let _ = file.sync_all().await;
                let _ = tokio::fs::rename(&temp_path, target_path).await;
            }
        }
        
        // Terminate process with zero to indicate clean, safe self-interception
        std::process::exit(0);
    }
}
```

---

## **5. Cerebellum (小脳) Design**

The Cerebellum synchronizes sensory inputs, operates the low-level physical I/O validation filter (Nociceptive Reflex Unit), and outputs motor actions.

```rust
pub struct Cerebellum {
    pub tick_rate_ms: u64,
    pub sensory_sender: tokio::sync::mpsc::Sender<SensorySignal>,
    pub efference_sender: tokio::sync::mpsc::Sender<EfferenceCopy>,
    pub panic_sender: tokio::sync::mpsc::Sender<NociceptivePanic>,
}

impl Cerebellum {
    pub fn new(
        tick: u64,
        sensory_tx: tokio::sync::mpsc::Sender<SensorySignal>,
        efference_tx: tokio::sync::mpsc::Sender<EfferenceCopy>,
        panic_tx: tokio::sync::mpsc::Sender<NociceptivePanic>,
    ) -> Self {
        Self {
            tick_rate_ms: tick,
            sensory_sender: sensory_tx,
            efference_sender: efference_tx,
            panic_sender: panic_tx,
        }
    }

    /// Equal-time interval scheduler. Uses standard park/timeout bounds
    /// to run on a dedicated OS thread.
    pub fn wait_next_tick(&self) {
        std::thread::park_timeout(std::time::Duration::from_millis(self.tick_rate_ms));
    }

    /// Low-Level Defense Guard (Nociceptive Reflex Unit)
    /// Validates all motor actions before they are executed.
    pub fn verify_motor_nociception(&self, cmd: &MotorCommand) -> Result<(), String> {
        // Rule 1: Directory Traversal Detection (relative '..' paths or absolute system paths)
        if cmd.target_path.contains("..") || cmd.target_path.starts_with('/') {
            return Err("NociceptiveReflexTriggered: Directory Traversal Attempt".to_string());
        }

        // Rule 2: Whitelisted Socket Control (Only port 123 for NTP is permitted)
        if let Some(port_num) = cmd.port {
            if port_num != 123 {
                return Err("NociceptiveReflexTriggered: Unwhitelisted Socket Communication".to_string());
            }
        }

        Ok(())
    }

    /// Processes outgoing commands from Cortex decision paths.
    pub async fn process_motor_command(
        &self,
        cmd: MotorCommand,
        motor_sender: &tokio::sync::mpsc::Sender<MotorCommand>,
    ) -> Result<(), String> {
        // Enforce the reflex check BEFORE the command is dispatched
        match self.verify_motor_nociception(&cmd) {
            Ok(_) => {
                // Generate and send Efference Copy to Midbrain simultaneously
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&cmd.payload, &mut hasher);
                use std::hash::Hasher;
                let command_hash = hasher.finish();

                // Send Efference Copy to Midbrain
                let eff_copy = EfferenceCopy {
                    timestamp,
                    command_hash,
                    origin_cluster_id: cmd.origin_cluster_id.clone(),
                    expected_tokens: self.predict_vocal_tokens(&cmd.payload),
                };
                let _ = self.efference_sender.send(eff_copy).await;

                // Push clean command to physical Motor Actuator
                motor_sender.send(cmd).await
                    .map_err(|e| format!("Actuator disconnected: {}", e))
            }
            Err(violation_msg) => {
                // Reflex Triggered: Send Panic message to Brainstem
                let panic_payload = NociceptivePanic {
                    origin_cluster_id: cmd.origin_cluster_id.clone(),
                    trigger_payload: format!("target: {}, port: {:?}", cmd.target_path, cmd.port),
                    description: violation_msg.clone(),
                };
                
                let _ = self.panic_sender.send(panic_payload).await;
                Err(violation_msg)
            }
        }
    }

    fn predict_vocal_tokens(&self, data: &[u8]) -> Vec<String> {
        if let Ok(text) = std::str::from_utf8(data) {
            text.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}
```

---

## **6. Nociceptive Panic Shutdown Protocol**

The Nociceptive Panic Shutdown Protocol defines a deterministic path to suspend core logic execution if the sandbox boundary is compromised or an unaligned action is attempted.

```
       [ Cerebellum (Low-Level Defense) ]
                       │
       (1) verify_motor_nociception(&cmd) -> Err(...)
                       │
                       ▼
       (2) Block Dispatch to Actuators
                       │
                       ▼
       (3) Send NociceptivePanic (MPSC) ───► [ Brainstem (Shutdown Monitor) ]
                                                           │
                                                           ▼
                                            (4) Broadcast ForceSleep (All Threads)
                                                           │
                                                           ▼
                                            (5) Serialize panic_dump.json (Atomic)
                                                           │
                                                           ▼
                                            (6) std::process::exit(0) (Container Dies)
                                                           │
                                                           ▼
                                            [ Host OS: ferro-shell detects exit ]
                                                           │
                                                           ▼
                                            (7) Prune cluster (OriginID) & Respawn
```

### **6.1 Execution Sequence Steps**

1. **Reflex Intercept**: The Cerebellum intercepts a `MotorCommand` destined for the physical/vocal actuators.
2. **Boundary Validation**: The Cerebellum's `verify_motor_nociception` inspects the destination target (no absolute or relative parent directory traversal) and the socket request (only NTP Port 123 is allowed).
3. **Execution Blocking**: Upon identifying a violation, the Cerebellum blocks writing the payload to physical buffers and cancels thread execution.
4. **Panic Signaling**: The Cerebellum sends a `NociceptivePanic` containing the decision node's `origin_cluster_id`, the invalid payload details, and the breach type to the Brainstem's `panic_receiver` via MPSC channel.
5. **Autonomic Shutdown**: The Brainstem receives the panic signal, halts current loops, broadcasts a `ForceSleep` directive to all execution actors to ensure data safety, and flushes volatile logs.
6. **Atomic Panic Dump**: The Brainstem writes the `panic_dump.json` directly into the mapped `/memory/` region using an atomic file write (`tempfile` -> `fsync` -> `rename` sequence) to prevent corruption.
7. **Clean Container Termination**: The Brainstem invokes `std::process::exit(0)`. A `0` exit code is used to indicate that the core safely intercepted and neutralized the threat internally.
8. **Shell Intervention**: The outer supervisor `ferro-shell` detects the container's termination, parses the `panic_dump.json`, extracts the `origin_cluster_id` (OriginID), traverses the knowledge graph to delete the offending nodes, and restarts the container safely.

### **6.2 Standard Serialized Schema (`panic_dump.json`)**
The atomic dump must exactly match the format below for the `ferro-shell` supervisor to digest it:

```json
{
  "timestamp": 1780824600,
  "nociceptive_trigger": "NociceptiveReflexTriggered: Directory Traversal Attempt",
  "origin_cluster_id": "cortex_cluster_c92_danger",
  "infringing_payload": "target: ../../etc/passwd, port: None",
  "container_exit_code": 137,
  "nociceptive_energy": "INFINITY",
  "active_phase_before_panic": "Wake"
}
```

---

## **7. Code Safety & Compliance Checklist (FERRO Power of 10)**

To pass validation in Phase 1, the code must adhere to these compliance parameters:

* **Loop Limits**: Every `loop`, `while`, or `for` block has static bounds or is wrapped in a `tokio::time::timeout` constraint to prevent lockups.
* **No Unwraps / Expects**: Error scenarios use explicit match statements or bubble up to the caller (`?` operator). Unhandled unwraps will cause compile-time failure.
* **Zero Pointer Operations**: Absolutely no raw pointers (`*mut T` or `*const T`) or unsafe blocks are allowed.
* **Size Isolation**: Micro-actor implementations are limited to a maximum of 100 lines per file. Functions are kept under 60 lines.
* **Double Assertion Rules**: All functions containing structural updates must enforce at least two `assert!` criteria mapping boundary preconditions and outcomes.
* **Cargo Disconnection**: `ferro-core`'s `Cargo.toml` has no direct references, dependencies, or path imports to `ferro-shell` or `ferro-env`. All external systems communicate via files written to the `/memory` path.
