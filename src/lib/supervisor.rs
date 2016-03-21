use std::thread;
use std::thread::{JoinHandle};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::collections::HashMap;

use super::worker::Worker;
use super::message::{Message, Task};

pub struct Supervisor<Obj:Task+Send + 'static> {
    name : String,                      /// The name of the Pool
    workers : usize,                     /// The number of threads
    gate : Sender<Message<Obj>>,        /// The external side of the INPUT channel
    input : Receiver<Message<Obj>>,     /// The internal side of the INPUT channel
    output  : Sender<Message<Obj>>,     /// The internal part of the OUTPUT channel
    worker_gate:HashMap<String, Sender<Message<Obj>>>,
    worker_ready:HashMap<String, bool>,
    worker_thread:Vec<JoinHandle<()>>,
    worker_result:Receiver<Message<Obj>>,
    active:usize,
    wait_quit:bool,
}

impl <Obj:Task+Send> Drop for Supervisor<Obj> {
    fn drop(&mut self) {
        while !self.worker_thread.is_empty() {
            if let Ok(_) = self.worker_thread.pop().unwrap().join() {
                trace!("{} successful join a thread.", self.name);
            }
        }
        trace!("{} was dropped.", self.name);
    }

}
impl <Obj:Task+Send> Supervisor <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name : Name, workers : usize, results : Sender<Message<Obj>>) -> Self {
        let name = name.into();
        trace!("{} created.", &name);
        let (gate, input) = mpsc::channel();
        let (worker_gate, worker_result) : (Sender<Message<Obj>>, Receiver<Message<Obj>>) = mpsc::channel();

        let mut pool = Supervisor {
            name:name,
            workers:workers,
            gate:gate,
            input:input,
            output:results,
            worker_gate:HashMap::new(),
            worker_ready:HashMap::new(),
            worker_thread:Vec::new(),
            worker_result:worker_result,
            active:0,
            wait_quit:false,
        };
        for idx in 0..pool.workers {
            let name = format!("Agent_{}", (idx+1));
            let mut worker = Worker::new(name.clone(), worker_gate.clone());
            pool.worker_gate.insert(name.clone(), worker.gate());
            pool.worker_ready.insert(name.clone(), true);
            pool.worker_thread.push(thread::spawn(move || worker.run()));
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
        return self.active >= self.worker_gate.len();
    }

    fn get_ready_worker(&self) -> Option<String> {
        for (k, v) in &self.worker_ready {
            if *v {
                return Some(k.clone());
            }
        }
        return None;
    }

    fn handle_input(&mut self, msg:Message<Obj>) -> (){
        match msg {
            Message::Quit => {
                trace!("{} <= Message::Quit", self.name);
                for (_, gate) in &self.worker_gate {
                    gate.send(Message::Quit).unwrap();
                    self.active += 1;
                }
                self.wait_quit = true;
            },
            Message::Invoke(task) => {
                match self.get_ready_worker() {
                    Some(name) => {
                        trace!("{} <= Message::Invoke({})", self.name, task.name());
                        *self.worker_ready.get_mut(&name).unwrap() = false;
                        self.worker_gate[&name].send(Message::Invoke(task)).unwrap();
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
            Message::Done(worker, task) => {
                trace!("{} <= Message::Done({},{})", self.name, worker, task.name());
                *self.worker_ready.get_mut(&worker).unwrap() = true;
                self.output.send(Message::Done(worker, task)).unwrap();
                self.active -= 1;
            }
            Message::Exited(worker) => {
                trace!("{} <= Message::Exited({})", self.name, worker);
                self.worker_gate.remove(&worker);
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
            match self.worker_result.recv() {
                Ok(msg) => self.handle_results(msg),
                Err(err) => panic!("{} has found {}", self.name, err),
            }
        } else {
            match self.worker_result.try_recv() {
                Ok(msg) => self.handle_results(msg),
                Err(TryRecvError::Empty) => {},
                Err(TryRecvError::Disconnected) => panic!("{} has found disconnected channel", self.name)
            }
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        while !self.worker_gate.is_empty() {
            self.process_results();
            self.process_input();
        }
        self.output.send(Message::Exited(self.name.clone())).unwrap();
    }
}
