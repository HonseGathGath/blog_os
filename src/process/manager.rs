// src/process/manager.rs
use super::process::{Process, ProcessId, ProcessState};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;

/// Process Manager - Tracks all processes and handles scheduling
pub struct ProcessManager {
    processes: Vec<Process>,
    ready_queue: VecDeque<ProcessId>,
    current_pid: Option<ProcessId>,
    next_pid: u64,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: Vec::new(),
            ready_queue: VecDeque::new(),
            current_pid: None,
            next_pid: 1,
        }
    }

    /// Add a new process to the manager
    pub fn add_process(&mut self, mut process: Process) -> ProcessId {
        let pid = ProcessId::new(self.next_pid);

        process.pid = pid;

        if process.is_runnable() {
            self.ready_queue.push_back(pid);
        }

        self.processes.push(process);
        pid
    }

    /// Get process by PID
    pub fn get_process(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }

    /// Remove a process (when terminated)
    pub fn remove_process(&mut self, pid: ProcessId) -> Option<Process> {
        if let Some(pos) = self.processes.iter().position(|p| p.pid == pid) {
            // Remove from ready queue
            self.ready_queue.retain(|&queued_pid| queued_pid != pid);

            // Remove current if it's this process
            if self.current_pid == Some(pid) {
                self.current_pid = None;
            }

            Some(self.processes.remove(pos))
        } else {
            None
        }
    }

    /// Schedule the next process to run
    pub fn schedule(&mut self) -> Option<&mut Process> {
        if self.ready_queue.is_empty() {
            return None;
        }

        // Simple round-robin scheduling
        if let Some(current) = self.current_pid {
            // Check if current process is runnable WITHOUT mutable borrow
            let is_runnable = self
                .processes
                .iter()
                .find(|p| p.pid == current)
                .map(|p| p.is_runnable())
                .unwrap_or(false);

            if is_runnable {
                self.ready_queue.push_back(current);
            }
        }

        // Get next process from front
        while let Some(next_pid) = self.ready_queue.pop_front() {
            // Find the process position
            if let Some(pos) = self.processes.iter().position(|p| p.pid == next_pid) {
                if self.processes[pos].is_runnable() {
                    self.current_pid = Some(next_pid);
                    self.processes[pos].state = super::process::ProcessState::Running;
                    // Return mutable reference to the process
                    return Some(&mut self.processes[pos]);
                }
            }
        }

        None
    }

    /// Get current running process
    pub fn current_process(&mut self) -> Option<&mut Process> {
        self.current_pid.and_then(|pid| self.get_process(pid))
    }

    /// Yield current process (voluntary context switch)
    pub fn yield_current(&mut self) -> Option<ProcessId> {
        if let Some(current) = self.current_pid {
            if let Some(process) = self.get_process(current) {
                process.state = super::process::ProcessState::Ready;
                self.ready_queue.push_back(current);
            }
            self.current_pid = None;
            return Some(current);
        }
        None
    }

    /// Get all processes (for debugging/info)
    pub fn all_processes(&self) -> &[Process] {
        &self.processes
    }

    /// Get number of processes
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    /// Get number of runnable processes
    pub fn runnable_count(&self) -> usize {
        self.processes.iter().filter(|p| p.is_runnable()).count()
    }
}
