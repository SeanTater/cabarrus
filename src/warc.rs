//! Web ARChive format parser
//!
//! This was originally warc-nom-parser.
//! The main difference is that most headers are ignored, and none are returned.
use std::{str, io, mem, ptr};
use nom::{Offset, space, Needed, Consumer, ConsumerState, Input, Move, IResult, Producer};
use std::io::{Read, StdinLock};
const SIZE_LIMIT: usize = 1 << 20;


/// Stream WARC's from stdin (probably what you want)
pub struct WarcStreamer {
    file_producer: StdinProducer,
    consumer: WarcConsumer,
}

impl WarcStreamer {
    /// Open a stream from stdin
    pub fn new() -> io::Result<Self> {
        Ok(WarcStreamer {
            file_producer: StdinProducer::new(SIZE_LIMIT)?,
            consumer: WarcConsumer::new(),
        })
    }
}
impl Iterator for WarcStreamer {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.file_producer.apply(&mut self.consumer);
            let m = mem::replace(&mut self.consumer.last_record, None);
            match self.consumer.state {
                State::Error => { return None; }
                State::Done => { return None; }
                _ => { if m.is_some() { return m; }} // else loop
            }
        }
    }
}

/// Warc record (we only keep the payload)
pub type Record = String;

#[derive(PartialEq,Eq,Debug,Clone)]
pub enum State {
    Beginning,
    End,
    Done,
    Error,
}

pub struct WarcConsumer {
    c_state: ConsumerState<usize, (), Move>,
    state: State,
    last_record: Option<Record>,
}

impl WarcConsumer {
    pub fn new() -> Self {
        WarcConsumer {
            state: State::Beginning,
            c_state: ConsumerState::Continue(Move::Consume(0)),
            last_record: None,
        }
    }
}

impl<'a> Consumer<&'a [u8], usize, (), Move> for WarcConsumer {
    fn state(&self) -> &ConsumerState<usize, (), Move> {
        &self.c_state
    }

    fn handle(&mut self, input: Input<&'a [u8]>) -> &ConsumerState<usize, (), Move> {
        self.last_record = None;
        match self.state {
            State::Beginning => {
                let end_of_file = match input {
                    Input::Eof(_) => true,
                    _ => false,
                };
                match input {
                    Input::Empty | Input::Eof(None) => {
                        self.state = State::Done;
                        self.c_state = ConsumerState::Error(());
                    }
                    Input::Element(sl) |
                    Input::Eof(Some(sl)) => {
                        match record_complete(sl) {
                            IResult::Error(_) => {
                                // Don't abort, instead skip along.
                                // WARC's break sometimes.
                                self.c_state = ConsumerState::Continue(Move::Consume(1));
                            }
                            IResult::Incomplete(n) => {
                                if end_of_file {
                                    self.state = State::End;
                                } else {
                                    self.c_state = ConsumerState::Continue(Move::Await(n));
                                }
                            }
                            IResult::Done(i, entry) => {
                                self.last_record = Some(entry);
                                self.c_state = ConsumerState::Continue(Move::Consume(sl.offset(i)));
                            }
                        }
                    }
                }
            }
            State::End => {
                self.state = State::Done;
            }
            State::Done | State::Error => {
                self.state = State::Error;
                self.c_state = ConsumerState::Error(())
            }
        };
        &self.c_state
    }
}

