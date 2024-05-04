use crate::todo::tasks::{Task, TasksFile};

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct TaskFactory {
    tasks: Vec<Task>,
    current: Task,
}

#[allow(dead_code)]
impl TaskFactory {
    pub fn new() -> Self {
        TaskFactory::default()
    }
    pub fn save(self) {
        TasksFile::add_many(self.tasks);
    }
    pub fn append(&mut self) -> &mut Self {
        self.tasks.push(self.current.clone());
        self.current = Task::default();
        self
    }
    pub fn set_description(&mut self, description: String) -> &mut Self {
        self.current.description = description;
        self
    }
    pub fn set_title(&mut self, title: String) -> &mut Self {
        self.current.title = title;
        self
    }
    pub fn set_done(&mut self, done: bool) -> &mut Self {
        self.current.done = done;
        self
    }
}
