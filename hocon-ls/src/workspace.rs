use std::collections::HashMap;

use nom_language::error::VerboseError;
use rusty_hocon::ast::HoconValue;
use rusty_hocon::config::HoconError;
use rusty_hocon::parser::parse;

pub struct Workspace {
    open_files: HashMap<String, OpenFile>,
}

// Must remain private to hide nasty unsafe properties
#[allow(clippy::box_collection)]
struct OpenFile {
    hocon: HoconValue<'static>,
    // Must box to avoid moves breaking the hocon value
    content: Box<String>,
}

impl<'a> OpenFile {
    fn new(content: String) -> Result<OpenFile, HoconError> {
        let content = Box::new(content);
        match parse::<VerboseError<&str>>(&content) {
            Ok(hocon) => {
                Ok(OpenFile {
                    // As long as access to hocon is restricted to the lifetime of OpenFile, this
                    // transmute is safe.
                    hocon: unsafe { std::mem::transmute(hocon) },
                    content,
                })
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_ast(&self) -> &HoconValue<'a> {
        // Expose the wrong 'static back as 'a lifetime.
        // This forces the borrow check back in live
        unsafe { std::mem::transmute(&self.hocon) }
    }
}

impl<'a> Workspace {
    pub fn new() -> Self {
        Workspace {
            open_files: HashMap::new(),
        }
    }

    pub fn open_file(&mut self, path: String, content: String) -> Result<(), HoconError> {
        let hocon = OpenFile::new(content)?;
        self.open_files.insert(path, hocon);
        Ok(())
    }

    pub fn get_ast(&self, path: &str) -> Option<&HoconValue<'a>> {
        self.open_files.get(path).map(|file| file.get_ast())
    }
}
