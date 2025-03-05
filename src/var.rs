use crate::Type;
use crate::Lang;
use std::fmt;
use serde::Serialize;
use crate::context::Context;
use crate::type_comparison;

type Name = String;
type Path = String;
type IsMutableOpaque = bool;

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum Permission {
    Private,
    Public
}

impl fmt::Display for Permission {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::Private => write!(f, "private"),
            Permission::Public => write!(f, "public")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Var(pub Name, pub Path, pub Permission, pub IsMutableOpaque, pub Type);

impl Var {
    pub fn from_language(l: Lang) -> Option<Var> {
        match l {
            Lang::Variable(name, path, perm, muta, typ) 
                => Some(Var(name, path, perm, muta, typ)),
            _ => None
        }
    }

    pub fn from_name(name: &str) -> Self {
        Var(
            name.to_string(),
            "".to_string(),
            Permission::Private,
            false,
            Type::Empty)
    }

    pub fn to_language(self) -> Lang {
        Lang::Variable(self.0, self.1, self.2, self.3, self.4)
    }

    pub fn set_type(self, typ: Type) -> Var {
        Var(self.0, self.1, self.2, self.3, typ)
    }

    pub fn set_permission(self, perm: bool) -> Var {
        let new_perm = if perm == true { Permission::Public } else { Permission::Private };
        Var(self.0, self.1, new_perm, self.3, self.4)
    }

    pub fn set_mutability(self, muta: bool) -> Var {
        Var(self.0, self.1, self.2, muta, self.4)
    }

    pub fn set_opacity(self, opa: bool) -> Var {
        Var(self.0, self.1, self.2, opa, self.4)
    }

    pub fn add_path(self, name: &str) -> Var {
        if self.1 == "" {
            Var(self.0, name.to_string(), self.2, self.3, self.4)
        } else {
            Var(self.0, self.1 + "/" + name, self.2, self.3, self.4)
        }
    }

    pub fn get_name(&self) -> String {
        self.0.to_string()
    }

    pub fn get_path(&self) -> String {
        self.1.to_string()
    }

    pub fn get_permission(&self) -> Permission {
        self.2
    }

    pub fn set_path(self, new_path: &str) -> Var {
        Var(self.0, new_path.to_string(), self.2, self.3, self.4)
    }

    pub fn get_type(&self) -> Type {
        self.4.clone()
    }

    pub fn get_is_mutable(&self) -> bool {
        self.3.clone()
    }

    pub fn get_is_opaque(&self) -> bool {
        self.3.clone()
    }

    pub fn is_alias(&self) -> bool {
        match self {
            Var(_, _, _, _, Type::Params(_)) => true,
            _ => false
        }
    }

    pub fn match_with(&self, var: &Var, context: &Context) -> bool {
        [(self.get_name() == var.get_name()),
        (self.get_path() == var.get_path()),
        (self.get_permission() == Permission::Public),
        type_comparison::is_matching(context, &self.get_type(), &var.get_type())].iter().all(|&x| x)
    }
}

impl fmt::Display for Var {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.1 == "" { 
            write!(f, "var('{}', empty, {}, {}, {})",
            self.0, self.2, self.3, self.4)       
        } else {
            write!(f, "var('{}', '{}', {}, {}, {})", 
                   self.0, self.1, self.2, self.3, self.4)
        }
    }
}

impl Default for Var {
    fn default() -> Self {
        Var("".to_string(), "".to_string(), Permission::Private, false, Type::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_perm1(){
        assert_eq!(
            Var("hey".to_string(), "".to_string(), Permission::Public, false, Type::Empty).set_permission(false),
            Var("hey".to_string(), "".to_string(), Permission::Private, false, Type::Empty)
            ); 
    }
}
