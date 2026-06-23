use crate::message::InteroceptiveSignal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Brainstem {
    terminate_flag: Arc<AtomicBool>,
    last_ram_free: u64,
    last_cpu_temp: f32,
}

impl Brainstem {
    pub fn new(terminate_flag: Arc<AtomicBool>) -> Self {
        Self {
            terminate_flag,
            last_ram_free: u64::MAX,
            last_cpu_temp: 0.0,
        }
    }

    pub fn handle_signal(&mut self, signal: InteroceptiveSignal) {
        if self.should_terminate() {
            return;
        }
        // R5: 状態変更を伴う・引数ありの関数に最低2つのアサーションを義務付け
        assert!(self.last_ram_free > 0, "Error: ram_free state must be valid");

        match signal {
            InteroceptiveSignal::CpuTemp(temp) => {
                assert!((-100.0..200.0).contains(&temp), "Error: Invalid CPU temperature");
                self.last_cpu_temp = temp;
                if temp > 85.0 {
                    self.terminate_flag.store(true, Ordering::SeqCst);
                }
            }
            InteroceptiveSignal::RamFree(free_bytes) => {
                self.last_ram_free = free_bytes;
                // 空きメモリが10MBを下回った場合に自死要請
                if free_bytes < 10 * 1024 * 1024 {
                    self.terminate_flag.store(true, Ordering::SeqCst);
                }
            }
            InteroceptiveSignal::DiskIo(io) => {
                assert!(io >= 0.0, "Error: Disk IO must be non-negative");
            }
            InteroceptiveSignal::ProcessError(err_count) => {
                if err_count > 5 {
                    self.terminate_flag.store(true, Ordering::SeqCst);
                }
            }
        }

        assert!(self.last_ram_free > 0, "Error: post-condition check failed");
        assert!(self.last_cpu_temp >= -100.0, "Error: cpu_temp invalid");
    }

    pub fn should_terminate(&self) -> bool {
        self.terminate_flag.load(Ordering::SeqCst)
    }
}
