mod py_runner;

// the name `TplEngine` should always be.
pub type TplEngine = py_runner::PyRunner;
pub use py_runner::TEMPLATES_DIR;