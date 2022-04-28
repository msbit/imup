extern crate imap;
extern crate native_tls;

use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::result;

#[derive(Debug)]
enum ApplicationError {
    NotEnoughArguments,
}

impl error::Error for ApplicationError {}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}

fn main() -> result::Result<(), Box<dyn error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(Box::new(ApplicationError::NotEnoughArguments));
    }

    let username = &args[1];
    let password = &args[2];
    let tls = native_tls::TlsConnector::builder().build()?;
    let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)?;
    let mut session = client.login(username, password).map_err(|e| e.0)?;

    for item in session.list(Some("/"), Some("*"))?.iter() {
        let name = item.name();
        print!("{}:", name);
        io::stdout().flush()?;
        fs::create_dir_all(format!("output/{}", name))?;
        session.select(name)?;
        let messages = session.search("ALL")?;
        for seq in messages.iter() {
            print!(" {}", seq);
            io::stdout().flush()?;
            let fetch = session.fetch(seq.to_string(), "BODY.PEEK[]")?;
            for f in fetch.iter() {
                if let Some(body) = f.body() {
                    fs::write(format!("output/{}/{}", name, seq), body)?;
                }
            }
        }
        println!("");
    }

    Ok(())
}
