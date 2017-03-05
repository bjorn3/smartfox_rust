extern crate regex;
extern crate serde_json;

use std::fmt;

/// A server returned packet of data.
#[derive(Clone, PartialEq)]
pub struct Packet {
    pub name: String,
    pub room: String,
    pub data: String,
}

impl ::std::str::FromStr for Packet {
    type Err = ();
    fn from_str(data: &str) -> Result<Self, ()> {
        use self::regex::Regex;

        lazy_static! {
            static ref PACKET_REGEX: Regex =
                Regex::new(r"^%xt%([[:word:]]+)%1%(0%)?([^0].*)$").expect("Invalid packet regex");
        }

        if data.is_empty() {
            return Ok(Packet {
                name: "".to_string(),
                room: "".to_string(),
                data: "".to_string(),
            });
        }

        if let Some(captures) = PACKET_REGEX.captures(&data) {
            let name = captures.get(1).unwrap().as_str();
            let data = captures.get(3).unwrap().as_str().trim_right_matches('%');
            assert!(captures.get(4).is_none());
            Ok(Packet {
                name: name.to_string(),
                room: "".to_string(),
                data: data.to_string(),
            })
        } else {
            Ok(Packet {
                name: "".to_string(),
                room: "".to_string(),
                data: data.to_string(),
            })
        }
    }
}

impl ::std::string::ToString for Packet {
    fn to_string(&self) -> String {
        if self.name.is_empty() {
            self.data.clone()
        } else if self.room.is_empty() {
            format!("%xt%{}%1%0{}%", self.name, self.data)
        } else {
            format!("%xt%{}%{}%1%0{}%", self.room, self.name, self.data)
        }
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{:9} @ {:12} ( {} ... )",
               self.name,
               self.room,
               self.data.chars().take(64).collect::<String>())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_server_packet() {
        assert_eq!("%xt%dwd%1%0%".parse::<Packet>().unwrap(),
                   Packet {
                       name: "dwd".to_string(),
                       room: "".to_string(),
                       data: "".to_string(),
                   });
        assert_eq!(r#"%xt%gbd%1%0%{"gpi":{"UID":0}}%"#.parse::<Packet>().unwrap(),
                   Packet {
                       name: "gbd".to_string(),
                       room: "".to_string(),
                       data: r#"{"gpi":{"UID":0}}"#.to_string(),
                   });
    }

    #[test]
    fn parse_long_server_packet() {
        assert_eq!("efroniveioej54549945wj9awjoawoiwa2322131298489439834#@*($&*($(*(*$@))))"
                       .parse::<Packet>()
                       .unwrap(),
                   Packet {
                       name: "".to_string(),
                       room: "".to_string(),
                       data:
                           "efroniveioej54549945wj9awjoawoiwa2322131298489439834#@*($&*($(*(*$@))))"
                           .to_string(),
                   })
    }

    #[test]
    fn display_server_packet() {
        assert_eq!(format!("{:?}",
                           Packet {
                               name: "erguiriu".to_string(),
                               room: "".to_string(),
                               data: "dsimoreoib".to_string(),
                           }),
                   "erguiriu  @              ( dsimoreoib ... )".to_string());
    }

    #[test]
    fn serialize_client_packet() {
        assert_eq!(Packet {
                           name: "friofr".to_string(),
                           room: "Hello room".to_string(),
                           data: "{43gjo}".to_string(),
                       }
                       .to_string(),
                   "%xt%Hello room%friofr%1%0{43gjo}%".to_string());
    }
}
