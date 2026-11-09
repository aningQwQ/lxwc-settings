use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Effect {
    WriteFile {
        path: PathBuf,
        contents: String,
        expected: Option<String>,
    },
    RunCommand {
        program: String,
        args: Vec<String>,
        best_effort: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub module_id: &'static str,
    pub title: String,
    pub detail: String,
    pub change_count: usize,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub id: String,
    pub module_id: &'static str,
    pub title: String,
    pub change_count: usize,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ApplyReport {
    pub results: Vec<TaskResult>,
    pub warnings: Vec<String>,
    pub elapsed_ms: u128,
}

impl ApplyReport {
    pub fn success_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.ok)
            .map(|result| result.change_count)
            .sum()
    }

    pub fn failure_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| !result.ok)
            .map(|result| result.change_count)
            .sum()
    }

    pub fn task_succeeded(&self, module_id: &str, task_id: &str) -> bool {
        self.results
            .iter()
            .any(|result| result.ok && result.module_id == module_id && result.id == task_id)
    }

    pub fn task_failed(&self, module_id: &str, task_id: &str) -> bool {
        self.results
            .iter()
            .any(|result| !result.ok && result.module_id == module_id && result.id == task_id)
    }
}
