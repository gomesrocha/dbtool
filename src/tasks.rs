#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub status: TaskStatus,
}

#[derive(Debug, PartialEq)]
pub enum TaskStatus {
    Success,
    Failed(String),
    Skipped,
}