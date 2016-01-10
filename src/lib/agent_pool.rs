use std::thread;
use std::thread::{JoinHandle};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::collections::HashMap;
use super::agent::{Agent};
use super::message::{Message, Task};

pub struct AgentPool<Obj:Task+Send + 'static> {
    name : String,                      /// The name of the Pool
    agents : usize,                     /// The number of threads
    gate : Sender<Message<Obj>>,        /// The external side of the INPUT channel
    input : Receiver<Message<Obj>>,     /// The internal side of the INPUT channel
    output  : Sender<Message<Obj>>,     /// The internal part of the OUTPUT channel
    agent_gate:HashMap<String, Sender<Message<Obj>>>,
    agent_ready:HashMap<String, bool>,
    agent_thread:Vec<JoinHandle<()>>,
    agent_result:Receiver<Message<Obj>>,
    active:usize,
    wait_quit:bool,

}
impl <Obj:Task+Send> Drop for AgentPool<Obj> {
    fn drop(&mut self) {
        println!("{}.drop()", self.name);
        while !self.agent_thread.is_empty() {
            self.agent_thread.pop().unwrap().join().unwrap();
        }
        println!("{} was dropped.", self.name);
        self.output.send(Message::Exited(self.name.clone())).unwrap();
    }

}
impl <Obj:Task+Send> AgentPool <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name : Name, agents : usize, results : Sender<Message<Obj>>) -> Self {
        let name = name.into();
        println!("{} created.", &name);
        let (gate, input) = mpsc::channel();
        let (agent_gate, agent_result) : (Sender<Message<Obj>>, Receiver<Message<Obj>>) = mpsc::channel();

        let mut pool = AgentPool {
            name:name,
            agents:agents,
            gate:gate,
            input:input,
            output:results,
            agent_gate:HashMap::new(),
            agent_ready:HashMap::new(),
            agent_thread:Vec::new(),
            agent_result:agent_result,
            active:0,
            wait_quit:false,
        };
        for idx in 0..pool.agents {
            let name = format!("Agent_{}", (idx+1));
            let mut agent = Agent::new(name.clone(), agent_gate.clone());
            pool.agent_gate.insert(name.clone(), agent.gate());
            pool.agent_ready.insert(name.clone(), true);
            pool.agent_thread.push(thread::spawn(move || agent.run()));
        }
        return pool;
    }

    #[allow(dead_code)]
    pub fn gate(&self) -> Sender<Message<Obj>> {
        println!("{} cloning Gate.", self.name);
        self.gate.clone()
    }

    fn is_pool_empty(&self) -> bool {
        return self.active == 0 && !self.wait_quit;
    }

    fn is_pool_full(&self) -> bool {
        return self.active >= self.agent_gate.len();
    }

    fn get_ready_agent(&self) -> Option<String> {
        for (k, v) in &self.agent_ready {
        //    println!("{}.next_agent(): ({}, {})", self.name, k, v);
            if *v {
                return Some(k.clone());
            }
        }
        return None;
    }

    fn handle_input(&mut self, msg:Message<Obj>) -> (){
        match msg {
            Message::Quit => {
                println!("{} has received 'Message::Quit'.", self.name);
                for (_, gate) in &self.agent_gate {
                    gate.send(Message::Quit).unwrap();
                    self.active += 1;
                }
                self.wait_quit = true;
            },
            Message::Invoke(arg) => {
                let name = self.get_ready_agent().unwrap();
                *self.agent_ready.get_mut(&name).unwrap() = false;
                println!("{} will send {} to {}", self.name, arg.name(), name);
                self.agent_gate[&name].send(Message::Invoke(arg)).unwrap();
                self.active += 1;
            }
            _ => {
                panic!("{} has received unexpected command.", self.name);
                }
        }

    }

    fn handle_results(&mut self, msg:Message<Obj>) -> () {
        match msg {
            Message::Done(agent, work) => {
                println!("{} has received {} from {}.", self.name, work.name(), agent);
                *self.agent_ready.get_mut(&agent).unwrap() = true;
                self.output.send(Message::Done(agent, work)).unwrap();
                self.active -= 1;
            }
            Message::Exited(name) => {
                println!("{} has removed the {} from pool.", self.name, &name);
                self.agent_gate.remove(&name);
                self.active -= 1;
            }
            _ => {
                panic!("{} has received unexpected command.", self.name);
            }
        }
    }

    fn process_input(&mut self) -> () {
        if self.is_pool_empty() {            
            match self.input.recv(){
                Ok(msg) => self.handle_input(msg),
                Err(err) => panic!("{} has found {}", self.name, err),
            }
        } else {
            match self.input.try_recv(){
                Ok(msg) => self.handle_input(msg),
                Err(TryRecvError::Empty) => {},
                Err(TryRecvError::Disconnected) => panic!("{} has found disconnected channel", self.name),
            }

        }
    }

    fn process_results(&mut self) -> () {
        if self.is_pool_full() {
            match self.agent_result.recv() {
                Ok(msg) => self.handle_results(msg),
                Err(err) => panic!("{} has found {}", self.name, err),
            }
        } else {
            match self.agent_result.try_recv() {
                Ok(msg) => self.handle_results(msg),
                Err(TryRecvError::Empty) => {},
                Err(TryRecvError::Disconnected) => panic!("{} has found disconnected channel", self.name)
            }
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        while !self.agent_gate.is_empty() {
            self.process_results();
            self.process_input();
        }
        println!("{}::run() has finish it's work", self.name);
////////////////////////////////////////////////////////////////////
    }
}
