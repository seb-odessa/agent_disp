use std::thread;
use std::thread::{sleep_ms};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};

extern crate rand;
extern crate lib;
use lib::message::{Message, Task};
use lib::agent_pool::{AgentPool};

struct Work {
    name : String,
    value : u32
}
impl Drop for Work {
    fn drop(&mut self) {
        println!("{} was dropped.", self.name);
    }
}
impl Work {
    pub fn new<Name : Into<String>>(name : Name) -> Self{
        Work { name : name.into(), value : 0 }
    }
}
impl Task for Work {
    fn invoke(&mut self) {
        self.value = rand::random::<u32>() % 3000;
        println!("{}.invoke() started. ETA: {} ms", self.name, self.value);
        sleep_ms(self.value);
        println!("{}.invoke() was completed!", self.name);
    }
    fn name(&self)->&str {
        &self.name
    }
}

#[allow(dead_code)]
struct WorkSource {
    idx : usize
}
impl WorkSource {
    #[allow(dead_code)]
    pub fn new() -> Self {
        WorkSource { idx : 0 }
    }
}
impl  Iterator for WorkSource {
    type Item = Work;
    fn next(&mut self) -> Option<Work> {
        self.idx += 1;
        sleep_ms(rand::random::<u32>() % 500);
        Some(Work::new(format!("Task_{}", &self.idx)))
    }
}

fn main() {
    const THREAD_MAX :usize = 8;
    let (pipe, results) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
    let mut pool = AgentPool::new("Pool", THREAD_MAX, pipe.clone());
    let gate = pool.gate();
    let thread = thread::spawn(move || pool.run());

    const MAX_TASK:usize = 20;
    let mut generated:usize = 0;
    let mut processed:usize = 0;

    enum WorkState {
        ReadyForTask,
        WaitForDone,
        Done
    }
    let mut state:WorkState = WorkState::ReadyForTask;
    let mut source = WorkSource::new();

    loop {
        let mut message = Message::Nothing;
        match state {
            WorkState::ReadyForTask => {
                if generated < MAX_TASK {
                    if let Some(task) = source.next() {
                        gate.send(Message::Invoke(task)).unwrap();
                        generated += 1;
                    }
                } else {
                    gate.send(Message::Quit).unwrap();
                    state = WorkState::WaitForDone;
                }
                if let Ok(msg) = results.try_recv() {
                    message = msg;
                }
            }
            WorkState::WaitForDone => {
                if let Ok(msg) = results.recv() {
                    message = msg;
                }
            }
            WorkState::Done => {
                break;
            }
        }
        match message {
            Message::Done(agent, task) => {
                println!("Message::Done({},{}) with the value {}", agent, task.name, task.value);
                processed += 1;
            }
            Message::Resend(task) => {
                println!("Message::Resend({})", task.name);
                gate.send(Message::Invoke(task)).unwrap();
            }
            Message::Exited(name) => {
                println!("Message::Exited({})", &name);
                state = WorkState::Done;
            }
            Message::Nothing => {
            }
            _ => {
                panic!("Has received unexpected command.");
            }
        }
    }

    thread.join().unwrap();
    println!("Main was done.");
    println!("Generated {} tasks.", generated);
    println!("Processed {} tasks.", processed);
}
