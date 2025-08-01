use std::collections::HashMap;
use crate::r#type::Type;
use crate::Context;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum TypeCategory {
    Array,
    Function,
    Record,
    Tag,
    Union,
    Interface,
    Boolean,
    Integer,
    Number,
    Char,
    Generic,
    DataFrame,
    Rest
}

impl TypeCategory {
    fn from_type(t: Type) -> TypeCategory {

        match t {
            Type::Array(_, _, _) => TypeCategory::Array,
            Type::Function(_, _, _, _) => TypeCategory::Function,
            Type::Record(_, _) => TypeCategory::Record,
            Type::Tag(_, _, _) => TypeCategory::Tag,
            Type::Union(_, _) => TypeCategory::Union,
            Type::Interface(_, _) => TypeCategory::Interface,
            Type::Boolean(_) => TypeCategory::Boolean,
            Type::Number(_) => TypeCategory::Number,
            Type::Char(_, _) => TypeCategory::Char,
            Type::Generic(_, _) => TypeCategory::Generic,
            Type::IndexGen(_, _) => TypeCategory::Generic,
            Type::LabelGen(_, _) => TypeCategory::Generic,
            Type::DataFrame(_) => TypeCategory::DataFrame,
            _ => TypeCategory::Rest
        }
    }
}

use std::fmt;
impl fmt::Display for TypeCategory {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            TypeCategory::Array => "Array",
            TypeCategory::Function => "Function",
            TypeCategory::Record => "Record",
            TypeCategory::Tag => "Tag",
            TypeCategory::Union => "Union",
            TypeCategory::Interface => "Interface",
            TypeCategory::Boolean => "logical",
            TypeCategory::Integer => "integer",
            TypeCategory::Number => "numeric",
            TypeCategory::DataFrame => "data.frame",
            TypeCategory::Char => "character",
            TypeCategory::Generic => "Generic",
            TypeCategory::Rest => "Rest"
        };
        write!(f, "{}", res)       
    }
}

#[derive(Debug, Clone)]
struct Categories(HashMap<TypeCategory, usize>);

#[derive(Debug, Clone)]
pub struct AliasNominal(HashMap<Type, Nominal>);

impl AliasNominal {
    pub fn new() -> Self {
        AliasNominal(HashMap::new())
    }

    pub fn get_nominal(&self, typ: &Type) -> Option<String> {
        self.0.get(typ).map(|nom| nom.0.clone())
    }

    pub fn push_alias(&mut self, alias_name: &str, typ: &Type) {
       let nominal = Nominal(alias_name.to_string());
       self.0.insert(typ.to_owned(), nominal);
    }

    pub fn contains(&self, typ: &Type) -> bool {
       self.0.keys()
           .find(|typ_| typ == *typ_ || (typ.is_generic() && typ_.is_generic()))
           .is_some()
    }

    pub fn push_type(&mut self, typ: Type, cat_nom: Nominal) {
        self.0.entry(typ).or_insert(cat_nom);
    }

    pub fn display_nominals(&self) -> String {
        "-----------------\n".to_string() +
        &self.0.iter().map(|(typ, nom)| {
            typ.pretty() + " ===> " + &nom.0
        }).collect::<Vec<_>>().join("\n") +
        "\n-----------------\n"
    }

}

#[derive(Debug, Clone)]
pub struct TypeNominal {
    pub body: AliasNominal, 
    categories: Categories
}


impl TypeNominal {
    pub fn new() -> Self {
        let mut categories = HashMap::new();
        categories.insert(TypeCategory::Array, 0 as usize);
        categories.insert(TypeCategory::Function, 0 as usize);
        categories.insert(TypeCategory::Record, 0 as usize);
        categories.insert(TypeCategory::Tag, 0 as usize);
        categories.insert(TypeCategory::Union, 0 as usize);
        categories.insert(TypeCategory::Interface, 0 as usize);
        categories.insert(TypeCategory::Boolean, 0 as usize);
        categories.insert(TypeCategory::Integer, 0 as usize);
        categories.insert(TypeCategory::Number, 0 as usize);
        categories.insert(TypeCategory::Char, 0 as usize);
        categories.insert(TypeCategory::Generic, 0 as usize);
        categories.insert(TypeCategory::Rest, 0 as usize);
        TypeNominal {
            body: AliasNominal::new(),
            categories: Categories(categories)
        }
    }

    fn get_nth(&self, type_: Type) -> (Categories, usize) {
      match type_ {
          Type::Array(_, _, _) => self.get_nth_helper(TypeCategory::Array),
          Type::Function(_, _, _, _) => self.get_nth_helper(TypeCategory::Function),
          Type::Record(_, _) => self.get_nth_helper(TypeCategory::Record),
          Type::Number(_) | Type::Integer(_, _) | Type::Char(_, _) | Type::Boolean(_) => (self.categories.clone(), 100 as usize),
          Type::Embedded(_, _) | Type::Generic(_, _) | Type::IndexGen(_, _) => (self.categories.clone(), 0 as usize),
          Type::Tag(_, _, _) => self.get_nth_helper(TypeCategory::Tag),
          Type::Union(_, _) => self.get_nth_helper(TypeCategory::Union),
          Type::Interface(_, _) => self.get_nth_helper(TypeCategory::Interface),
          Type::Failed(_, _) | Type::Params(_, _) | Type::Empty(_) => (self.categories.clone(), 0 as usize),
          Type::Add(_, _, _) | Type::Mul(_, _, _) | Type::Div(_, _, _) | Type::Minus(_, _, _) => (self.categories.clone(), 0 as usize),
          _ => self.get_nth_helper(TypeCategory::Rest)
      }
   } 

