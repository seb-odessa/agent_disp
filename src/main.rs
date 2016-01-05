use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver};
use std::collections::VecDeque;
use std::collections::HashSet;

pub trait Task {
    fn name(&self)->&str;
    fn run(&self);
}

#[derive(Debug)]
pub enum Message<Obj:Task>{
    Quit,
    Exited(String),
    Invoke(Obj),
    Done(Obj),
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
    pub fn run(&self) {
        while let Ok(msg) = self.input.recv() {
            match msg {
                Message::Quit => {
                    println!("{} has received 'AgentRequest::Quit'.", self.name);
                    break
                },
                Message::Invoke(arg) => {
                    println!("{} has received {}.", self.name, &arg.name());
                    arg.run();
                    self.output.send(Message::Done(arg)).unwrap();
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
    name : String
}
impl Drop for Work {
    fn drop(&mut self) {
        println!("{} was dropped.", self.name);
    }
}
impl Work {
    pub fn new<Name : Into<String>>(name : Name) -> Self{
        Work { name : name.into() }
    }
}
impl Task for Work {
    fn run(&self) {
        println!("{} executes Work::run()", self.name);
    }
    fn name(&self)->&str {
        &self.name
    }
}

fn main() {

    let (tx, rx) : (Sender<Message<Work>>, Receiver<Message<Work>>) = mpsc::channel();
    let a1 = Agent::new("Agent1", tx.clone());
    let a2 = Agent::new("Agent2", tx.clone());

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
        match rx.recv() {
            Ok(message) => {
                match message {
                    Message::Done(work) => {
                        println!("{} received back.", work.name());
                    }
                    Message::Exited(name) => {
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
