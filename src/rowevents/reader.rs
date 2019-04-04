
use rowevents::parser::Parser;
use rowevents::stream::FileStream;
use rowevents::event_header::EventHeader;
use rowevents::events::*;
use std::io::{Error, ErrorKind, Result, Read};
extern crate regex;
use regex::Regex;


// T: BinlogReader

// -- FileReader  ---+
//                   | ---- Parser
// -- SocketReader --+

// rename to file reader

// create socket reader

pub trait EventReader {
    fn read_event(&mut self) -> Result<(EventHeader, Event)>;
}

pub struct FileReader {
    parser: Parser<FileStream>,
    skip_next_event: bool,
    concerned_events: Vec<i8>,
    excluded_db_table_list: Vec<Regex>,
}

impl FileReader {
    pub fn new(filename: &str) -> Result<FileReader> {
        if let Some(stream) = FileStream::from_file(filename) {
            let mut parser = Parser::new(stream);
            parser.get_mut().read_binlog_file_header();
            Ok(FileReader{
                parser: parser,
                skip_next_event: false,
                concerned_events: Vec::with_capacity(20),
                excluded_db_table_list: Vec::with_capacity(20),
            })
        } else {
            Err(Error::new(ErrorKind::Other, "oh no!"))
        }
    }

    pub fn open_next_binlog_file(&mut self) -> bool {
        self.parser.get_mut().read_next_binlog_file();
        self.parser.get_mut().read_binlog_file_header()
    }

    #[inline]
    pub fn add_concerned_event(&mut self, event_type: i8) {
        self.concerned_events.push(event_type);
    }

    #[inline]
    pub fn is_concerned_event(&mut self, event_type: i8) -> bool  {
        self.concerned_events.len() == 0 || self.concerned_events.contains(&event_type)
    }

    pub fn add_excluded_db_table(&mut self, db_table_name: &str) {
        let regexp = db_table_name.replace(".", "\\.");
        let regexp = regexp.replace("*", "\\w*");
        self.excluded_db_table_list.push(Regex::new(&regexp).unwrap());
    }

    pub fn is_excluded(&mut self, db_name: &str, table_name: &str) -> bool {
        let db_table_name = db_name.to_string() + "." + table_name;
        for ref re in self.excluded_db_table_list.iter() {
            if re.is_match(&db_table_name) {
                return true;
            }
        }
        return false;
    }

    #[inline]
    pub fn read_event_header(&mut self) -> Result<EventHeader> {
        self.parser.read_event_header()
    }

    pub fn read_event_detail(&mut self, eh: &EventHeader) -> Result<Event> {
        match eh.get_event_type() {
            QUERY_EVENT => self.parser.read_query_event(eh),

            STOP_EVENT | ROTATE_EVENT => {
                let e = self.parser.read_rotate_event(eh);
                self.open_next_binlog_file();
                e
            },

            FORMAT_DESCRIPTION_EVENT => self.parser.read_format_descriptor_event(eh),
            XID_EVENT => self.parser.read_xid_event(eh),

            TABLE_MAP_EVENT  => self.parser.read_table_map_event(eh),

            // WRITE_ROWS_EVENT  => self.parser.read_event(eh),
            // UPDATE_ROWS_EVENT  => self.parser.read_event(eh),
            // DELETE_ROWS_EVENT  => self.parser.read_event(eh),

            WRITE_ROWS_EVENT2 => self.parser.read_write_event(eh),
            UPDATE_ROWS_EVENT2 => self.parser.read_update_event(eh),
            DELETE_ROWS_EVENT2 => self.parser.read_delete_event(eh),

            _ => self.parser.read_unknown_event(eh)
        }
    }

    pub fn read_unknown_event(&mut self, eh: &EventHeader) -> Result<Event> {
        self.parser.read_unknown_event(eh)
    }

    pub fn set_skip_next_event(&mut self, skip: bool) {
        self.skip_next_event = skip;
    }

    #[inline]
    pub fn skip_next_event(&self) -> bool {
        self.skip_next_event
    }
}

impl EventReader for FileReader {
    fn read_event(&mut self) -> Result<(EventHeader, Event)> {
        if let Ok(eh) = self.read_event_header() {

            if self.skip_next_event || !self.is_concerned_event(eh.get_event_type()) {
                if let Ok(e) = self.read_unknown_event(&eh) {
                    // Recover from skip
                    self.set_skip_next_event(false);
                    Ok((eh, e))
                } else {
                    Err(Error::new(ErrorKind::Other, "oh no!"))
                }
            } else if let Ok(e) = self.read_event_detail(&eh) {
                match e {
                    Event::TableMap(ref e) => {
                        if self.is_excluded(&e.db_name, &e.table_name) {
                            // println!("Excluded {}.{}", e.db_name, e.table_name);
                            self.set_skip_next_event(true);
                        }
                    },

                    Event::Rotate(ref e) => {
                        println!("Open next binlog file...");
                        self.open_next_binlog_file();
                    },
                    _ => ()
                }

                Ok((eh, e))

            } else {
                Err(Error::new(ErrorKind::Other, "oh no!"))
            }
        } else {
            Err(Error::new(ErrorKind::Other, "oh no!"))
        }
    }
}

impl Iterator for FileReader {
    type Item = (EventHeader, Event);

    // next() is the only required method
    fn next(&mut self) -> Option<(EventHeader, Event)> {
        self.read_event().ok()
    }
}