   fn get_nth_helper(&self, category: TypeCategory) -> (Categories, usize) {
       (self.incr(category), self.get(category))
   }

   fn incr(&self, category: TypeCategory) -> Categories {
       let mut mut_cate = self.categories.clone();
       mut_cate.0.insert(category, *self.categories.0.get(&category).unwrap() + 1);
       mut_cate.clone()
   }

   fn get(&self, category: TypeCategory) -> usize {
       *self.categories.0.get(&category).unwrap()
   }

   fn new_nominal(&self, type_: Type) -> (Categories, Nominal) {
       let res = self.get_nth(type_.clone());
        (res.0, Nominal::from((type_, res.1)))
   }

   fn contains(&self, typ: &Type) -> bool {
       self.body.contains(typ)
   }

   pub fn push_type(&self, typ: Type) -> TypeNominal {
       if self.contains(&typ) || typ.clone().to_category() == TypeCategory::Rest {
            self.to_owned()
       } else {
           let cat_nom = self.new_nominal(typ.clone());
           let mut alias_nominal = self.body.to_owned();
           alias_nominal.push_type(typ, cat_nom.1);
           TypeNominal {
               body: alias_nominal,
               categories: cat_nom.0
           }
       }
   }

   pub fn push_alias(&self, alias_name: String, typ: Type) -> TypeNominal {
       let mut alias_nominal = self.body.to_owned();
       alias_nominal.push_alias(&alias_name, &typ);
       TypeNominal {
           body: alias_nominal,
           ..self.to_owned()
       }
   }

   pub fn get_class(&self, typ: &Type, _cont: &Context) -> String {
       match typ {
           Type::Empty(_) => "Empty".to_string(),
           Type::Any(_) => "Empty".to_string(),
           Type::Integer(_, _) => "integer".to_string(),
           Type::Number(_) => "numeric".to_string(),
           Type::Char(_, _) => "character".to_string(),
           _ => {
               self.body.get_nominal(typ).unwrap_or("Empty".to_string())
           }
       }
   }

   pub fn get_type_from_class(&self, class: &str) -> Type {
       self.body.0.iter()
           .find(|(_type, nominal)| class == nominal.0)
           .unwrap().0.clone()
   }

   pub fn get_nominal(&self, typ: Type) -> (TypeNominal, String) {
       match typ {
           Type::Integer(_, _) => (self.to_owned(), "integer".to_string()),
           Type::Char(_, _) => (self.to_owned(), "character".to_string()),
           Type::Boolean(_) => (self.to_owned(), "logical".to_string()),
           Type::Number(_) => (self.to_owned(), "numeric".to_string()),
           Type::DataFrame(_) => (self.to_owned(), "data.frame".to_string()),
           Type::Alias(name, _, _, _, _) => (self.to_owned(), name),
           _ => match self.body.get_nominal(&typ) {
               Some(nom) => (self.to_owned(), nom),
               None => self.generate_nominal(typ)
           }
       }
   }

   pub fn generate_nominal(&self, typ: Type) -> (TypeNominal, String) {
       let new_type_nominal = self.push_type(typ.clone());
       let type_category = typ.to_category();
       let categories = new_type_nominal.categories.0.clone();
       let index = categories.get(&type_category)
           .expect(&format!("The type category {} wasn't found", type_category));
       (new_type_nominal, format!("{}{}", type_category, index))
   }

   pub fn display_nominals(&self) -> String {
       self.body.display_nominals()
   }

}

#[derive(Debug, PartialEq, Clone)]
pub struct Nominal(pub String);

impl From<(Type, usize)> for Nominal {
   fn from(val: (Type, usize)) -> Self {
       let category = TypeCategory::from_type(val.0);
       if val.1 != 100 { // if it's not a primitive like int or bool
           match category {
               TypeCategory::Rest => Nominal("".to_string()),
               _ => Nominal(format!("{}_{}", category, val.1))
           }
       } else {
           Nominal(format!("{}", category))
       }
   } 
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder;
    use crate::tint::Tint;
    use crate::help_data::HelpData;

    #[test]
    fn test_get_nominal1(){
        let tn = TypeNominal::new();
        //let typ = builder::integer_type_default();
        let typ = Type::Integer(Tint::Unknown, HelpData::example());
        let tn = tn.push_type(typ.clone());
        assert_eq!(tn.get_nominal(typ).1, "integer");
    }

    #[test]
    fn test_get_nominal2(){
        let tn = TypeNominal::new();
        let typ = builder::integer_type(3);
        let rec = builder::record_type(&[("a".to_string(), typ.clone())]);
        let tn = tn.push_type(rec.clone());
        assert_eq!(tn.get_nominal(rec).1, "Record_0");
    }

    #[test]
    fn test_get_push_alias(){
        let tn = TypeNominal::new();
        let typ = builder::integer_type(3);
        let rec = builder::record_type(&[("a".to_string(), typ.clone())]);
        let tn = tn.push_alias("Counter".to_string(), rec.clone());
        let tn = tn.push_type(rec.clone());
        assert_eq!(tn.get_nominal(rec).1, "Counter");
    }

}
