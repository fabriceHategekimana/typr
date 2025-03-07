use std::fmt;
use serde::Serialize;
use crate::language::Lang;
use crate::module::Module;
use crate::NominalContext;
use crate::Context;

#[derive(Debug, Serialize, Clone)]
pub struct Adt(pub Vec<Lang>);

impl fmt::Display for Adt {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cont = Context::new(vec![], vec![]);
        let res = self.0.iter().map(|x| x.disp(&NominalContext::new(), &cont))
            .reduce(|acc, x| format!("{}, {}", acc, x))
            .unwrap_or("".to_string());
        write!(f, "sequence([{}])", res)       
    }
}

impl From<Vec<Lang>> for Adt {
   fn from(val: Vec<Lang>) -> Self {
        Adt(val)
   } 
}

fn find_alias(name: &str, v: Vec<Lang>) -> Lang {
    v.iter().find(|x| 
        match x {
          Lang::Alias(var, _params, _typ) 
                if var.get_name() == name => true,
            _ => false
        }).unwrap().clone()
}

impl Adt {
    pub fn iter(&self) -> std::slice::Iter<Lang> {
        self.0.iter()
    }

    pub fn find_alias_module(&self, path: &str, name: &str) -> (Lang, Module) {
        let module = self.find_module(path);
        let alias = find_alias(name, module.get_body());
        (alias, module)
    }

    fn find_module(&self, path: &str) -> Module {
        if let Lang::Module(name, body) = self.iter().find(|x| match x {
            Lang::Module(name, _body) 
                if name == path => true,
            _ => false
        }).unwrap_or(&Lang::Empty) {
            Module(name.to_string(), body.to_vec())
        } else {
            panic!("The module '{}' was not found", path)
        }
    }

    pub fn to_r(&self, nominal: &NominalContext, cont: &Context) -> String {
        self.iter()
            //.inspect(|x| println!("x: {:?}", x))
            .map(|line| line.to_r(nominal, cont))
            .fold(String::from(""), |acc, x| format!("{}\n{}", acc, x))
    }
}
