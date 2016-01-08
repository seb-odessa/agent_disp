use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use super::message::{Message, Task};

pub struct Agent<Obj:Task+Send>
{
    name : String,
    gate : Sender<Message<Obj>>,
    input : Receiver<Message<Obj>>,
    output  : Sender<Message<Obj>>,
}
impl <Obj:Task+Send> Drop for Agent <Obj> {
    fn drop(&mut self) {
        println!("{} dropped.", self.name);
    }
}
impl <Obj:Task+Send> Agent <Obj> {
    #[allow(dead_code)]
    pub fn new<Name : Into<String>>(name:Name, results:Sender<Message<Obj>>) -> Self {
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
