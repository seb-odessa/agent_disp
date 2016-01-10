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
        while !self.agent_thread.is_empty() {
            if let Ok(_) = self.agent_thread.pop().unwrap().join() {
                println!("{} successful join a thread.", self.name);
            }
        }
        println!("{} was dropped.", self.name);
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

    pub fn gate(&self) -> Sender<Message<Obj>> {
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
            if *v {
                return Some(k.clone());
            }
        }
        return None;
    }

    fn handle_input(&mut self, msg:Message<Obj>) -> (){
        match msg {
            Message::Quit => {
                println!("{} <= Message::Quit", self.name);
                for (_, gate) in &self.agent_gate {
                    gate.send(Message::Quit).unwrap();
                    self.active += 1;
                }
                self.wait_quit = true;
            },
            Message::Invoke(task) => {
                match self.get_ready_agent() {
                    Some(name) => {
                        println!("{} <= Message::Invoke({})", self.name, task.name());
                        *self.agent_ready.get_mut(&name).unwrap() = false;
                        self.agent_gate[&name].send(Message::Invoke(task)).unwrap();
                        self.active += 1;
                    }
                    None => {
                        self.output.send(Message::Resend(task)).unwrap()
                    }
                }
            }
            _ => {
                panic!("{} has received unexpected command.", self.name);
                }
        }

    }

    fn handle_results(&mut self, msg:Message<Obj>) -> () {
        match msg {
            Message::Done(agent, task) => {
                println!("{} <= Message::Done({},{})", self.name, agent, task.name());
                *self.agent_ready.get_mut(&agent).unwrap() = true;
                self.output.send(Message::Done(agent, task)).unwrap();
                self.active -= 1;
            }
            Message::Exited(agent) => {
                println!("{} <= Message::Exited({})", self.name, agent);
                self.agent_gate.remove(&agent);
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
        self.output.send(Message::Exited(self.name.clone())).unwrap();
    }
}
