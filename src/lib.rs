//! CPL, the Combined Programming Language of Cambridge and London (1962-66),
//! implemented from the Reference Manual draft in the CPL Working Papers of
//! July 1966. See the README for what is and is not covered.

pub mod ast;
pub mod builtins;
pub mod code;
pub mod compile;
pub mod lexer;
pub mod parser;
pub mod value;
pub mod vm;

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

pub fn compile_source(src: &str) -> Result<Rc<code::Proto>, String> {
    let toks = lexer::lex(src)?;
    let items = parser::parse(toks)?;
    compile::compile(&items)
}

/// CPL recursion is carried on the host stack, so programs run on a thread
/// with a large one. WebAssembly has no threads to start: there the program
/// runs where it is and `vm::MAX_DEPTH` is lower.
#[cfg(target_family = "wasm")]
pub fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    f()
}

#[cfg(not(target_family = "wasm"))]
pub fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .expect("cannot start the interpreter thread")
        .join()
        .expect("the interpreter thread failed")
}

struct Shared(Rc<RefCell<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run a program with the given input stream; returns what it
/// wrote and how it ended.
pub fn run_source(src: &str, input: &str, max_steps: Option<u64>) -> (String, Result<(), String>) {
    let (src, input) = (src.to_string(), input.to_string());
    on_big_stack(move || {
        let proto = match compile_source(&src) {
            Ok(p) => p,
            Err(e) => return (String::new(), Err(e)),
        };
        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut vm = vm::Vm::new(
            Box::new(Shared(buf.clone())),
            Box::new(std::io::Cursor::new(input.into_bytes())),
        );
        vm.max_steps = max_steps;
        let r = vm.execute(proto);
        drop(vm);
        let out = String::from_utf8_lossy(&buf.borrow()).into_owned();
        (out, r)
    })
}
