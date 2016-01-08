use std::thread;
use std::thread::{JoinHandle, sleep_ms};
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
    agents_threads:Vec<JoinHandle<()>>,
    agents_gates:HashMap<String, Sender<Message<Obj>>>,
    agents_tasks:HashMap<String, usize>,
    agent_result:Receiver<Message<Obj>>,
    active:i64,
}
impl <Obj:Task+Send> Drop for AgentPool<Obj> {
    fn drop(&mut self) {
        while !self.agents_threads.is_empty() {
            self.agents_threads.pop().unwrap().join().unwrap();
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
            agents_threads:Vec::new(),
            agents_gates:HashMap::new(),
            agents_tasks:HashMap::new(),
            agent_result:agent_result,
            active:0,
        };
        for idx in 0..pool.agents {
            let name = format!("Agent_{}", (idx+1));
            let mut agent = Agent::new(name.clone(), agent_gate.clone());
            pool.agents_tasks.insert(name.clone(), 0);
            pool.agents_gates.insert(name.clone(), agent.gate());
            pool.agents_threads.push(thread::spawn(move || agent.run()));
        }
        return pool;
    }

    #[allow(dead_code)]
    pub fn gate(&self) -> Sender<Message<Obj>> {
        println!("{} cloning Gate.", self.name);
        self.gate.clone()
    }

    fn next_agent(&self) -> String {
        let mut map = self.agents_tasks.iter();
        let (mut min_k, mut min_v) = map.next().unwrap();
        for (k, v) in map.skip(1) {
            if v < min_v {
                min_k = k;
                min_v = v;
            }
        }
        return min_k.clone();
    }

    fn process_input(&mut self, msg:Message<Obj>) -> bool{
        match msg {
            Message::Quit => {
                println!("{} has received 'Message::Quit'.", self.name);
                for (_, gate) in &self.agents_gates {
                    gate.send(Message::Quit).unwrap();
                }
                return true;
            },
            Message::Invoke(arg) => {
                let name = self.next_agent();
                *self.agents_tasks.get_mut(&name).unwrap() += 1;
                println!("{} will send {}-th task to {}", self.name, self.agents_tasks[&name], name);
                self.agents_gates[&name].send(Message::Invoke(arg)).unwrap();
                return true;
            }
            _ => {
                panic!("{} has received unexpected command.", self.name);
                }
        }
    }

    fn read_input(&mut self) -> bool
    {
        match self.input.try_recv(){
            Ok(msg) => return self.process_input(msg),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => panic!("{} has found disconnected channel", self.name),            
        }
        return true;
    }

    fn process_result(&mut self, msg:Message<Obj>) -> bool{
        match msg {
            Message::Done(agent, work) => {
                println!("{} has received task {} from {}.", self.name, work.name(), agent);
                *self.agents_tasks.get_mut(&agent).unwrap() -= 1;
                self.output.send(Message::Done(agent, work)).unwrap();
            }
            Message::Exited(name) => {
                println!("{} has removed the {} from pool. Remined tasks {}.", self.name, &name, self.agents_tasks[&name]);
                self.agents_gates.remove(&name);
            }
            _ => {
                println!("Has received unexpected command.");
            }
        }
        return true;
    }

    fn read_results(&mut self) -> bool {
        match self.agent_result.try_recv() {
            Ok(msg) => return self.process_result(msg),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => panic!("{} has found disconnected channel", self.name)
        }
        true;
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        while !self.agents_gates.is_empty() {
            let has_input = self.read_input();
            if has_input {
                self.active += 1;
            }
            let has_results = self.read_results();
            if has_results {
                self.active -= 1;
            }
            if !has_input && !has_results {
                println!("{} will sleep for 300 ms. Active Tasks {}", self.name, self.active);
                sleep_ms(300);
            }
        }
        println!("{}::run() has finish it's work", self.name);
////////////////////////////////////////////////////////////////////
    }
}
