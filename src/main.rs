extern crate flate2;
extern crate imap;
extern crate native_tls;

use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::result;
use std::str;

use flate2::write::GzEncoder;
use flate2::Compression;

use imap::types;

use indicatif::ProgressBar;

#[derive(Debug)]
enum ApplicationError {
    NotEnoughArguments,
    MissingEnvelope,
    MissingBody,
    MissingMessageId,
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
        if item.attributes().contains(&types::NameAttribute::NoSelect) {
            continue;
        }

        let name = item.name();
        println!("{}", name);
        fs::create_dir_all(format!("output/{}", name))?;
        session.select(name)?;
        let messages = session.search("ALL")?;
        let progress = ProgressBar::new(messages.len().try_into().unwrap());
        for seq in messages.iter() {
            let fetch = session.fetch(seq.to_string(), "(BODY.PEEK[] ENVELOPE)")?;
            for f in fetch.iter() {
                let envelope = f.envelope().ok_or(ApplicationError::MissingEnvelope)?;
                let body = f.body().ok_or(ApplicationError::MissingBody)?;
                let message_id = envelope
                    .message_id
                    .ok_or(ApplicationError::MissingMessageId)?;

                let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
                encoder.write_all(body)?;
                let compressed_bytes = encoder.finish()?;
                let filebase = str::replace(str::from_utf8(message_id)?, "/", "_");
                fs::write(format!("output/{}/{}.gz", name, filebase), compressed_bytes)?;
            }
            progress.inc(1);
        }
    }

    Ok(())
}
