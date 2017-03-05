#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

use std::io;
use std::str;

use futures::{future, Future, BoxFuture};
use tokio_core::io::{Codec, EasyBuf, Io, Framed};
use tokio_service::Service;

pub mod packet;

pub struct SmartFoxCodec;

impl Codec for SmartFoxCodec {
    type In = String;
    type Out = String;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        if let Some(i) = buf.as_slice().iter().position(|&b| b == b'\0') {
            // remove the serialized frame from the buffer.
            let line = buf.drain_to(i);

            // Also remove the '\0'
            buf.drain_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            match str::from_utf8(line.as_slice()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }

    fn encode(&mut self, msg: String, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.extend(msg.as_bytes());
        buf.push(b'\0');
        Ok(())
    }
}

pub trait Delegate {
    fn login(&mut self, room: &str, un: &str, pw: &str) -> Vec<packet::Packet>;
    fn request(&mut self, packet: packet::Packet) -> Vec<packet::Packet>;
}

pub struct SmartFoxService<D: Delegate> {
    delegate: std::cell::RefCell<D>,
    state: std::cell::Cell<SmartFoxServiceState>,
}

impl<D: Delegate> SmartFoxService<D> {
    pub fn new(delegate: D) -> Self {
        SmartFoxService {
            delegate: std::cell::RefCell::new(delegate),
            state: std::cell::Cell::new(SmartFoxServiceState::Handshake),
        }
    }
}

#[derive(Copy, Clone)]
enum SmartFoxServiceState {
    Handshake,
    Login,
    Running,

    Error,
}

impl<D: Delegate> Service for SmartFoxService<D> {
    type Request = String;
    type Response = String;
    type Error = std::io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;
    fn call(&self, input: Self::Request) -> Self::Future {
        println!("Input: {}", input);
        match self.state.get() {
            SmartFoxServiceState::Handshake => {
                if &*input ==
                   "<msg t='sys'><body action='verChk' r='0'><ver v='166' /></body></msg>" {
                    println!("Correct version");
                    self.state.set(SmartFoxServiceState::Login);
                    future::ok("<msg t='sys'><body action='apiOK' r='0'></body></msg>".to_string())
                        .boxed()
                } else {
                    // vvv Dont know official response
                    println!("Invalid version: {:?}", input);
                    self.state.set(SmartFoxServiceState::Error);
                    future::ok("<msg t='sys'><body action='apiERR' r='0'></body></msg>".to_string())
                        .boxed()
                }
            }
            SmartFoxServiceState::Login => {
                let login_regex = regex::Regex::new(
                    r#"<msg t='sys'><body action='login' r='0'><login z='([[:word:]]+)'><nick><!\[CDATA\[([[:word:]]*)\]\]></nick><pword><!\[CDATA\[([[:word:]%]*)\]\]></pword></login></body></msg>"#
                ).unwrap();
                if let Some(captures) = login_regex.captures(&input) {
                    let room = captures.get(1).unwrap().as_str();
                    let name = captures.get(2).unwrap().as_str();
                    let pass = captures.get(3).unwrap().as_str();
                    println!("Room: {}, name: {}, pass: {}", room, name, pass);
                    self.state.set(SmartFoxServiceState::Running);
                    let res = self.delegate.borrow_mut().login(&room, &name, &pass).into_iter().map(|pkt|pkt.to_string()).collect::<Vec<_>>().join("\0");
                    future::ok(res).boxed()
                } else {
                    println!("Invalid login packet");
                    self.state.set(SmartFoxServiceState::Error);
                    future::err(std::io::ErrorKind::InvalidData.into()).boxed()
                }
            }
            SmartFoxServiceState::Running => {
                let res = self.delegate.borrow_mut().request(input.parse().unwrap()).into_iter().map(|pkt|pkt.to_string()).collect::<Vec<_>>().join("\0");
                future::ok(res).boxed()
            },
            SmartFoxServiceState::Error => {
                future::err(std::io::ErrorKind::InvalidData.into()).boxed()
            }
        }
    }
}
