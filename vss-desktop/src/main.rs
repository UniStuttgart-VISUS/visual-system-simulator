mod cmd;
mod flow;
mod node;

use vss::*;

fn main() -> Result<(), String> {
    set_load(Box::new(|path| {
        std::fs::read(path)
            .map(std::io::Cursor::new)
            .map_err(|err| format!("Cannot read file '{path}' ({err})"))
    }));
    cmd::run()
}
