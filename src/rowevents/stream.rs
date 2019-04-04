use std::fs::File;
use std::option::Option;
use std::io::{Result, Error, ErrorKind, Read};

pub struct FileStream {
    filename: String,
    file: Option<File>,
    content: Vec<u8>,
    offset: usize,
    counter: usize,
}

impl Read for FileStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(1)
    }
}


fn get_next_binlog_filename(filename: &String) -> Option<String> {
    let p = filename.rfind('.').unwrap();
    let numstr = &filename[p + 1 ..];
    if let Ok(num) = numstr.parse::<u32>() {
        let mut next = (&filename[.. p + 1]).to_owned();
        next += &format!("{:0w$}", num + 1, w=numstr.len());
        Some(next)
    } else {
        None
    }
}

impl FileStream {

    pub fn read_binlog_file_header(&mut self) -> bool {
        if let Some(ref mut file) = self.file {
            std::io::copy(&mut file.take(4), &mut std::io::sink()).is_ok()
        } else {
            false
        }
    }

    // pub fn read_next_binlog_file(&mut self) -> bool {
    //     self.reader.read_next_binlog_file();
    //     true
    // }

    pub fn from_file(filename: &str) -> Option<FileStream> {
        let mut result = File::open(filename);
        if let Ok(mut file) = result {
            Some(FileStream {
                filename: filename.to_string(),
                file: Some(file),
                content: vec![],
                offset: 0,
                counter: 0
                })
        } else {
            None
        }
    }

    pub fn read_next_binlog_file(&mut self) {
        if let Some(next_binlog_filename) = get_next_binlog_filename(&self.filename) {

            let mut result = File::open(&next_binlog_filename);
            if let Ok(mut file) = result {
                self.filename = next_binlog_filename;
                self.file = Some(file);
                self.content = vec![];
                self.offset = 0;
            }
        }
    }

    pub fn read(&mut self, size: usize) -> &[u8] {
        let mut from = self.offset;
        if from + size >= self.content.len() {
            match self.read_file(size) {
                Ok(0) => {
                    // println!("Reach the end of this binlog file");
                    return &[][..]
                },
                Err(_) => {
                    return &[][..]
                },
                Ok(read) if read < size => {
                    // Sometimes, especially when end of the file, read < size;
                    println!("!{:?}", &self.content[from .. from + read]);
                    return &self.content[from .. from + read]
                },
                Ok(_) => {}
            }
        }
        let threshold: usize = 1000000;
        self.offset += size;
        if from >= threshold {
            let remain = self.content.drain(threshold .. ).collect();
            self.content = remain;
            self.offset -= threshold;
            from -= threshold;
            println!("Resize content len => {}", self.content.len());
        }

        &self.content[from .. from + size]
    }

    // try! Read size * 2 bytes from file
    pub fn read_file(&mut self, size: usize) -> Result<usize> {
        let mut buffer = Vec::with_capacity(size * 2);
        buffer.resize(size, 0); // TODO: Read more content into buffer, and reduce the read times
        if let Some(ref mut file) = self.file {
            let read = file.read(&mut buffer)?;
            self.counter += 1;  // Read times + 1
            if read > 0 {
                // TO fix!
                // println!("DD {} {} {:?}", size, read, &buffer[0..read]);
                self.content.extend_from_slice(&buffer[0..read]);
                Ok(read)
            } else {
                Ok(0)
            }
            // TODO: the read MAYBE 0, should return Err
        } else {
            Err(Error::new(ErrorKind::Other, "oh no!"))
        }
    }
}
