use std::thread;
use std::thread::{JoinHandle, sleep_ms};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::collections::HashMap;
use super::agent::{Agent};
use super::message::{Message, Task};

pub struct AgentPool<Obj:Task+Send + 'static> {
    name : String,
    agents : usize,
    gate : Sender<Message<Obj>>,
    input : Receiver<Message<Obj>>,
    output  : Sender<Message<Obj>>,
    threads : Vec<JoinHandle<()>>,
}
impl <Obj:Task+Send> Drop for AgentPool<Obj> {
    fn drop(&mut self) {
        while !self.threads.is_empty() {
            self.threads.pop().unwrap().join().unwrap();
        }
        println!("{} was dropped.", self.name);
    }
}
impl <Obj:Task+Send> AgentPool <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name : Name, agents : usize, results : Sender<Message<Obj>>) -> Self {
        let name = name.into();
        println!("{} created.", &name);
        let (tx, rx) = mpsc::channel();
        AgentPool {
            name:name,
            agents:agents,
            gate:tx,
            input:rx,
            output:results,
            threads : Vec::new(),
        }

    }

    #[allow(dead_code)]
    pub fn gate(&self) -> Sender<Message<Obj>> {
        println!("{} cloning Gate.", self.name);
        self.gate.clone()
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        let (pipe, results) : (Sender<Message<Obj>>, Receiver<Message<Obj>>) = mpsc::channel();
        let mut gates = HashMap::new();
        let mut tasks = HashMap::new();
////////////////////////////////////////////////////////////////////
        for idx in 0..self.agents {
            let name = format!("Agent_{}", (idx+1));
            let mut agent = Agent::new(name.clone(), pipe.clone());
            tasks.insert(name.clone(), 0);
            gates.insert(name.clone(), agent.gate());
            self.threads.push(thread::spawn(move || agent.run()));
        }
////////////////////////////////////////////////////////////////////
        let mut curr:usize = 0;
        let mut wait:bool = false;
        while !gates.is_empty() {
            if wait {
                wait = false;
                println!("{} will sleep for 300 ms", self.name);
                sleep_ms(300);
            }
            match self.input.try_recv(){
                Ok(msg) =>  {
                    println!("{} has received {} th command.", self.name, curr);
                    match msg {
                        Message::Quit => {
                            println!("{} has received 'Message::Quit'.", self.name);
                            for (_, gate) in &gates {
                                gate.send(Message::Quit).unwrap();
                            }
                            break
                        },
                        Message::Invoke(arg) => {
                            let mut idx:usize = 0;
                            for (name, gate) in &gates {
                                //println!("{} has looking for agent idx = {}, curr = {}.", self.name, idx, curr % self.agents);
                                if curr % self.agents == idx {
                                    println!("{} has received {} and will send to {}.", self.name, &arg.name(), name);
                                    gate.send(Message::Invoke(arg)).unwrap();
                                    *tasks.get_mut(name).unwrap() += 1;
                                    println!("{} has send {}-th task to {}", self.name, tasks[name], name);
                                    break;
                                }
                                idx += 1;
                            }
                        }
                        _ => {
                            println!("{} has received unexpected command.", self.name);
                            }
                    }
                    curr = curr + 1;
                }
                Err(TryRecvError::Empty) => {
                    wait = true;
                }
                Err(TryRecvError::Disconnected) => {
                    println!("{} has found disconnected channel", self.name);
                }
            }
            match results.try_recv() {
                Ok(message) => {
                    wait = false;
                    match message {
                        Message::Done(agent, work) => {
                            println!("{} has received task {} from {}.", self.name, work.name(), agent);
                            *tasks.get_mut(&agent).unwrap() -= 1;
                            self.output.send(Message::Done(agent, work)).unwrap();

                        }
                        Message::Exited(name) => {
                            println!("{} has removed the {} from pool. Remined tasks {}.", self.name, &name, tasks[&name]);
                            gates.remove(&name);
                        }
                        _ => {
                            println!("Has received unexpected command.");
                        }
                    }
                },
                Err(TryRecvError::Empty) => {
                    wait = true;
                }
                Err(TryRecvError::Disconnected) => {
                    println!("{} has found disconnected channel", self.name);
                }
            }
        }
////////////////////////////////////////////////////////////////////
        while !gates.is_empty() {
            if let Ok(message) = results.recv() {
                if let Message::Exited(name) = message {
                    println!("{} has exited", &name);
                    gates.remove(&name);
                }
            }
        }
        self.output.send(Message::Exited(self.name.clone())).unwrap();
        println!("{} has finish it's work", self.name);
////////////////////////////////////////////////////////////////////
    }
}