fn version_number(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            46 | 48...57 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn utf8_allowed(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            0...31 => return IResult::Done(&input[idx..], &input[..idx]),
            _ => continue,
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            33 | 35...39 | 42 | 43 | 45 | 48...57 | 65...90 | 94...122 | 124 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

named!(init_line <&[u8], (&str, &str)>,
    chain!(
        tag!("\r")?                 ~
        tag!("\n")?                 ~
        tag!("WARC")                ~
        tag!("/")                   ~
        space?                      ~
        version: map_res!(version_number, str::from_utf8)~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        || {("WARCVERSION", version)}
    )
);

named!(header_match <&[u8], (&str, &str)>,
    chain!(
        name: map_res!(token, str::from_utf8)~
        space?                      ~
        tag!(":")                   ~
        space?                      ~
        value: map_res!(utf8_allowed, str::from_utf8)~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        || {(name, value)}
    )
);

named!(header_aggregator<&[u8], Vec<(&str,&str)> >, many1!(header_match));

named!(warc_header<&[u8], ((&str, &str), Vec<(&str,&str)>) >,
    chain!(
        version: init_line          ~
        headers: header_aggregator  ~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        move ||{(version, headers)}
    )
);

/// Parses one record and returns an IResult from nom
///
/// IResult<&[u8], Record>
///
/// See records for processing more than one. The documentation is not displaying.
///
/// # Examples
/// ```ignore
///  extern crate warc_parser;
///  extern crate nom;
///  use nom::{IResult};
///  let parsed = warc_parser::record(&bbc);
///  match parsed{
///      IResult::Error(_) => assert!(false),
///      IResult::Incomplete(_) => assert!(false),
///      IResult::Done(i, entry) => {
///          let empty: Vec<u8> =  Vec::new();
///          assert_eq!(empty, i);
///          assert_eq!(13, entry.headers.len());
///      }
///  }
/// ```
pub fn record(input: &[u8]) -> IResult<&[u8], Record> {
    // TODO if the stream parser does not get all the header it fails .
    // like a default size of 10 doesnt for for a producer
    match warc_header(input) {
        IResult::Done(i, ((_name, _version), headers)) => {
            let length = headers.iter()
                .find(|&&(k, _v)| k == "Content-Length")
                .map(|&(_k, v)| v)
                .and_then(|l| l.parse::<usize>().ok())
                .unwrap_or(0);
            if length > i.len() {
                // Need to refill the buffer
                return IResult::Incomplete(Needed::Size(length - i.len()));
            } else {
                // It's already in the buffer
                return IResult::Done(&i[length..], // slide forward
                                     String::from_utf8_lossy(&i[0..length])
                                             .into_owned());
            }
        }
        IResult::Incomplete(a) => IResult::Incomplete(a),
        IResult::Error(a) => IResult::Error(a),
    }
}

named!(record_complete <&[u8], Record >,
    chain!(
        entry: record              ~
        tag!("\r")?                 ~
        tag!("\n")                  ~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        move ||{entry}
    )
);


//
// Taken from Nom, it was FileProducer
//

#[derive(Debug,Copy,Clone,PartialEq,Eq)]
pub enum StdinProducerState {
    Normal,
    Error,
    Eof,
}

#[derive(Debug)]
pub struct StdinProducer {
    size: usize,
    v: Vec<u8>,
    start: usize,
    end: usize,
    state: StdinProducerState,
}

impl StdinProducer {
    pub fn new(buffer_size: usize) -> io::Result<StdinProducer> {
        Ok(StdinProducer {
            size: buffer_size,
            v: vec![0; buffer_size],
            start: 0,
            end: 0,
            state: StdinProducerState::Normal,
        })
    }

    pub fn shift(&mut self) {
        let length = self.end - self.start;
        unsafe {
            ptr::copy((&self.v[self.start..self.end]).as_ptr(),
                      (&mut self.v[..length]).as_mut_ptr(),
                      length);
        }
        self.start = 0;
        self.end = length;
    }
    
    /// Read into the buffer but handle errors using state rather than Result
    pub fn protected_read(&mut self, file: &mut StdinLock) -> usize {
        match file.read(&mut self.v[self.end..]) {
            Err(_) => {
                self.state = StdinProducerState::Error;
                0
            }
            Ok(n) => {
                self.end += n;
                if n == 0 {
                    self.state = StdinProducerState::Eof;
                }
                n
            }
        }
    }

    /// Try to advance the end of the buffer forward by n bytes.
    ///
    /// Returns how many bytes it could actually get.
    pub fn await(&mut self) -> usize {
        self.shift();
        let stdin = io::stdin();
        let mut file = stdin.lock();
        let space = self.size - self.end;
        let mut received = 0;
        let mut this_chunk = 1;
        while received < space && this_chunk != 0 {
            this_chunk = self.protected_read(&mut file);
            received += this_chunk;
        }
        received
    }
    
    /// Read N bytes but don't keep them
    pub fn skip_through(&mut self, amount: usize) -> usize {
        let stdin = io::stdin();
        let mut file = stdin.lock();
        let mut skipped = 0;
        self.start = 0;
        while skipped < amount {
            self.end = 0;
            let returned = self.protected_read(&mut file);
            if returned == 0 {
                // Eof
                self.start = 0;
                self.end = 0;
                break;
            } else if returned >= amount {
                self.start = returned-amount;
                self.end = returned;
                break;
            } else {
                skipped += returned;
            }
        }
        skipped
    }
    
    /// Advance the start of the buffer by N bytes
    pub fn consume(&mut self, amount: usize) -> usize {
        let length = self.end - self.start;
        if amount < length {
            // move a pointer
            self.start += amount;
            amount
        } else if amount == length {
            self.start = 0;
            self.end = 0;
            self.await();
            amount
        } else {
            // refill the buffer
            self.skip_through(amount-length)
        }
    }
}



impl<'x> Producer<'x, &'x [u8], Move> for StdinProducer {
    fn apply<'a, O, E>(&'x mut self,
                       consumer: &'a mut Consumer<&'x [u8], O, E, Move>)
                       -> &'a ConsumerState<O, E, Move> {
        if {
            if let &ConsumerState::Continue(ref m) = consumer.state() {
                match *m {
                    Move::Consume(s) => {
                        self.consume(s);
                    }
                    Move::Await(Needed::Size(n)) => {
                        if self.await() == 0 {
                            warn!("In a bind! Trashing a record that wants {} bytes", self.size+n);
                            // The buffer is full but the record is incomplete
                            // So to avoid a lock we have to corrupt the next record
                            // and move until the next non-failure.
                            {let s=self.size; self.consume(s + n);}
                        }
                    },
                    Move::Await(Needed::Unknown) => {
                        // See above
                        if self.await() == 0 {
                            warn!("In a bind! Trashing a record that wants more than {} bytes.", self.size);
                            {let s=self.size; self.consume(s);}
                        }
                    },
                    Move::Seek(_position) => {
                        self.state = StdinProducerState::Error;
                    }
                }
                true
            } else {
                false
            }
        } {
            match self.state {
                StdinProducerState::Normal => {
                    consumer.handle(Input::Element(&self.v[self.start..self.end]))
                }
                StdinProducerState::Eof => {
                    let slice = &self.v[self.start..self.end];
                    if slice.is_empty() {
                        consumer.handle(Input::Eof(None))
                    } else {
                        consumer.handle(Input::Eof(Some(slice)))
                    }
                }
                // is it right?
                StdinProducerState::Error => consumer.state(),
            }
        } else {
            consumer.state()
        }
    }
}
