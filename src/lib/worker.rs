use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use super::message::{Message, Task};

pub struct Worker<Obj:Task+Send>
{
    name : String,
    gate : Sender<Message<Obj>>,
    input : Receiver<Message<Obj>>,
    output  : Sender<Message<Obj>>,
    processed : usize,
}
impl <Obj:Task+Send> Drop for Worker <Obj> {
    fn drop(&mut self) {
        trace!("{} dropped. Processed {} tasks.", self.name, self.processed);
    }
}
impl <Obj:Task+Send> Worker <Obj> {
    pub fn new<Name : Into<String>>(name:Name, results:Sender<Message<Obj>>) -> Self {
        let name = name.into();
        trace!("{} created.", &name);
        let (tx, rx) = mpsc::channel();
        Worker { name:name, gate:tx, input:rx, output:results, processed : 0 }
    }

    pub fn gate(&self) -> Sender<Message<Obj>> {
        self.gate.clone()
    }

    pub fn run(&mut self) {
        while let Ok(msg) = self.input.recv() {
            match msg {
                Message::Quit => {
                    trace!("{} <= Message::Quit", self.name);
                    break
                },
                Message::Invoke(mut task) => {
                    trace!("{} <= Message::Invoke({})", self.name, task.name());
                    task.run();
                    self.output.send(Message::Done(self.name.clone(), task)).unwrap();
                    self.processed += 1;
                }
                _ => {
                    panic!("{} has received unexpected command.", self.name);
                }
            }
        }
        self.output.send(Message::Exited(self.name.clone())).unwrap();
    }
}
