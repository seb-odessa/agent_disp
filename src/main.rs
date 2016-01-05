use std::thread;
use std::thread::{JoinHandle,sleep_ms};
use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver};
use std::collections::VecDeque;
use std::collections::HashSet;
use rand::Rng;
use std::collections::HashMap;

extern crate rand;

pub trait Task {
    fn name(&self)->&str;
    fn run(&mut self);
}

#[derive(Debug)]
pub enum Message<Obj:Task>{
    Quit,
    Exited(String),
    Invoke(Obj),
    Done(String, Obj),
}

struct Agent<Obj:Task>
{
    name : String,
    gate : Sender<Message<Obj>>,
    input : Receiver<Message<Obj>>,
    output  : Sender<Message<Obj>>,
}
impl <Obj:Task> Drop for Agent <Obj> {
    fn drop(&mut self) {
        println!("{} dropped.", self.name);
    }
}
impl <Obj:Task> Agent <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name : Name, results : Sender<Message<Obj>>) -> Self {
        let name = name.into();
        println!("{} created.", &name);
        let (tx, rx) = mpsc::channel();
        Agent { name:name, gate:tx, input:rx, output:results }
    }

    #[allow(dead_code)]
    pub fn gate(&self) -> Sender<Message<Obj>> {
        println!("{} cloning Gate.", self.name);
        self.gate.clone()
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        while let Ok(msg) = self.input.recv() {
            match msg {
                Message::Quit => {
                    println!("{} has received 'Message::Quit'.", self.name);
                    break
                },
                Message::Invoke(mut arg) => {
                    println!("{} has received {}.", self.name, &arg.name());
                    arg.run();
                    self.output.send(Message::Done(self.name.clone(), arg)).unwrap();
                }
                _ => {
                    println!("{} has received unexpected command.", self.name);
                }
            }
        }
        println!("{} finish it's work.", &self.name);
        self.output.send(Message::Exited(self.name.clone())).unwrap();
    }
}

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
        let timeout = rand::random::<u32>() % 2000;
        println!("{} Work::run() will busy {} ms", self.name, timeout);
        sleep_ms(timeout);
        self.value = timeout;
        println!("{} finish Work::run()", self.name);
    }
    fn name(&self)->&str {
        &self.name
    }
}

struct AgentPool<Obj:Task> {
    name : String,
    agents : u32,
    gate : Sender<Message<Obj>>,
    input : Receiver<Message<Obj>>,
    output  : Sender<Message<Obj>>,
}
impl <Obj:Task> Drop for AgentPool<Obj> {
    fn drop(&mut self) {
        println!("{} dropped.", self.name);
    }
}
impl <Obj:Task> AgentPool <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name : Name, agents : u32, results : Sender<Message<Obj>>) -> Self {
        let name = name.into();
        println!("{} created.", &name);
        let (tx, rx) = mpsc::channel();
        AgentPool {
            name:name,
            agents:agents,
            gate:tx,
            input:rx,
            output:results
        }
    }

    #[allow(dead_code)]
    pub fn gate(&self) -> Sender<Message<Obj>> {
        println!("{} cloning Gate.", self.name);
        self.gate.clone()
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        let (pipe, results) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
        let mut gates = HashMap::new();
        let mut threads = VecDeque::new();
////////////////////////////////////////////////////////////////////
        for idx in 0..self.agents {
            let agent_name = format!("Agent_{}", (idx+1));
            let mut agent = Agent::new(agent_name.clone(), pipe.clone());
            gates.insert(agent_name.clone(), agent.gate());
            threads.push_back(thread::spawn(move || agent.run()));
        }
////////////////////////////////////////////////////////////////////
        while let Ok(msg) = self.input.recv() {
            match msg {
                Message::Quit => {
                    println!("{} has received 'Message::Quit'.", self.name);
                    for (_, gate) in &gates {
                        gate.send(Message::Quit).unwrap();
                    }
                    break
                },
                Message::Invoke(mut arg) => {
                    println!("{} has received {}.", self.name, &arg.name());
                    arg.run();
                    self.output.send(Message::Done(self.name.clone(), arg)).unwrap();
                }
                _ => {
                    println!("{} has received unexpected command.", self.name);
                }
            }
        }
        println!("{} finish it's work.", &self.name);
////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////
        while !gates.is_empty() {
            match results.recv() {
                Ok(message) => {
                    if let Message::Exited(name) = message {
                        println!("{} has exited!!!!", &name);
                        gates.remove(&name);
                    }
                },
                Err(what) => {
                    println!("Has received error {}.", what);
                }
            }
        }
////////////////////////////////////////////////////////////////////
        while !threads.is_empty() {
            threads.pop_front().unwrap().join().unwrap();
        }
        println!("Done.");
////////////////////////////////////////////////////////////////////
    }
}

fn main() {

    let (pipe, results) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
    let mut a1 = Agent::new("Agent1", pipe.clone());
    let mut a2 = Agent::new("Agent2", pipe.clone());

    let gate1 = a1.gate();
    let gate2 = a2.gate();

    let mut threads = VecDeque::new();
    let mut agents = HashSet::new();

    agents.insert(a1.name.clone());
    threads.push_back(thread::spawn(move || a1.run()));
    agents.insert(a2.name.clone());
    threads.push_back(thread::spawn(move || a2.run()));

    gate1.send(Message::Invoke(Work::new("Task1"))).unwrap();
    gate1.send(Message::Invoke(Work::new("Task2"))).unwrap();
    gate2.send(Message::Invoke(Work::new("Task3"))).unwrap();
    gate1.send(Message::Invoke(Work::new("Task4"))).unwrap();
    gate1.send(Message::Quit).unwrap();

    gate2.send(Message::Invoke(Work::new("Task5"))).unwrap();
    gate2.send(Message::Quit).unwrap();


    while !agents.is_empty() {
        match results.recv() {
            Ok(message) => {
                match message {
                    Message::Done(agent, work) => {
                        println!("{} has done with value {} from agent {}", work.name, work.value, agent);
                    }
                    Message::Exited(name) => {
                        println!("{} has exited!!!!", &name);
                        agents.remove(&name);
                    }
                    _ => {
                        println!("Has received unexpected command.");
                    }
                }
            },
            Err(what) => {
                println!("Has received error {}.", what);
            }
        }
    }

    println!("Done.");
    while !threads.is_empty() {
        threads.pop_front().unwrap().join().unwrap();
    }
}
