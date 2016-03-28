//! This library contains representation of the linked network of cells.
//!
//! The possible way for traverse of the network
//!
//! ```
//!use std::thread;
//!use std::thread::sleep;
//!use std::time::Duration;
//!use std::sync::mpsc;
//!use std::sync::mpsc::{Sender, Receiver};
//!
//!#[macro_use]
//!extern crate log;
//!extern crate env_logger;
//!
//!extern crate rand;
//!extern crate lib;
//!use lib::message::{Message, Task};
//!use lib::supervisor::Supervisor;
//!
//!struct Work {
//!    name : String,
//!    value : u32
//!}
//!impl Drop for Work {
//!    fn drop(&mut self) {
//!        trace!("{} was dropped.", self.name);
//!    }
//!}
//!impl Work {
//!    pub fn new<Name : Into<String>>(name : Name) -> Self{
//!        let name = name.into();
//!        trace!("Work::new({})", name);
//!        Work { name : name.into(), value : 0 }
//!    }
//!}
//!impl Task for Work {
//!    fn run(&mut self) {
//!        self.value = rand::random::<u32>() % 3000;
//!        trace!("{}.invoke() started. ETA: {} ms", self.name, self.value);
//!        sleep(Duration::new(0,self.value));
//!        trace!("{}.invoke() was completed!", self.name);
//!    }
//!    fn name(&self)->String {
//!        self.name.clone()
//!    }
//!}
//!
//!struct WorkSource {
//!    idx : usize
//!}
//!impl WorkSource {
//!    pub fn new() -> Self {
//!        WorkSource { idx : 0 }
//!    }
//!}
//!impl Iterator for WorkSource {
//!    type Item = Work;
//!    fn next(&mut self) -> Option<Work> {
//!        self.idx += 1;
//!        sleep(Duration::new(0, rand::random::<u32>() % 100));
//!        Some(Work::new(format!("Task_{}", &self.idx)))
//!    }
//!}
//!
//!fn main() {
//!    let _ = env_logger::init().unwrap();
//!    const THREAD_MAX :usize = 4;
//!    let (pipe, results) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
//!    let mut sup = Supervisor::new("Pool", THREAD_MAX, pipe.clone());
//!    let gate = sup.gate();
//!    let thread = thread::spawn(move || sup.run());
//!
//!    const MAX_TASK:usize = 100000;
//!    let mut generated:usize = 0;
//!    let mut processed:usize = 0;
//!
//!    enum WorkState {
//!        ReadyForTask,
//!        WaitForDone,
//!        Done
//!    }
//!    let mut state:WorkState = WorkState::ReadyForTask;
//!    let mut source = WorkSource::new();
//!    loop {
//!        let mut message = Message::Nothing;
//!        match state {
//!            WorkState::ReadyForTask => {
//!                if generated < MAX_TASK {
//!                    if let Some(task) = source.next() {
//!                        trace!("gate.send(Message::Invoke(Task({})))", task.name());
//!                        gate.send(Message::Invoke(task)).unwrap();
//!                        generated += 1;
//!                    }
//!                } else {
//!                    trace!("gate.send(Message::Quit)");
//!                    gate.send(Message::Quit).unwrap();
//!                    state = WorkState::WaitForDone;
//!                }
//!                if let Ok(msg) = results.try_recv() {
//!                    message = msg;
//!                }
//!            }
//!            WorkState::WaitForDone => {
//!                if let Ok(msg) = results.recv() {
//!                    message = msg;
//!                }
//!            }
//!            WorkState::Done => {
//!                break;
//!            }
//!        }
//!        match message {
//!            Message::Done(agent, task) => {
//!                trace!("Message::Done({},{}) with the value {}", agent, task.name, task.value);
//!                processed += 1;
//!            }
//!            Message::Resend(task) => {
//!                println!("Message::Resend({})", task.name);
//!                gate.send(Message::Invoke(task)).unwrap();
//!            }
//!            Message::Exited(name) => {
//!                trace!("Message::Exited({})", &name);
//!                state = WorkState::Done;
//!            }
//!            Message::Nothing => {
//!            }
//!            _ => {
//!                panic!("Has received unexpected command.");
//!            }
//!        }
//!    }
//!
//!    thread.join().unwrap();
//!    println!("Generated {} tasks.", generated);
//!    println!("Processed {} tasks.", processed);
//!}
//!
//! ```
//!

pub mod types;
pub mod worker;
pub mod supervisor;
pub use self::types::{Message};
pub use self::worker::Worker;
pub use self::supervisor::Supervisor;

#[macro_use]
extern crate log;
extern crate env_logger;
