
pub trait Task {
    fn name(&self)->String;
    fn run(&mut self);
}

pub enum Message<Obj:Task+Send>{
    Quit,
    Exited(String),
    Invoke(Obj),
    Done(String, Obj),
    Resend(Obj),
    Nothing
}
