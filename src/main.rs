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
    fn run(&mut self) {
        println!("{} start Work::run()", self.name);
        let timeout = rand::random::<u32>() % 3000;
        println!("{} Work::run() will busy {} ms", self.name, timeout);
        sleep_ms(timeout);
        self.value = timeout;
        println!("{} Work::run() was completed!!!!", self.name);
    }
    fn name(&self)->&str {
        &self.name
    }
}


fn main() {
    const THREAD_MAX :usize = 2;
    let (pipe, results) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
    let mut pool = AgentPool::new("Pool", THREAD_MAX, pipe.clone());
    let gate = pool.gate();
    let thread = thread::spawn(move || pool.run());

    let mut generated:usize = 0;
    for idx in 0..10 {
        let name = format!("Task_{}", idx);
        let task = Work::new(name.clone());
        sleep_ms(rand::random::<u32>() % 100);
        gate.send(Message::Invoke(task)).unwrap();
        println!("Main has send {}", name.clone());
        generated += 1;
    }
    gate.send(Message::Quit).unwrap();

    let mut processed:usize = 0;
    while let Ok(msg) = results.recv() {
        match msg {
            Message::Done(_, work) => {
                println!("Main has received {} with the value {}", work.name, work.value);
                processed += 1;
            }
            Message::Exited(name) => {
                println!("Main has received Exited('{}')", &name);
                break;
            }
            _ => {
                println!("Main has received unexpected command.");
            }
        }
    }
    thread.join().unwrap();
    println!("Main was done.");
    println!("Generated {} tasks.", generated);
    println!("Processed {} tasks.", processed);
}
