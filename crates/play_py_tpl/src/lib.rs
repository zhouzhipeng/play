use include_dir::{Dir, include_dir};

mod py_runner;

// the name `TplEngine` should always be.
pub type TplEngine = py_runner::PyRunner;
pub static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../server/templates");
