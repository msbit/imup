extern crate flate2;
extern crate imap;
extern crate native_tls;

use std::collections::HashMap;
use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
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
    let mut message_paths = HashMap::new();

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
            for f in session.fetch(seq.to_string(), "(ENVELOPE)")?.iter() {
                let envelope = f.envelope().ok_or(ApplicationError::MissingEnvelope)?;
                let msg_id = String::from_utf8(
                    envelope
                        .message_id
                        .ok_or(ApplicationError::MissingMessageId)?
                        .to_vec(),
                )?;
                let filename = format!("output/{}/{}.gz", name, str::replace(&msg_id, "/", "_"));
                if Path::new(&filename).exists() {
                    continue;
                }

                if message_paths.contains_key(&msg_id) {
                    fs::hard_link(message_paths.get(&msg_id).unwrap(), filename)?;
                    continue;
                }

                for f in session
                    .fetch(seq.to_string(), "(BODY.PEEK[] ENVELOPE)")?
                    .iter()
                {
                    let envelope = f.envelope().ok_or(ApplicationError::MissingEnvelope)?;
                    assert!(
                        msg_id
                            == String::from_utf8(
                                envelope
                                    .message_id
                                    .ok_or(ApplicationError::MissingMessageId)?
                                    .to_vec()
                            )?
                    );

                    let body = f.body().ok_or(ApplicationError::MissingBody)?;

                    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
                    encoder.write_all(body)?;
                    let compressed_bytes = encoder.finish()?;
                    fs::write(&filename, compressed_bytes)?;
                }

                message_paths.insert(msg_id, filename);
            }

            progress.inc(1);
        }
    }

    Ok(())
}
