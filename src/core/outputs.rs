/// One output mode: resolution plus refresh rate.
#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
}

impl Mode {
    pub fn label(&self) -> String {
        format!("{} × {} @ {:.2} Hz", self.width, self.height, self.refresh)
    }

    pub fn same_mode(&self, other: &Mode) -> bool {
        self.width == other.width
            && self.height == other.height
            && (self.refresh - other.refresh).abs() < 0.001
    }
}

/// One connected or configured output.
#[derive(Clone, Debug)]
pub struct OutputInfo {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub current: Option<Mode>,
    pub modes: Vec<Mode>,
    pub position: Option<(i32, i32)>,
    pub scale: Option<f32>,
}

/// Capability supplied by an output provider plugin.
///
/// Pages depend on this trait, never on a concrete provider.
pub trait Outputs: Send + Sync {
    fn scan(&self) -> Result<Vec<OutputInfo>, String>;
    fn apply(&self, output: &str, mode: &Mode) -> Result<(), String>;
}
