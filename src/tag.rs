use crate::Type;
use serde::Serialize;
use crate::help_data::HelpData;

type Name = String;

#[derive(Debug, Clone, PartialEq, Serialize, Eq, Hash)]
pub struct Tag(pub Name, pub Type, pub HelpData);

impl Tag {
    pub fn new(name: String, typ: Type, h: HelpData) -> Tag {
        Tag(name, typ, h)
    }

    pub fn from_type(typ: Type) -> Option<Tag> {
        match typ {
            Type::Tag(name, typ2, h) => 
                Some(Tag(name.to_string(), (*typ2).clone(), h)),
            _ => None
        }
    }

    //pub fn from_language(lang: Lang, _context: &Context) -> Option<Tag> {
        //match lang {
            //Lang::Tag(name, _typ, h) => Some(Tag(name, Type::Any, h)),
            //_ => None
        //}
    //}

    pub fn to_type(&self) -> Type {
        Type::Tag(self.0.clone(), Box::new(self.1.clone()), self.2.clone())
    }

    pub fn get_name(&self) -> String {
        self.0.clone()
    }

    pub fn get_type(&self) -> Type {
        self.1.clone()
    }

}

use std::fmt;
impl fmt::Display for Tag {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":{}({})", self.0, self.1)       
    }
}
